use std::collections::HashMap;
use crate::core::event::Symbol;
use crate::syntax::ast::Term;
use super::ty::Type;

/// TypeMode controls how the checker handles free variables.
///
/// This is the v0.4 upgrade: the distinction must be named explicitly
/// rather than being a convenient runtime default.
///
/// ClosedWorld: strict — unknown variables are type errors.
///   Use this when you want soundness guarantees (compiler, proof mode).
///
/// OpenWorld: permissive — unknown variables are treated as symbolic atoms
///   with type Unit. Use this for REPL, prototyping, and exploratory work.
///
/// The key philosophical point: admissibility geometry changes under the
/// two modes. In ClosedWorld, the set of admissible terms is bounded by
/// the context. In OpenWorld, the option space is "everything not yet
/// refused." The two modes have different reachability structures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TypeMode {
    /// Reject unknown variables (sound, proof-grade).
    ClosedWorld,
    /// Treat unknown variables as Unit (permissive, REPL-grade).
    #[default]
    OpenWorld,
}

impl std::fmt::Display for TypeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeMode::ClosedWorld => write!(f, "closed-world"),
            TypeMode::OpenWorld => write!(f, "open-world"),
        }
    }
}

/// A typing context maps names to types.
#[derive(Debug, Clone, Default)]
pub struct Context {
    bindings: HashMap<String, Type>,
    pub mode: TypeMode,
}

impl Context {
    pub fn new() -> Self {
        Self { bindings: HashMap::new(), mode: TypeMode::OpenWorld }
    }

    pub fn closed() -> Self {
        Self { bindings: HashMap::new(), mode: TypeMode::ClosedWorld }
    }

    pub fn open() -> Self {
        Self { bindings: HashMap::new(), mode: TypeMode::OpenWorld }
    }

    pub fn with_mode(mut self, mode: TypeMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn extend(&self, name: &Symbol, ty: Type) -> Self {
        let mut ctx = self.clone();
        ctx.bindings.insert(name.0.clone(), ty);
        ctx
    }

    pub fn lookup(&self, name: &Symbol) -> Option<&Type> {
        self.bindings.get(&name.0)
    }
}

/// Type errors are failed reachability proofs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeError {
    UnboundVariable(Symbol),
    Mismatch { expected: Type, got: Type },
    InadmissibleCollapse(Type),
    NotAFunction(Type),
    UniverseLevelViolation { ty_level: u32, used_at: u32 },
    Other(String),
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeError::UnboundVariable(s) =>
                write!(f, "unbound variable: {} (closed-world mode)", s),
            TypeError::Mismatch { expected, got } =>
                write!(f, "type mismatch: expected {}, got {}", expected, got),
            TypeError::InadmissibleCollapse(t) =>
                write!(f, "inadmissible collapse of type {} (must be Admissible(_))", t),
            TypeError::NotAFunction(t) =>
                write!(f, "expected function type, got {}", t),
            TypeError::UniverseLevelViolation { ty_level, used_at } =>
                write!(f, "universe level {} used at level {}", ty_level, used_at),
            TypeError::Other(msg) => write!(f, "type error: {}", msg),
        }
    }
}

impl std::error::Error for TypeError {}

/// Infer the type of a term.
pub fn infer(ctx: &Context, term: &Term) -> Result<Type, TypeError> {
    match term {
        Term::Var(x) => {
            match ctx.lookup(x) {
                Some(ty) => Ok(ty.clone()),
                None => match ctx.mode {
                    // ClosedWorld: unknown variable is a hard error.
                    TypeMode::ClosedWorld => Err(TypeError::UnboundVariable(x.clone())),
                    // OpenWorld: unknown variable is a symbolic atom of type Unit.
                    TypeMode::OpenWorld => Ok(Type::Unit),
                },
            }
        }

        Term::Universe(n) => Ok(Type::Universe(n + 1)),

        Term::Ann(inner, ty) => {
            check(ctx, inner, ty)?;
            Ok(*ty.clone())
        }

        Term::Lam { param, ty, body } => {
            let body_ctx = ctx.extend(param, *ty.clone());
            let body_ty = infer(&body_ctx, body)?;
            Ok(Type::Pi {
                param: param.clone(),
                param_ty: ty.clone(),
                body_ty: Box::new(body_ty),
            })
        }

        Term::App(fun, arg) => {
            let fun_ty = infer(ctx, fun)?;
            match fun_ty {
                Type::Pi { param_ty, body_ty, .. } => {
                    check(ctx, arg, &param_ty)?;
                    Ok(*body_ty)
                }
                other => Err(TypeError::NotAFunction(other)),
            }
        }

        Term::Pi { .. } => Ok(Type::Universe(0)),

        Term::Let { name, value, body } => {
            let val_ty = infer(ctx, value)?;
            let body_ctx = ctx.extend(name, val_ty);
            infer(&body_ctx, body)
        }

        // Pop wraps in Admissible — committing to a choice produces a certificate.
        Term::Pop(inner) => {
            let inner_ty = infer(ctx, inner)?;
            Ok(Type::Admissible(Box::new(inner_ty)))
        }

        // Refuse wraps in Refused WITH the reason from the term.
        // v0.2: the reason is threaded through the type.
        Term::Refuse(inner, reason) => {
            let inner_ty = infer(ctx, inner)?;
            Ok(Type::Refused {
                inner: Box::new(inner_ty),
                reason: reason.clone(),
            })
        }

        // Collapse seals Admissible(_) into Collapsed(_, rule).
        // KEY RULE: collapse is only legal on an Admissible type.
        // v0.3: the collapse rule is threaded through the type.
        Term::Collapse(inner, rule) => {
            let inner_ty = infer(ctx, inner)?;
            match &inner_ty {
                Type::Admissible(t) => Ok(Type::Collapsed {
                    inner: t.clone(),
                    rule: rule.clone(),
                }),
                other => Err(TypeError::InadmissibleCollapse(other.clone())),
            }
        }

        Term::Bind(lhs, rhs) => {
            let lhs_ty = infer(ctx, lhs)?;
            let rhs_ty = infer(ctx, rhs)?;
            Ok(Type::Process {
                from: Box::new(lhs_ty),
                to: Box::new(rhs_ty),
            })
        }

        Term::Seq(terms) => {
            if terms.is_empty() { return Ok(Type::Unit); }
            let mut last = Type::Unit;
            for t in terms { last = infer(ctx, t)?; }
            Ok(last)
        }
    }
}

/// Check that a term has a given expected type.
pub fn check(ctx: &Context, term: &Term, expected: &Type) -> Result<(), TypeError> {
    let got = infer(ctx, term)?;
    if &got == expected {
        Ok(())
    } else {
        Err(TypeError::Mismatch { expected: expected.clone(), got })
    }
}

/// Admissibility judgment.
pub fn is_admissible(ctx: &Context, term: &Term) -> bool {
    match infer(ctx, term) {
        Ok(Type::Admissible(_)) | Ok(Type::Collapsed { .. }) => true,
        _ => false,
    }
}

/// Infer type using ClosedWorld mode from a fresh context.
pub fn infer_closed(term: &Term) -> Result<Type, TypeError> {
    infer(&Context::closed(), term)
}

/// Infer type using OpenWorld mode from a fresh context.
pub fn infer_open(term: &Term) -> Result<Type, TypeError> {
    infer(&Context::open(), term)
}
