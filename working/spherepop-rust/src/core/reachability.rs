/// Reachability geometry — the forward-pointing structure.
///
/// In v0.1–v0.5, the framework was retrospective: histories record the past,
/// states are projections of what happened.
///
/// This module adds the forward projection:
///   ρ : (H, Ω) → F
/// mapping world states to *reachability geometries* — descriptions of
/// what can still happen from the current world state.
///
/// The two-projection architecture is:
///   World ──ρ──► F ──πc──► O_c
///
/// Observable state is a projection of a projection:
/// it is a quotient of future geometry, not a direct quotient of history.
///
/// This also implements:
///   - Admissibility fields φ : H → [0,1] (graded admissibility)
///   - Deformation operators δe : F → F (per-event geometry change)
///   - The generalised possibility functional Π(H,Ω)
///   - Observational entropy S_c(o) = log|c⁻¹(o)|

use std::collections::BTreeSet;
use super::event::{CollapseRule, Event, Symbol};
use super::history::History;
use super::observable::ObservableState;
use super::option_space::OptionSpace;

/// A reachability geometry describes what can still happen from a world state.
///
/// It is the pair (admissibility_set, option_space):
///   - admissibility_set: symbols that have not been refused (semantic availability)
///   - option_space: symbols structurally available (structural availability)
///
/// The two sets can diverge: refusal shrinks admissibility without touching structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReachabilityGeometry {
    /// Admissible futures: symbols not yet refused.
    pub admissible: BTreeSet<Symbol>,
    /// Structurally available: symbols not yet popped.
    pub available: BTreeSet<Symbol>,
}

impl ReachabilityGeometry {
    /// Construct the initial geometry from the full option space.
    pub fn initial(omega_0: &OptionSpace) -> Self {
        let syms: BTreeSet<Symbol> = omega_0.iter().cloned().collect();
        Self {
            admissible: syms.clone(),
            available: syms,
        }
    }

    /// Number of reachable possibilities: symbols that are both
    /// admissible and structurally available.
    pub fn reachable_count(&self) -> usize {
        self.admissible.intersection(&self.available).count()
    }

    /// True if a symbol is fully reachable (admissible AND available).
    pub fn is_reachable(&self, s: &Symbol) -> bool {
        self.admissible.contains(s) && self.available.contains(s)
    }

    /// True if the geometry has any reachable possibility.
    pub fn has_reachable(&self) -> bool {
        self.reachable_count() > 0
    }
}

impl std::fmt::Display for ReachabilityGeometry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let reachable: BTreeSet<&Symbol> =
            self.admissible.intersection(&self.available).collect();
        write!(f, "R{{adm:{}, avail:{}, reach:{}}}",
            self.admissible.len(), self.available.len(), reachable.len())
    }
}

/// The forward projection ρ : (H, Ω) → ReachabilityGeometry.
///
/// Maps a world state to its reachability geometry — what can still happen.
pub fn forward_project(history: &History, options: &OptionSpace) -> ReachabilityGeometry {
    // Admissible: everything in Ω₀ that was never refused.
    let mut admissible: BTreeSet<Symbol> = options.iter().cloned().collect();
    // Add symbols that were popped (they were admissible when popped).
    for event in history.events() {
        if let Event::Pop(s) = event {
            admissible.insert(s.clone());
        }
    }
    // Remove symbols that were refused.
    for event in history.events() {
        if let Event::Refuse { target, .. } = event {
            admissible.remove(target);
        }
    }
    // Structurally available: current option space (already tracks this).
    let available: BTreeSet<Symbol> = options.iter().cloned().collect();
    ReachabilityGeometry { admissible, available }
}

/// Deformation operators: how each event type transforms reachability geometry.
///
/// Pop deforms structural option space (forecloses structural possibility).
/// Refuse deforms admissibility set (forecloses semantic possibility).
/// Bind and Collapse leave the reachability geometry unchanged.
pub fn deform(geom: &ReachabilityGeometry, event: &Event) -> ReachabilityGeometry {
    let mut geom = geom.clone();
    match event {
        Event::Pop(x) => {
            // Pop removes x from structural availability.
            geom.available.remove(x);
        }
        Event::Refuse { target, .. } => {
            // Refuse removes target from admissibility (not from availability).
            geom.admissible.remove(target);
        }
        Event::Bind { .. } | Event::Collapse { .. }
        | Event::LamIntro { .. } | Event::Apply { .. }
        | Event::ScopeOpen(_) | Event::ScopeClose(_) => {
            // These events do not change the reachability geometry.
        }
    }
    geom
}

