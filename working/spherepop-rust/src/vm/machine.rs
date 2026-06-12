use crate::core::World;
use crate::core::event::Symbol;
use crate::core::option_space::OptionSpace;
use crate::eval::value::{Env, Value};
use crate::ir::event_ir::{IrEvent, IrBlock};
use crate::types::ty::Type;

#[derive(Debug, Clone)]
pub enum VmError {
    UnboundVariable(String),
    StackUnderflow,
    NotAFunction,
    UnavailableOption(String),
    InadmissibleCollapse,
    Other(String),
}

impl std::fmt::Display for VmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmError::UnboundVariable(s) => write!(f, "vm: unbound variable: {}", s),
            VmError::StackUnderflow => write!(f, "vm: stack underflow"),
            VmError::NotAFunction => write!(f, "vm: not a function"),
            VmError::UnavailableOption(s) => write!(f, "vm: unavailable option: {}", s),
            VmError::InadmissibleCollapse => write!(f, "vm: inadmissible collapse"),
            VmError::Other(msg) => write!(f, "vm: {}", msg),
        }
    }
}

pub struct Machine {
    pub stack: Vec<Value>,
    pub env: Env,
    pub world: World,
}

impl Machine {
    pub fn new(options: impl IntoIterator<Item = Symbol>) -> Self {
        Self { stack: Vec::new(), env: Env::new(), world: World::new(OptionSpace::new(options)) }
    }

    pub fn empty() -> Self {
        Self { stack: Vec::new(), env: Env::new(), world: World::empty() }
    }

    fn push(&mut self, v: Value) { self.stack.push(v); }

    fn pop_stack(&mut self) -> Result<Value, VmError> {
        self.stack.pop().ok_or(VmError::StackUnderflow)
    }

    pub fn run(&mut self, block: &IrBlock) -> Result<Value, VmError> {
        for event in &block.events {
            self.step(event)?;
        }
        Ok(self.stack.last().cloned().unwrap_or(Value::Unit))
    }

    fn step(&mut self, event: &IrEvent) -> Result<(), VmError> {
        match event {
            IrEvent::Load(name) => {
                let sym = Symbol::new(name.clone());
                let val = self.env.lookup(&sym).cloned().unwrap_or(Value::Symbol(sym));
                self.push(val);
            }
            IrEvent::PushUnit => self.push(Value::Unit),
            IrEvent::PushType(n) => self.push(Value::TypeValue(Type::Universe(*n))),

            IrEvent::Store(name) => {
                let val = self.pop_stack()?;
                self.env = self.env.extend(Symbol::new(name.clone()), val);
            }

            IrEvent::BeginLambda { param } => {
                self.world.record_lam_intro(Symbol::new(param.clone()));
            }
            IrEvent::EndLambda => {}

            IrEvent::Apply => {
                let arg = self.pop_stack()?;
                let fun = self.pop_stack()?;
                match fun {
                    Value::Closure { param, env: cenv, .. } => {
                        self.world.record_apply(param.clone(), Symbol::new("arg"));
                        self.push(Value::Admissible(Box::new(Value::Symbol(param))));
                        let _ = (arg, cenv);
                    }
                    other => {
                        self.world.record_apply(Symbol::new("unknown"), Symbol::new("arg"));
                        self.push(Value::Process { from: Box::new(other), to: Box::new(arg) });
                    }
                }
            }

            IrEvent::Pop => {
                let top = self.pop_stack()?;
                let name = value_to_symbol(&top);
                self.world.pop(name.clone())
                    .map_err(|_| VmError::UnavailableOption(name.0.clone()))?;
                self.push(Value::Admissible(Box::new(top)));
            }

            // v0.2: reason passed through to World.
            IrEvent::Refuse(reason) => {
                let top = self.pop_stack()?;
                let name = value_to_symbol(&top);
                self.world.refuse(name, reason.clone());
                self.push(Value::Refused(Box::new(top), reason.clone()));
            }

            // v0.3: rule passed through to World.
            IrEvent::Collapse(rule) => {
                let top = self.pop_stack()?;
                match &top {
                    Value::Admissible(_) => {
                        let name = value_to_symbol(&top);
                        self.world.collapse(name, rule.clone());
                        self.push(Value::Collapsed(Box::new(top), rule.clone()));
                    }
                    _ => return Err(VmError::InadmissibleCollapse),
                }
            }

            IrEvent::Bind => {
                let rhs = self.pop_stack()?;
                let lhs = self.pop_stack()?;
                self.world.bind(value_to_symbol(&lhs), value_to_symbol(&rhs));
                self.push(Value::Process { from: Box::new(lhs), to: Box::new(rhs) });
            }

            IrEvent::OpenScope(name) => self.world.open_scope(Symbol::new(name.clone())),
            IrEvent::CloseScope(name) => {
                self.world.close_scope(Symbol::new(name.clone()))
                    .map_err(|e| VmError::Other(e.to_string()))?;
            }

            IrEvent::BeginSeq | IrEvent::EndSeq | IrEvent::Return => {}
        }
        Ok(())
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
