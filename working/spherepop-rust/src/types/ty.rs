use crate::core::event::{CollapseRule, RefusalReason, Symbol};

/// Types in Spherepop are admissibility certificates.
/// A type is not a set of values; it is a proof that a transformation
/// preserves the reachability structure it claims to preserve.
///
/// v0.2: Refused now carries the RefusalReason.
/// v0.3: Collapsed now carries the CollapseRule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Unit,
    Never,
    Var(Symbol),
    Universe(u32),

    /// Π (param : param_ty) . body_ty — subsumes simple A → B.
    Pi {
        param: Symbol,
        param_ty: Box<Type>,
        body_ty: Box<Type>,
    },

    /// Admissible(T): T was reached through admissible transitions.
    Admissible(Box<Type>),

    /// Refused(T, reason): T was encountered but found inadmissible.
    /// v0.2: reason is now part of the type, not erased.
    Refused {
        inner: Box<Type>,
        reason: RefusalReason,
    },

    /// Collapsed(T, rule): T was sealed into observable state via rule.
    /// v0.3: the collapse rule is part of the type.
    Collapsed {
        inner: Box<Type>,
        rule: CollapseRule,
    },

    /// Process(from ~> to): a dependency-preserving transformation.
    Process { from: Box<Type>, to: Box<Type> },

    /// History(T): the type of an append-only history of T-events.
    History(Box<Type>),
}

impl Type {
    pub fn arrow(from: Type, to: Type) -> Type {
        Type::Pi {
            param: Symbol::new("_"),
            param_ty: Box::new(from),
            body_ty: Box::new(to),
        }
    }

    pub fn admissible(self) -> Type { Type::Admissible(Box::new(self)) }

    pub fn refused(self, reason: RefusalReason) -> Type {
        Type::Refused { inner: Box::new(self), reason }
    }

    pub fn collapsed(self, rule: CollapseRule) -> Type {
        Type::Collapsed { inner: Box::new(self), rule }
    }

    pub fn is_admissible_type(&self) -> bool {
        matches!(self, Type::Admissible(_))
    }

    pub fn is_universe(&self) -> bool { matches!(self, Type::Universe(_)) }

    pub fn universe_level(&self) -> Option<u32> {
        match self { Type::Universe(n) => Some(*n), _ => None }
    }

    pub fn strip_admissible(&self) -> &Type {
        match self { Type::Admissible(inner) => inner, other => other }
    }

    /// Extract the inner type and collapse rule if this is a Collapsed type.
    pub fn collapse_info(&self) -> Option<(&Type, &CollapseRule)> {
        match self {
            Type::Collapsed { inner, rule } => Some((inner, rule)),
            _ => None,
        }
    }

    /// Extract the inner type and refusal reason if this is a Refused type.
    pub fn refusal_info(&self) -> Option<(&Type, &RefusalReason)> {
        match self {
            Type::Refused { inner, reason } => Some((inner, reason)),
            _ => None,
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Unit => write!(f, "Unit"),
            Type::Never => write!(f, "Never"),
            Type::Var(s) => write!(f, "{}", s),
            Type::Universe(0) => write!(f, "Type"),
            Type::Universe(n) => write!(f, "Type{}", n),
            Type::Pi { param, param_ty, body_ty } => {
                if param.0 == "_" {
                    write!(f, "({} → {})", param_ty, body_ty)
                } else {
                    write!(f, "(Π {} : {} . {})", param, param_ty, body_ty)
                }
            }
            Type::Admissible(t) => write!(f, "Admissible({})", t),
            Type::Refused { inner, reason } => write!(f, "Refused({} ∵ {})", inner, reason),
            Type::Collapsed { inner, rule } => write!(f, "Collapsed({} via {})", inner, rule),
            Type::Process { from, to } => write!(f, "Process({} ~> {})", from, to),
            Type::History(t) => write!(f, "History({})", t),
        }
    }
}