/// Apply a history as a composed sequence of deformations.
///
/// This is the implementation of Proposition 22.4 (Histories as Composed Deformations):
///   ρ(H, Ω₀) = (δeₙ ∘ ⋯ ∘ δe₁)(Ω₀, Ω₀)
pub fn compose_deformations(history: &History, initial: &OptionSpace) -> ReachabilityGeometry {
    let mut geom = ReachabilityGeometry::initial(initial);
    for event in history.events() {
        geom = deform(&geom, event);
    }
    geom
}

/// The generalised possibility functional Π(H, Ω).
///
/// Implements the conservation theorem (Theorem 22.1):
///   Π(H_t, Ω_t) = |Ω₀| for all legal executions.
///
/// The weight function w assigns:
///   w(Pop)     = 1  (consumes possibility)
///   w(Refuse)  = 0  (documents inadmissibility without consuming)
///   w(Bind)    = 0
///   w(Collapse)= 0
///   w(others)  = 0
pub fn possibility_functional(history: &History, options: &OptionSpace) -> usize {
    let consumed: usize = history.events().iter().filter_map(|e| {
        match e {
            Event::Pop(_) => Some(1),
            _ => Some(0),
        }
    }).sum();
    options.len() + consumed
}

/// Admissibility field: a graded measure of how "open" the future is.
///
/// φ(H, Ω) ∈ [0.0, 1.0] where:
///   1.0 = full admissibility (all originally available options still reachable)
///   0.0 = no admissibility (all options either popped or refused)
///
/// This is the continuous generalisation of Boolean admissibility.
pub fn admissibility_field(history: &History, options: &OptionSpace,
                            initial_size: usize) -> f64 {
    if initial_size == 0 { return 1.0; }
    let geom = forward_project(history, options);
    geom.reachable_count() as f64 / initial_size as f64
}

/// Observational entropy S_c(o) = log|c⁻¹(o)|.
///
/// Implements Definition 13.8 (Observational Entropy).
/// Given a set of histories and an observable state,
/// counts how many histories produce that state (fibre cardinality)
/// and returns the log.
///
/// This is the discrete Spherepop instance of CLIO representational entropy:
///   S_π(m) = log Vol(π⁻¹(m))
pub fn observational_entropy(
    histories: &[History],
    target: &ObservableState,
    rule: &CollapseRule,
) -> f64 {
    let fibre_size = histories.iter()
        .filter(|h| &ObservableState::derive(h, rule) == target)
        .count();
    if fibre_size == 0 { f64::NEG_INFINITY }
    else { (fibre_size as f64).ln() }
}

/// Two-projection architecture: World → F → O_c.
///
/// Implements Definition 22.9 (Two-Projection Architecture).
/// Returns (reachability_geometry, observable_state) as the two-stage projection.
pub fn two_projection(
    history: &History,
    options: &OptionSpace,
    rule: &CollapseRule,
) -> (ReachabilityGeometry, ObservableState) {
    let geom = forward_project(history, options);
    let obs = ObservableState::derive(history, rule);
    (geom, obs)
}

/// Fibre of an observable state: all histories (from a set) that collapse to it.
///
/// Implements Definition 19.4 (Inverse-Problem Fibre).
pub fn fibre<'a>(
    histories: &'a [History],
    target: &ObservableState,
    rule: &CollapseRule,
) -> Vec<&'a History> {
    histories.iter()
        .filter(|h| &ObservableState::derive(h, rule) == target)
        .collect()
}

/// Verify the conservation law Π(H_t, Ω_t) = |Ω₀|.
///
/// Returns Ok(()) if the invariant holds, Err with a description if not.
pub fn verify_conservation(
    history: &History,
    options: &OptionSpace,
    initial_size: usize,
) -> Result<(), String> {
    let actual = possibility_functional(history, options);
    if actual == initial_size {
        Ok(())
    } else {
        Err(format!(
            "conservation violated: Π = {} but |Ω₀| = {}",
            actual, initial_size
        ))
    }
}
