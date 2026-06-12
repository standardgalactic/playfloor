use super::event::Symbol;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// Attempted to pop a symbol not in the option space.
    UnavailableOption(Symbol),
    /// Attempted to collapse a term that is not admissible.
    InadmissibleCollapse(Symbol),
    /// History replay mismatch: compiled and interpreted paths diverged.
    HistoryMismatch { expected: usize, got: usize },
    /// Scope closed before it was opened.
    UnbalancedScope(Symbol),
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreError::UnavailableOption(s) =>
                write!(f, "cannot pop unavailable option: {}", s),
            CoreError::InadmissibleCollapse(s) =>
                write!(f, "inadmissible collapse: {}", s),
            CoreError::HistoryMismatch { expected, got } =>
                write!(f, "history mismatch: expected {} events, got {}", expected, got),
            CoreError::UnbalancedScope(s) =>
                write!(f, "unbalanced scope: {}", s),
        }
    }
}

impl std::error::Error for CoreError {}
