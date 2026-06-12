/// A symbol is the fundamental name-bearing unit.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Symbol(pub String);

impl Symbol {
    pub fn new(s: impl Into<String>) -> Self {
        Symbol(s.into())
    }
}

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Why a branch was refused.
///
/// v0.1 used bare Refuse(Symbol). v0.2 makes refusal *documented*:
/// refusal should carry a proof obligation, not just a name.
/// This is the mechanical counterpart of the philosophical point that
/// inadmissibility must be *reasoned about*, not merely noted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefusalReason {
    /// A named constraint was violated (the simplest case).
    ConstraintViolation(Symbol),
    /// The option was not in the current option space.
    NotAvailable,
    /// The term was already refused once; further use is blocked.
    AlreadyRefused(Symbol),
    /// A type-level admissibility check failed.
    TypeInadmissible { expected: Symbol, got: Symbol },
    /// Explicit user-provided reason (for REPL / testing).
    Explicit(String),
}

impl std::fmt::Display for RefusalReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefusalReason::ConstraintViolation(s) => write!(f, "constraint:{}", s),
            RefusalReason::NotAvailable => write!(f, "not-available"),
            RefusalReason::AlreadyRefused(s) => write!(f, "already-refused:{}", s),
            RefusalReason::TypeInadmissible { expected, got } =>
                write!(f, "type-inadmissible(expected:{}, got:{})", expected, got),
            RefusalReason::Explicit(msg) => write!(f, "explicit:{}", msg),
        }
    }
}

/// A collapse rule determines HOW observable state is derived from history.
///
/// v0.1: collapse just appended an event.
/// v0.3: collapse carries a rule so that observable state can be computed
///       as a *quotient* of history under that rule.
///
/// Think of it as: each CollapseRule defines an equivalence relation on
/// histories. Two histories that agree up to a CollapseRule are
/// observationally indistinguishable under that rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollapseRule {
    /// Identity: the observable state is the full history.
    Identity,
    /// Last-write-wins: only the most recent pop for each symbol matters.
    LastWrite,
    /// Accumulate: observable state counts how many times each symbol was popped.
    Accumulate,
    /// Projection onto a named stratum: only events tagged with this label matter.
    Projection(Symbol),
    /// User-defined rule, identified by name.
    Named(Symbol),
}

impl std::fmt::Display for CollapseRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CollapseRule::Identity => write!(f, "id"),
            CollapseRule::LastWrite => write!(f, "last-write"),
            CollapseRule::Accumulate => write!(f, "accumulate"),
            CollapseRule::Projection(s) => write!(f, "proj:{}", s),
            CollapseRule::Named(s) => write!(f, "rule:{}", s),
        }
    }
}

/// The primitive objects of the Spherepop runtime are events.
///
/// History grows monotonically; option space shrinks (on Pop).
/// Values and states are derived from histories — not the other way around.
///
/// v0.2 changes:
///   Refuse now carries RefusalReason (documented inadmissibility)
///
/// v0.3 changes:
///   Collapse now carries CollapseRule (observable state = quotient of history)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Pop: commit to a choice, removing it from the option space.
    Pop(Symbol),

    /// Refuse: record an inadmissible branch — WITH A DOCUMENTED REASON.
    /// The system survives refusal; the reason is part of the permanent record.
    Refuse {
        target: Symbol,
        reason: RefusalReason,
    },

    /// Bind: record a dependency between two symbols.
    Bind { name: Symbol, target: Symbol },

    /// Collapse: commit a branch into observable state via an explicit rule.
    /// Observable state is the quotient of history under this rule.
    Collapse {
        target: Symbol,
        rule: CollapseRule,
    },

    /// Lambda creation: record introduction of an abstraction.
    LamIntro { param: Symbol },

    /// Application: record elimination of an abstraction.
    Apply { fun: Symbol, arg: Symbol },

    /// Scope: mark the start and end of a lexical region.
    ScopeOpen(Symbol),
    ScopeClose(Symbol),
}

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Pop(s) => write!(f, "pop({})", s),
            Event::Refuse { target, reason } =>
                write!(f, "refuse({} ∵ {})", target, reason),
            Event::Bind { name, target } =>
                write!(f, "bind({} → {})", name, target),
            Event::Collapse { target, rule } =>
                write!(f, "collapse({} via {})", target, rule),
            Event::LamIntro { param } => write!(f, "λ({})", param),
            Event::Apply { fun, arg } => write!(f, "apply({}, {})", fun, arg),
            Event::ScopeOpen(s) => write!(f, "scope_open({})", s),
            Event::ScopeClose(s) => write!(f, "scope_close({})", s),
        }
    }
}
