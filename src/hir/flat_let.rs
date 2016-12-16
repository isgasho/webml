use prim::*;
use hir::*;
use pass::Pass;

pub struct FlatLet {
    id: usize,
}

fn take_binds(mut expr: Expr) -> (Expr, Vec<Val>) {
    use hir::Expr::*;
    match expr {
        Binds{binds, ret, ..} => {
            expr = *ret;
            (expr, binds)
        }
        App{mut fun, mut arg, ty} => {
            let (f, mut fbinds) = take_binds(*fun);
            let (a, mut abinds) = take_binds(*arg);
            fun = Box::new(f);
            arg = Box::new(a);
            fbinds.append(&mut abinds);
            let expr = App {fun: fun, arg: arg, ty: ty};
            (expr, fbinds)
        },
        If {mut cond, then, else_, ty} => {
            let (c, cbinds) = take_binds(*cond);
            cond = Box::new(c);
            let expr = If {cond: cond, then: then, else_: else_, ty: ty};
            (expr, cbinds)
        },
        x @ Fun{..} |
        x @ PrimFun{..} |
        x @ Sym{..} |
        x @ Lit{..} => (x, Vec::new())

    }
}

impl FlatLet {
    pub fn new() -> Self {
        FlatLet {
            id: 0
        }
    }

    pub fn gensym(&mut self) -> Symbol {
        let name = format!("#g{:06}", self.id);
        self.id += 1;
        Symbol(name)
    }

   pub fn flat_hir(&mut self, mut hir: HIR) -> HIR {
       hir.0 = hir.0.into_iter().map(|val| self.flat_val(val)).collect();
       hir
    }

    fn flat_val(&mut self, mut val: Val) -> Val {
        val.expr = self.flat_expr(val.expr);
        val
    }

    fn flat_expr(&mut self, expr: Expr) -> Expr {
        use hir::Expr::*;
        match expr {
            Binds{mut binds, mut ret, ty} => {
                let mut vec = Vec::new();
                for mut val in binds.into_iter() {
                    val.expr = self.flat_expr(val.expr);
                    let (expr, mut binds) = take_binds(val.expr);
                    val.expr = expr;
                    vec.append(&mut binds);
                    vec.push(val)
                }
                let (expr, mut binds_) = take_binds(*ret);
                ret = Box::new(expr);
                vec.append(&mut binds_);
                binds = vec;
                Binds {binds: binds, ret: ret, ty: ty}
            },
            Fun{mut body, ty, param} => {
                body = Box::new(self.flat_expr(*body));
                Fun{body: body, ty: ty, param: param}
            }
            App{mut fun, mut arg, ty} => {
                fun = Box::new(self.flat_expr(*fun));
                arg = Box::new(self.flat_expr(*arg));
                App{fun: fun, arg: arg, ty: ty}
            }
            If {mut cond, mut then, mut else_, ty} => {
                cond = Box::new(self.flat_expr(*cond));
                then = Box::new(self.flat_expr(*then));
                else_ = Box::new(self.flat_expr(*else_));
                If {ty: ty, cond: cond, then: then, else_: else_}
            }
            x @ PrimFun{..} |
            x @ Sym{..} |
            x @ Lit{..} => x

        }
    }
}


impl Pass<HIR> for FlatLet {
    type Target = HIR;
    type Err = TypeError;

    fn trans(&mut self, hir: HIR) -> ::std::result::Result<Self::Target, Self::Err> {
        Ok(self.flat_hir(hir))
    }
}