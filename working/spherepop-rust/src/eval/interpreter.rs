use crate::core::World;
use crate::core::event::Symbol;
use crate::core::option_space::OptionSpace;
use crate::syntax::ast::Term;
use crate::types::ty::Type;
use super::value::{Env, Value};

#[derive(Debug, Clone)]
pub struct EvalStep {
    pub rule: &'static str,
    pub admissibility: AdmissibilityStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissibilityStatus {
    Admissible,
    Refused,
    Collapsed,
    Pending,
}

#[derive(Debug, Clone)]
pub struct EvalResult {
    pub value: Value,
    pub steps: Vec<EvalStep>,
    pub admissible: bool,
}

#[derive(Debug, Clone)]
pub enum EvalError {
    UnboundVariable(Symbol),
    NotAFunction(Value),
    UnavailableOption(Symbol),
    InadmissibleCollapse,
    Other(String),
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::UnboundVariable(s) => write!(f, "unbound variable: {}", s),
            EvalError::NotAFunction(v) => write!(f, "not a function: {}", v),
            EvalError::UnavailableOption(s) => write!(f, "unavailable option: {}", s),
            EvalError::InadmissibleCollapse => write!(f, "inadmissible collapse"),
            EvalError::Other(msg) => write!(f, "eval error: {}", msg),
        }
    }
}

pub fn eval(term: &Term, env: &Env, world: &mut World) -> Result<EvalResult, EvalError> {
    match term {
        Term::Var(x) => {
            // Open-world semantics: free variables evaluate to their symbol.
            let val = env.lookup(x).cloned().unwrap_or_else(|| Value::Symbol(x.clone()));
            Ok(EvalResult {
                value: val,
                steps: vec![EvalStep { rule: "Var", admissibility: AdmissibilityStatus::Pending }],
                admissible: true,
            })
        }

        Term::Lam { param, ty, body } => {
            world.record_lam_intro(param.clone());
            Ok(EvalResult {
                value: Value::Closure {
                    param: param.clone(),
                    param_ty: *ty.clone(),
                    body: body.clone(),
                    env: env.clone(),
                },
                steps: vec![EvalStep { rule: "Lam", admissibility: AdmissibilityStatus::Admissible }],
                admissible: true,
            })
        }

        Term::App(fun, arg) => {
            let fun_res = eval(fun, env, world)?;
            let arg_res = eval(arg, env, world)?;
            match fun_res.value {
                Value::Closure { param, body, env: cenv, .. } => {
                    let new_env = cenv.extend(param.clone(), arg_res.value);
                    world.record_apply(param.clone(), Symbol::new("arg"));
                    let body_res = eval(&body, &new_env, world)?;
                    let mut steps = fun_res.steps;
                    steps.extend(arg_res.steps);
                    steps.extend(body_res.steps);
                    steps.push(EvalStep { rule: "App-Beta", admissibility: AdmissibilityStatus::Admissible });
                    Ok(EvalResult { value: body_res.value, steps, admissible: body_res.admissible })
                }
                other => Err(EvalError::NotAFunction(other)),
            }
        }

        Term::Let { name, value, body } => {
            let val_res = eval(value, env, world)?;
            let new_env = env.extend(name.clone(), val_res.value);
            let body_res = eval(body, &new_env, world)?;
            let mut steps = val_res.steps;
            steps.extend(body_res.steps);
            Ok(EvalResult { value: body_res.value, steps, admissible: body_res.admissible })
        }

        Term::Universe(n) => Ok(EvalResult {
            value: Value::TypeValue(Type::Universe(*n)),
            steps: vec![EvalStep { rule: "Universe", admissibility: AdmissibilityStatus::Admissible }],
            admissible: true,
        }),

        Term::Pi { .. } => Ok(EvalResult {
            value: Value::TypeValue(Type::Unit),
            steps: vec![EvalStep { rule: "Pi", admissibility: AdmissibilityStatus::Admissible }],
            admissible: true,
        }),

        Term::Ann(inner, _) => eval(inner, env, world),

        Term::Pop(inner) => {
            let inner_res = eval(inner, env, world)?;
            let name = value_to_symbol(&inner_res.value);
            world.pop(name.clone()).map_err(|_| EvalError::UnavailableOption(name))?;
            let mut steps = inner_res.steps;
            steps.push(EvalStep { rule: "Pop", admissibility: AdmissibilityStatus::Admissible });
            Ok(EvalResult {
                value: Value::Admissible(Box::new(inner_res.value)),
                steps,
                admissible: true,
            })
        }

        // v0.2: Refuse threads the reason into the World and the Value.
        Term::Refuse(inner, reason) => {
            let inner_res = eval(inner, env, world)?;
            let name = value_to_symbol(&inner_res.value);
            world.refuse(name, reason.clone());
            let mut steps = inner_res.steps;
            steps.push(EvalStep { rule: "Refuse", admissibility: AdmissibilityStatus::Refused });
            Ok(EvalResult {
                value: Value::Refused(Box::new(inner_res.value), reason.clone()),
                steps,
                admissible: false,
            })
        }

        // v0.3: Collapse threads the rule into the World and the Value.
        Term::Collapse(inner, rule) => {
            let inner_res = eval(inner, env, world)?;
            if !inner_res.admissible {
                return Err(EvalError::InadmissibleCollapse);
            }
            let name = value_to_symbol(&inner_res.value);
            world.collapse(name, rule.clone());
            let mut steps = inner_res.steps;
            steps.push(EvalStep { rule: "Collapse", admissibility: AdmissibilityStatus::Collapsed });
            Ok(EvalResult {
                value: Value::Collapsed(Box::new(inner_res.value), rule.clone()),
                steps,
                admissible: true,
            })
        }

        Term::Bind(lhs, rhs) => {
            let lhs_res = eval(lhs, env, world)?;
            let rhs_res = eval(rhs, env, world)?;
            world.bind(value_to_symbol(&lhs_res.value), value_to_symbol(&rhs_res.value));
            let mut steps = lhs_res.steps;
            steps.extend(rhs_res.steps);
            steps.push(EvalStep { rule: "Bind", admissibility: AdmissibilityStatus::Admissible });
            Ok(EvalResult {
                value: Value::Process { from: Box::new(lhs_res.value), to: Box::new(rhs_res.value) },
                steps,
                admissible: true,
            })
        }

        Term::Seq(terms) => {
            if terms.is_empty() {
                return Ok(EvalResult {
                    value: Value::Unit,
                    steps: vec![],
                    admissible: true,
                });
            }
            let mut all_steps = Vec::new();
            let mut last = EvalResult { value: Value::Unit, steps: vec![], admissible: true };
            for t in terms {
                last = eval(t, env, world)?;
                all_steps.extend(last.steps.iter().cloned());
            }
            Ok(EvalResult { value: last.value, steps: all_steps, admissible: last.admissible })
        }
    }
}

