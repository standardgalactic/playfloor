pub mod ty;
pub mod checker;

pub use ty::Type;
pub use checker::{Context, TypeMode, TypeError, infer, check, is_admissible};

/// Infer type with ClosedWorld mode.
pub fn infer_closed(term: &crate::syntax::ast::Term) -> Result<Type, TypeError> {
    checker::infer(&Context::closed(), term)
}

/// Infer type with OpenWorld mode.
pub fn infer_open(term: &crate::syntax::ast::Term) -> Result<Type, TypeError> {
    checker::infer(&Context::open(), term)
}
