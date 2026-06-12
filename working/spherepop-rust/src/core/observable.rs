use std::collections::HashMap;
use super::event::{CollapseRule, Event, Symbol};
use super::history::History;

/// Observable state is NOT stored in the World.
/// It is DERIVED from History by applying a CollapseRule.
///
/// This is the v0.3 upgrade: collapse defines a quotient/projection,
/// not merely an event stamp. Two histories that agree under a given
/// CollapseRule produce identical ObservableState under that rule.
///
/// This is the mechanical form of the admissibility geometry intuition:
/// the CollapseRule defines an equivalence relation on histories,
/// and ObservableState is the equivalence class representative.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservableState {
    /// The rule used to derive this observation.
    pub rule: CollapseRule,
    /// The derived symbolic state: each symbol mapped to its observable count/value.
    pub entries: HashMap<String, u64>,
    /// Whether any refusals appear in the history under this projection.
    pub has_refusals: bool,
    /// The names of all symbols that were refused (with reasons summarised).
    pub refused_symbols: Vec<(Symbol, String)>,
}

impl ObservableState {
    /// Derive observable state from history under a given collapse rule.
    pub fn derive(history: &History, rule: &CollapseRule) -> Self {
        match rule {
            CollapseRule::Identity => derive_identity(history),
            CollapseRule::LastWrite => derive_last_write(history),
            CollapseRule::Accumulate => derive_accumulate(history),
            CollapseRule::Projection(label) => derive_projection(history, label),
            CollapseRule::Named(name) => {
                // Named rules fall back to Identity for now;
                // a real implementation would look up a rule registry.
                let mut state = derive_identity(history);
                state.rule = CollapseRule::Named(name.clone());
                state
            }
        }
    }

    /// Two histories are observationally equivalent under a rule
    /// iff they produce the same ObservableState.
    pub fn equivalent(h1: &History, h2: &History, rule: &CollapseRule) -> bool {
        Self::derive(h1, rule) == Self::derive(h2, rule)
    }
}

impl std::fmt::Display for ObservableState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ObsState[{}]{{", self.rule)?;
        let mut pairs: Vec<_> = self.entries.iter().collect();
        pairs.sort_by_key(|(k, _)| k.as_str());
        for (i, (k, v)) in pairs.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{}:{}", k, v)?;
        }
        if self.has_refusals {
            write!(f, " | refused:{}", self.refused_symbols.len())?;
        }
        write!(f, "}}")
    }
}

// ── Rule implementations ────────────────────────────────────────────────────

/// Identity: every pop event contributes; pops are counted.
fn derive_identity(history: &History) -> ObservableState {
    let mut entries: HashMap<String, u64> = HashMap::new();
    let mut refused_symbols = Vec::new();
    let mut has_refusals = false;
    // For Identity, preserve the full event ordering as a synthetic entry
    // so that two histories with the same symbols but different orders
    // produce distinct ObservableStates.
    // We encode the ordered event sequence as a single "::order" key
    // whose value is a hash of the event sequence.
    let mut order_key: u64 = 0xcbf29ce484222325; // FNV offset basis
    for event in history.events() {
        // FNV-1a mixing of event display string
        for b in event.to_string().bytes() {
            order_key ^= b as u64;
            order_key = order_key.wrapping_mul(0x100000001b3);
        }
        match event {
            Event::Pop(s) => {
                *entries.entry(s.0.clone()).or_insert(0) += 1;
            }
            Event::Refuse { target, reason } => {
                has_refusals = true;
                refused_symbols.push((target.clone(), reason.to_string()));
            }
            Event::Collapse { target, .. } => {
                entries.entry(target.0.clone()).or_insert(0);
            }
            _ => {}
        }
    }
    // Encode the ordering fingerprint as a synthetic entry
    entries.insert("::order".to_string(), order_key);

    ObservableState {
        rule: CollapseRule::Identity,
        entries,
        has_refusals,
        refused_symbols,
    }
}

/// LastWrite: only the most recent pop for each symbol is observable.
fn derive_last_write(history: &History) -> ObservableState {
    let mut entries: HashMap<String, u64> = HashMap::new();
    let mut refused_symbols = Vec::new();
    let mut has_refusals = false;

    for event in history.events() {
        match event {
            Event::Pop(s) => {
                // Overwrite — last pop wins.
                entries.insert(s.0.clone(), 1);
            }
            Event::Refuse { target, reason } => {
                has_refusals = true;
                refused_symbols.push((target.clone(), reason.to_string()));
                // LastWrite: a refusal *removes* the symbol from observable state.
                entries.remove(&target.0);
            }
            _ => {}
        }
    }

    ObservableState {
        rule: CollapseRule::LastWrite,
        entries,
        has_refusals,
        refused_symbols,
    }
}

/// Accumulate: count every pop — order-insensitive.
fn derive_accumulate(history: &History) -> ObservableState {
    let mut entries: HashMap<String, u64> = HashMap::new();
    let mut refused_symbols = Vec::new();
    let mut has_refusals = false;

    for event in history.events() {
        match event {
            Event::Pop(s) => {
                *entries.entry(s.0.clone()).or_insert(0) += 1;
            }
            Event::Refuse { target, reason } => {
                has_refusals = true;
                refused_symbols.push((target.clone(), reason.to_string()));
            }
            _ => {}
        }
    }
    // NOTE: No ::order key — Accumulate is intentionally order-insensitive.
    ObservableState {
        rule: CollapseRule::Accumulate,
        entries,
        has_refusals,
        refused_symbols,
    }
}

/// Projection: only events tagged with a specific label matter.
/// Here we interpret the label as a prefix match on symbol names.
fn derive_projection(history: &History, label: &Symbol) -> ObservableState {
    let prefix = &label.0;
    let mut entries: HashMap<String, u64> = HashMap::new();
    let mut refused_symbols = Vec::new();
    let mut has_refusals = false;

    for event in history.events() {
        match event {
            Event::Pop(s) if s.0.starts_with(prefix.as_str()) => {
                *entries.entry(s.0.clone()).or_insert(0) += 1;
            }
            Event::Refuse { target, reason } if target.0.starts_with(prefix.as_str()) => {
                has_refusals = true;
                refused_symbols.push((target.clone(), reason.to_string()));
            }
            _ => {}
        }
    }

    ObservableState {
        rule: CollapseRule::Projection(label.clone()),
        entries,
        has_refusals,
        refused_symbols,
    }
}