fn value_to_symbol(v: &Value) -> Symbol {
    match v {
        Value::Symbol(s) => s.clone(),
        Value::Unit => Symbol::new("unit"),
        Value::Closure { param, .. } => Symbol::new(format!("closure_{}", param)),
        Value::Admissible(inner) => Symbol::new(format!("adm_{}", value_to_symbol(inner))),
        Value::Refused(inner, _) => Symbol::new(format!("ref_{}", value_to_symbol(inner))),
        Value::Collapsed(inner, _) => Symbol::new(format!("col_{}", value_to_symbol(inner))),
        Value::Process { from, to } =>
            Symbol::new(format!("proc_{}_{}", value_to_symbol(from), value_to_symbol(to))),
        Value::TypeValue(t) => Symbol::new(format!("type_{}", t)),
    }
}

pub fn eval_with_options(
    term: &Term,
    options: impl IntoIterator<Item = Symbol>,
) -> Result<(EvalResult, World), EvalError> {
    let mut world = World::new(OptionSpace::new(options));
    let result = eval(term, &Env::new(), &mut world)?;
    Ok((result, world))
}

pub fn eval_closed(term: &Term) -> Result<(EvalResult, World), EvalError> {
    let mut world = World::empty();
    let result = eval(term, &Env::new(), &mut world)?;
    Ok((result, world))
}
