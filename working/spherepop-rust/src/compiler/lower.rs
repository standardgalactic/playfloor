use crate::syntax::ast::Term;
use crate::ir::event_ir::{IrEvent, IrBlock};

pub struct Compiler {
    block: IrBlock,
    scope_counter: usize,
}

impl Compiler {
    pub fn new(name: impl Into<String>) -> Self {
        Self { block: IrBlock::new(name), scope_counter: 0 }
    }

    fn fresh_scope(&mut self) -> String {
        let n = self.scope_counter;
        self.scope_counter += 1;
        format!("scope_{}", n)
    }

    pub fn lower(&mut self, term: &Term) {
        match term {
            Term::Var(x) => self.block.emit(IrEvent::Load(x.0.clone())),
            Term::Universe(n) => self.block.emit(IrEvent::PushType(*n)),
            Term::Ann(inner, _) => self.lower(inner),
            Term::Pi { .. } => self.block.emit(IrEvent::PushUnit),

            Term::Lam { param, body, .. } => {
                let scope = self.fresh_scope();
                self.block.emit(IrEvent::OpenScope(scope.clone()));
                self.block.emit(IrEvent::BeginLambda { param: param.0.clone() });
                self.lower(body);
                self.block.emit(IrEvent::EndLambda);
                self.block.emit(IrEvent::CloseScope(scope));
            }

            Term::App(fun, arg) => {
                self.lower(fun);
                self.lower(arg);
                self.block.emit(IrEvent::Apply);
            }

            Term::Let { name, value, body } => {
                self.lower(value);
                self.block.emit(IrEvent::Store(name.0.clone()));
                self.lower(body);
            }

            Term::Pop(inner) => {
                self.lower(inner);
                self.block.emit(IrEvent::Pop);
            }

            // v0.2: reason emitted into IR.
            Term::Refuse(inner, reason) => {
                self.lower(inner);
                self.block.emit(IrEvent::Refuse(reason.clone()));
            }

            // v0.3: rule emitted into IR.
            Term::Collapse(inner, rule) => {
                self.lower(inner);
                self.block.emit(IrEvent::Collapse(rule.clone()));
            }

            Term::Bind(lhs, rhs) => {
                self.lower(lhs);
                self.lower(rhs);
                self.block.emit(IrEvent::Bind);
            }

            Term::Seq(terms) => {
                self.block.emit(IrEvent::BeginSeq);
                for t in terms { self.lower(t); }
                self.block.emit(IrEvent::EndSeq);
            }
        }
    }

    pub fn finish(mut self) -> IrBlock {
        self.block.emit(IrEvent::Return);
        self.block
    }
}

pub fn compile(term: &Term) -> IrBlock {
    let mut c = Compiler::new("main");
    c.lower(term);
    c.finish()
}
