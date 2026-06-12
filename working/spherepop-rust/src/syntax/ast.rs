use crate::core::event::{CollapseRule, RefusalReason, Symbol};
use crate::types::ty::Type;

/// The Spherepop AST.
///
/// Lambda calculus lives INSIDE Spherepop — not the other way around.
///
/// v0.2: Refuse carries RefusalReason.
/// v0.3: Collapse carries CollapseRule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    Var(Symbol),

    Lam { param: Symbol, ty: Box<Type>, body: Box<Term> },
    App(Box<Term>, Box<Term>),
    Let { name: Symbol, value: Box<Term>, body: Box<Term> },

    Universe(u32),
    Pi { param: Symbol, param_ty: Box<Type>, body_ty: Box<Type> },
    Ann(Box<Term>, Box<Type>),

    /// Pop: commit to a possibility.
    Pop(Box<Term>),

    /// Refuse: mark a branch inadmissible. Now carries a documented reason.
    Refuse(Box<Term>, RefusalReason),

    /// Collapse: seal an admissible term via an explicit rule.
    Collapse(Box<Term>, CollapseRule),

    /// Bind: record a dependency.
    Bind(Box<Term>, Box<Term>),

    /// Sequence.
    Seq(Vec<Term>),
}

impl Term {
    pub fn var(s: impl Into<String>) -> Self { Term::Var(Symbol::new(s)) }

    pub fn lam(param: impl Into<String>, ty: Type, body: Term) -> Self {
        Term::Lam { param: Symbol::new(param), ty: Box::new(ty), body: Box::new(body) }
    }

    pub fn app(f: Term, x: Term) -> Self { Term::App(Box::new(f), Box::new(x)) }

    pub fn let_in(name: impl Into<String>, value: Term, body: Term) -> Self {
        Term::Let { name: Symbol::new(name), value: Box::new(value), body: Box::new(body) }
    }

    pub fn pop(inner: Term) -> Self { Term::Pop(Box::new(inner)) }

    /// Refuse with an explicit reason.
    pub fn refuse(inner: Term, reason: RefusalReason) -> Self {
        Term::Refuse(Box::new(inner), reason)
    }

    /// Refuse with the default ConstraintViolation reason (convenience).
    pub fn refuse_constraint(inner: Term, constraint: impl Into<String>) -> Self {
        Term::Refuse(Box::new(inner), RefusalReason::ConstraintViolation(Symbol::new(constraint)))
    }

    /// Refuse with an explicit string reason.
    pub fn refuse_explicit(inner: Term, msg: impl Into<String>) -> Self {
        Term::Refuse(Box::new(inner), RefusalReason::Explicit(msg.into()))
    }

    /// Collapse with an explicit rule.
    pub fn collapse(inner: Term, rule: CollapseRule) -> Self {
        Term::Collapse(Box::new(inner), rule)
    }

    /// Collapse with the Identity rule (convenience).
    pub fn collapse_id(inner: Term) -> Self {
        Term::Collapse(Box::new(inner), CollapseRule::Identity)
    }

    pub fn bind(a: Term, b: Term) -> Self { Term::Bind(Box::new(a), Box::new(b)) }
    pub fn seq(terms: Vec<Term>) -> Self { Term::Seq(terms) }
    pub fn ann(term: Term, ty: Type) -> Self { Term::Ann(Box::new(term), Box::new(ty)) }
}

impl std::fmt::Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Term::Var(s) => write!(f, "{}", s),
            Term::Lam { param, ty, body } => write!(f, "(λ{} : {} . {})", param, ty, body),
            Term::App(fun, arg) => write!(f, "({} {})", fun, arg),
            Term::Let { name, value, body } =>
                write!(f, "(let {} = {} in {})", name, value, body),
            Term::Universe(0) => write!(f, "Type"),
            Term::Universe(n) => write!(f, "Type{}", n),
            Term::Pi { param, param_ty, body_ty } =>
                write!(f, "(Π{} : {} . {})", param, param_ty, body_ty),
            Term::Ann(t, ty) => write!(f, "({} : {})", t, ty),
            Term::Pop(inner) => write!(f, "pop({})", inner),
            Term::Refuse(inner, reason) => write!(f, "refuse({} ∵ {})", inner, reason),
            Term::Collapse(inner, rule) => write!(f, "collapse({} via {})", inner, rule),
            Term::Bind(a, b) => write!(f, "bind({}, {})", a, b),
            Term::Seq(ts) => {
                write!(f, "seq[")?;
                for (i, t) in ts.iter().enumerate() {
                    if i > 0 { write!(f, "; ")?; }
                    write!(f, "{}", t)?;
                }
                write!(f, "]")
            }
        }
    }
}
