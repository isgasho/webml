use crate::ast::util::Traverse;
use crate::ast::*;
use crate::config::Config;
use crate::id::Id;
use crate::pass::Pass;
use crate::prim::*;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut, Drop};

pub struct Rename {
    tables: Vec<HashMap<Symbol, u64>>,
    pos: usize,
    id: Id,
}

struct Scope<'a>(&'a mut Rename);

impl<'a> Deref for Scope<'a> {
    type Target = Rename;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> DerefMut for Scope<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> Drop for Scope<'a> {
    fn drop(&mut self) {
        self.pos -= 1;
    }
}

impl<'a> Scope<'a> {
    fn new(inner: &'a mut Rename) -> Self {
        let pos = inner.pos;
        if inner.tables.len() <= pos {
            inner.tables.push(HashMap::new())
        } else {
            inner.tables[pos].clear();
        }

        inner.pos += 1;
        Scope(inner)
    }

    fn new_scope(&mut self) -> Scope {
        Scope::new(self)
    }

    fn new_symbol(&mut self, symbol: &mut Symbol) {
        let pos = self.pos - 1;
        let new_id = self.id.next();
        self.tables[pos].insert(symbol.clone(), new_id);
        symbol.1 = new_id;
    }

    fn new_symbol_pattern(&mut self, pat: &mut Pattern) {
        use Pattern::*;
        match pat {
            Wildcard { .. } | Lit { .. } => (),
            Var { name, .. } => self.new_symbol(name),
            Tuple { tuple } => {
                for (_, sym) in tuple {
                    self.new_symbol(sym)
                }
            }
        }
    }

    fn rename(&mut self, symbol: &mut Symbol) {
        let pos = self.pos;
        for table in self.tables[0..pos].iter_mut().rev() {
            match table.get(symbol) {
                Some(new_id) => {
                    symbol.1 = *new_id;
                    return;
                }
                None => {}
            }
        }
    }
}

impl<'a> util::Traverse for Scope<'a> {
    fn traverse_ast<'b, 'c>(&'b mut self, hir: &'c mut AST) {
        let scope = self;
        for val in hir.0.iter_mut() {
            if val.rec {
                scope.new_symbol_pattern(&mut val.pattern);
                scope.traverse_val(val);
            } else {
                scope.traverse_val(val);
                scope.new_symbol_pattern(&mut val.pattern);
            }
        }
    }

    fn traverse_val<'b, 'c>(&'b mut self, val: &'c mut Val) {
        self.traverse_expr(&mut val.expr);
    }

    fn traverse_binds(&mut self, _ty: &mut TyDefer, binds: &mut Vec<Val>, ret: &mut Box<Expr>) {
        let mut scope = self.new_scope();
        for bind in binds.iter_mut() {
            scope.traverse_val(bind);
            scope.new_symbol_pattern(&mut bind.pattern);
        }
        scope.traverse_expr(ret);
    }

    fn traverse_binop(
        &mut self,
        op: &mut Symbol,
        _ty: &mut TyDefer,
        l: &mut Box<Expr>,
        r: &mut Box<Expr>,
    ) {
        self.rename(op);
        self.traverse_expr(l);
        self.traverse_expr(r);
    }

    fn traverse_fun(
        &mut self,
        _param_ty: &mut TyDefer,
        param: &mut Symbol,
        _body_ty: &mut TyDefer,
        body: &mut Box<Expr>,
    ) {
        let mut scope = self.new_scope();
        scope.new_symbol(param);
        scope.traverse_expr(body);
    }

    fn traverse_case(
        &mut self,
        _ty: &mut TyDefer,
        expr: &mut Box<Expr>,
        arms: &mut Vec<(Pattern, Expr)>,
    ) {
        self.traverse_expr(expr);
        for &mut (ref mut pat, ref mut arm) in arms.iter_mut() {
            let mut scope = self.new_scope();
            scope.new_symbol_pattern(pat);
            scope.traverse_expr(arm);
        }
    }

    fn traverse_sym(&mut self, _ty: &mut TyDefer, name: &mut Symbol) {
        self.rename(name);
    }
}

impl Rename {
    pub fn new(id: Id) -> Self {
        // leave built in functions as non_renamed
        let prims = crate::BUILTIN_FUNCTIONS
            .iter()
            .map(|s| (Symbol::new(*s), 0))
            .collect();

        Rename {
            tables: vec![prims],
            pos: 0,
            id,
        }
    }

    fn scope<'a>(&'a mut self) -> Scope<'a> {
        Scope::new(self)
    }
}

impl<E> Pass<AST, E> for Rename {
    type Target = AST;

    fn trans(&mut self, mut ast: AST, _: &Config) -> ::std::result::Result<Self::Target, E> {
        self.scope().traverse_ast(&mut ast);
        Ok(ast)
    }
}