use crate::core::event::{CollapseRule, RefusalReason};

/// The Spherepop IR emits events, not instructions.
///
/// v0.2: Refuse carries RefusalReason.
/// v0.3: Collapse carries CollapseRule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrEvent {
    Load(String),
    PushUnit,
    PushType(u32),
    BeginLambda { param: String },
    EndLambda,
    Apply,
    Pop,
    /// v0.2: documented refusal reason.
    Refuse(RefusalReason),
    /// v0.3: explicit collapse rule.
    Collapse(CollapseRule),
    Bind,
    OpenScope(String),
    CloseScope(String),
    BeginSeq,
    EndSeq,
    Store(String),
    Return,
}

#[derive(Debug, Clone, Default)]
pub struct IrBlock {
    pub events: Vec<IrEvent>,
    pub name: String,
}

impl IrBlock {
    pub fn new(name: impl Into<String>) -> Self {
        Self { events: Vec::new(), name: name.into() }
    }

    pub fn emit(&mut self, e: IrEvent) { self.events.push(e); }
    pub fn len(&self) -> usize { self.events.len() }
    pub fn is_empty(&self) -> bool { self.events.is_empty() }
}

impl std::fmt::Display for IrBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "IrBlock({}):", self.name)?;
        for (i, e) in self.events.iter().enumerate() {
            writeln!(f, "  {:04}  {:?}", i, e)?;
        }
        Ok(())
    }
}
