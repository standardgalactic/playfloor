use crate::core::event::{CollapseRule, RefusalReason, Symbol};
use crate::syntax::ast::Term;
use crate::types::ty::Type;

#[derive(Debug, Clone)]
pub enum Value {
    Unit,
    Symbol(Symbol),
    Closure { param: Symbol, param_ty: Type, body: Box<Term>, env: Env },
    /// v0.2: Admissible carries no extra payload (presence is the certificate).
    Admissible(Box<Value>),
    /// v0.2: Refused carries the documented reason.
    Refused(Box<Value>, RefusalReason),
    /// v0.3: Collapsed carries the collapse rule used to derive it.
    Collapsed(Box<Value>, CollapseRule),
    Process { from: Box<Value>, to: Box<Value> },
    TypeValue(Type),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Unit => write!(f, "()"),
            Value::Symbol(s) => write!(f, "{}", s),
            Value::Closure { param, .. } => write!(f, "<λ{}>", param),
            Value::Admissible(v) => write!(f, "Admissible({})", v),
            Value::Refused(v, r) => write!(f, "Refused({} ∵ {})", v, r),
            Value::Collapsed(v, rule) => write!(f, "Collapsed({} via {})", v, rule),
            Value::Process { from, to } => write!(f, "Process({} ~> {})", from, to),
            Value::TypeValue(t) => write!(f, "Type({})", t),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Env {
    bindings: Vec<(Symbol, Value)>,
}

impl Env {
    pub fn new() -> Self { Self { bindings: Vec::new() } }

    pub fn extend(&self, name: Symbol, val: Value) -> Self {
        let mut e = self.clone();
        e.bindings.push((name, val));
        e
    }

    pub fn lookup(&self, name: &Symbol) -> Option<&Value> {
        self.bindings.iter().rev().find(|(n, _)| n == name).map(|(_, v)| v)
    }
}
