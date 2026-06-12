use std::collections::BTreeSet;
use super::event::Symbol;

/// The OptionSpace is the set of currently reachable / available choices.
///
/// Invariant: every Pop event removes one element.
/// Refuse does NOT remove from option space — it records inadmissibility
/// without foreclosing possibilities.
#[derive(Debug, Clone, Default)]
pub struct OptionSpace {
    options: BTreeSet<Symbol>,
}

impl OptionSpace {
    pub fn new(options: impl IntoIterator<Item = Symbol>) -> Self {
        Self {
            options: options.into_iter().collect(),
        }
    }

    pub fn empty() -> Self {
        Self { options: BTreeSet::new() }
    }

    /// Insert a new option (used when a scope is opened or a lambda is introduced).
    pub fn insert(&mut self, s: Symbol) {
        self.options.insert(s);
    }

    pub fn contains(&self, s: &Symbol) -> bool {
        self.options.contains(s)
    }

    /// Remove and return whether it was present.
    pub fn remove(&mut self, s: &Symbol) -> bool {
        self.options.remove(s)
    }

    pub fn restrict<F>(&mut self, mut keep: F)
    where
        F: FnMut(&Symbol) -> bool,
    {
        self.options.retain(|x| keep(x));
    }

    pub fn len(&self) -> usize {
        self.options.len()
    }

    pub fn is_empty(&self) -> bool {
        self.options.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Symbol> {
        self.options.iter()
    }
}

impl std::fmt::Display for OptionSpace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        for (i, s) in self.options.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{}", s)?;
        }
        write!(f, "}}")
    }
}
