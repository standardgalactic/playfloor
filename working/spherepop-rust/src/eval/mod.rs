pub mod value;
pub mod interpreter;

pub use value::{Value, Env};
pub use interpreter::EvalResult;
pub use interpreter::{eval, eval_with_options, eval_closed, EvalError, AdmissibilityStatus};
