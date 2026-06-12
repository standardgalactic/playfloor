pub mod event;
pub mod error;
pub mod history;
pub mod observable;
pub mod option_space;
pub mod reachability;

use event::{CollapseRule, Event, RefusalReason, Symbol};
use error::CoreError;
use history::History;
use observable::ObservableState;
use option_space::OptionSpace;
pub use reachability::{
    ReachabilityGeometry, forward_project, deform, compose_deformations,
    possibility_functional, admissibility_field, observational_entropy,
    two_projection, fibre, verify_conservation,
};

/// The World is the fundamental Spherepop runtime state.
///
/// It pairs an append-only History with a shrinking OptionSpace.
/// Observable state is ALWAYS derived from history — never stored.
///
/// The World also tracks the initial option space size |Ω₀| for
/// conservation law verification (Theorem 22.1).
#[derive(Debug, Clone)]
pub struct World {
    pub options: OptionSpace,
    pub history: History,
    /// |Ω₀|: initial option space size for conservation law.
    pub initial_size: usize,
}

impl World {
    pub fn new(options: OptionSpace) -> Self {
        let initial_size = options.len();
        Self { options, history: History::new(), initial_size }
    }

    pub fn empty() -> Self {
        Self { options: OptionSpace::empty(), history: History::new(), initial_size: 0 }
    }

    // ── Primitive operations ───────────────────────────────────────────────

    pub fn pop(&mut self, choice: Symbol) -> Result<(), CoreError> {
        if !self.options.remove(&choice) {
            return Err(CoreError::UnavailableOption(choice));
        }
        self.history.append(Event::Pop(choice));
        Ok(())
    }

    pub fn refuse(&mut self, target: Symbol, reason: RefusalReason) {
        self.history.append(Event::Refuse { target, reason });
    }

    pub fn bind(&mut self, name: Symbol, target: Symbol) {
        self.history.append(Event::Bind { name, target });
    }

    pub fn collapse(&mut self, target: Symbol, rule: CollapseRule) {
        self.history.append(Event::Collapse { target, rule });
    }

    pub fn open_scope(&mut self, name: Symbol) {
        self.options.insert(name.clone());
        self.history.append(Event::ScopeOpen(name));
    }

    pub fn close_scope(&mut self, name: Symbol) -> Result<(), CoreError> {
        if !self.options.remove(&name) {
            return Err(CoreError::UnbalancedScope(name.clone()));
        }
        self.history.append(Event::ScopeClose(name));
        Ok(())
    }

    pub fn record_lam_intro(&mut self, param: Symbol) {
        self.history.append(Event::LamIntro { param });
    }

    pub fn record_apply(&mut self, fun: Symbol, arg: Symbol) {
        self.history.append(Event::Apply { fun, arg });
    }

    // ── v0.3: Derived observations ─────────────────────────────────────────

    pub fn observe(&self, rule: &CollapseRule) -> ObservableState {
        ObservableState::derive(&self.history, rule)
    }

    pub fn observe_identity(&self) -> ObservableState {
        self.observe(&CollapseRule::Identity)
    }

    // ── v0.2: Derived admissibility ────────────────────────────────────────

    pub fn commitment_depth(&self) -> usize { self.history.pop_count() }
    pub fn refusal_count(&self) -> usize { self.history.refuse_count() }

    pub fn is_admissible(&self, s: &Symbol) -> bool {
        !self.history.ever_refused(s)
    }

    pub fn refusal_reason(&self, s: &Symbol) -> Option<&RefusalReason> {
        self.history.events().iter().find_map(|e| {
            if let Event::Refuse { target, reason } = e {
                if target == s { Some(reason) } else { None }
            } else { None }
        })
    }

    // ── v4: Forward projection and reachability geometry ──────────────────

    /// Forward projection ρ(H, Ω) → ReachabilityGeometry.
    /// Maps the current world state to its reachability geometry.
    pub fn reachability_geometry(&self) -> ReachabilityGeometry {
        forward_project(&self.history, &self.options)
    }

    /// Graded admissibility field φ(H, Ω) ∈ [0,1].
    /// 1.0 = all originally available options still reachable.
    /// 0.0 = no options reachable.
    pub fn admissibility_field(&self) -> f64 {
        admissibility_field(&self.history, &self.options, self.initial_size)
    }

    /// Generalised possibility functional Π(H, Ω).
    /// Conserved quantity: Π(H_t, Ω_t) = |Ω₀| for all t.
    pub fn possibility_functional(&self) -> usize {
        possibility_functional(&self.history, &self.options)
    }

    /// Verify the conservation law Π(H, Ω) = |Ω₀|.
    pub fn verify_conservation(&self) -> Result<(), String> {
        verify_conservation(&self.history, &self.options, self.initial_size)
    }

    /// Two-projection architecture: World → F → O_c.
    pub fn two_projection(&self, rule: &CollapseRule) -> (ReachabilityGeometry, ObservableState) {
        two_projection(&self.history, &self.options, rule)
    }
}

impl std::fmt::Display for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "World {{ options: {}, history: {}, φ: {:.2} }}",
            self.options, self.history, self.admissibility_field())
    }
}
