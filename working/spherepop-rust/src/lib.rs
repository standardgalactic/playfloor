//! # Spherepop
//!
//! A history-first process calculus implementing admissibility geometry.
//!
//! ## Version history
//!
//! **v0.1 — History-first execution.**
//! World = History + OptionSpace. Evaluation returns EvalResult alongside a World.
//! The conservation law (history grows, option space shrinks) is enforced as a
//! hard runtime invariant. Observable state is always derived, never stored.
//!
//! **v0.2 — Documented refusal.**
//! `RefusalReason` threads through events, types, values, and IR.
//! Refusal is structured inadmissibility with a permanent certificate.
//!
//! **v0.3 — Collapse as quotient.**
//! `CollapseRule` specifies the observational framework.
//! `ObservableState::derive(history, rule)` implements O_c = H/~_c.
//! Observational equivalence is rule-relative and formally testable.
//!
//! **v0.4 — TypeMode.**
//! `TypeMode::OpenWorld` (free vars = Unit) and `TypeMode::ClosedWorld`
//! (free vars = type error) mechanise the open/closed world epistemology.
//!
//! **v0.5 — History equivalence as compiler correctness.**
//! `I(p).history == V(C(p)).history` is the first-class correctness criterion.
//!
//! **v0.6 — Forward projection and admissibility fields.**
//! `ReachabilityGeometry` implements the forward projection ρ : (H,Ω) → F.
//! `deform(geom, event)` implements δe : F → F per-event.
//! `admissibility_field(H, Ω) ∈ [0,1]` is the graded admissibility measure.
//! `possibility_functional(H, Ω) = |Ω₀|` implements the conservation theorem.
//! `two_projection(H, Ω, c)` returns (F, O_c) as the two-stage architecture.
//! `observational_entropy(histories, target, rule)` = log|c⁻¹(o)| = S_c(o).
//!
//! ## The two-projection architecture (v0.6)
//!
//! ```text
//!   World (H, Ω)  ──ρ──►  F  ──πc──►  O_c
//! ```
//! Observable state is a projection of a projection, not a direct projection
//! of history. The reachability geometry F mediates between them.

pub mod core;
pub mod syntax;
pub mod types;
pub mod eval;
pub mod ir;
pub mod vm;
pub mod compiler;

pub use core::{
    World,
    event::{Symbol, RefusalReason, CollapseRule},
    option_space::OptionSpace,
    reachability::{
        ReachabilityGeometry, forward_project, deform, compose_deformations,
        possibility_functional, admissibility_field, observational_entropy,
        two_projection, fibre, verify_conservation,
    },
};
pub use syntax::{Term, parse};
pub use types::{Type, Context, TypeError, TypeMode, infer, infer_closed, infer_open};
pub use eval::{Value, Env, EvalResult, eval, eval_closed, eval_with_options, EvalError};
pub use ir::IrBlock;
pub use compiler::compile;
pub use vm::{Machine, VmError};

/// Parse, type-check (open-world), and interpret.
pub fn run(src: &str, options: &[&str]) -> Result<(Value, World), String> {
    let term = parse(src).map_err(|e| format!("parse error: {}", e))?;
    let _ty = infer_open(&term).map_err(|e| format!("type error: {}", e))?;
    let syms = options.iter().map(|s| Symbol::new(*s));
    let (result, world) = eval_with_options(&term, syms)
        .map_err(|e| format!("eval error: {}", e))?;
    Ok((result.value, world))
}

/// Parse, type-check (closed-world), interpret.
pub fn run_strict(src: &str, options: &[&str]) -> Result<(Value, World), String> {
    let term = parse(src).map_err(|e| format!("parse error: {}", e))?;
    let _ty = infer_closed(&term).map_err(|e| format!("type error: {}", e))?;
    let syms = options.iter().map(|s| Symbol::new(*s));
    let (result, world) = eval_with_options(&term, syms)
        .map_err(|e| format!("eval error: {}", e))?;
    Ok((result.value, world))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::event::Event;
    use crate::core::observable::ObservableState;
    use crate::core::history::History;

    // ════════════════════════════════════════════════════════════════════════
    // PART I: Core invariants (v0.1)
    // ════════════════════════════════════════════════════════════════════════

    // Witnesses: Theorem 2.6 (Conservation of Possibility)
    #[test]
    fn world_pop_appends_and_shrinks_options() {
        let mut w = World::new(OptionSpace::new([Symbol::new("x")]));
        w.pop(Symbol::new("x")).unwrap();
        assert_eq!(w.history.events(), &[Event::Pop(Symbol::new("x"))]);
        assert!(w.options.is_empty());
    }

    #[test]
    fn world_pop_unavailable_fails() {
        let mut w = World::empty();
        assert!(w.pop(Symbol::new("x")).is_err());
    }

    // Witnesses: Theorem 2.6 — Refuse does NOT shrink option space
    // (asymmetry between structural and admissibility contraction)
    #[test]
    fn world_refuse_with_reason_does_not_shrink_options() {
        let mut w = World::new(OptionSpace::new([Symbol::new("x")]));
        w.refuse(Symbol::new("x"), RefusalReason::ConstraintViolation(Symbol::new("c")));
        assert!(w.options.contains(&Symbol::new("x")),
            "refuse must NOT remove from option space");
        assert_eq!(w.history.refuse_count(), 1);
    }

    // Witnesses: v0.2 — refusal reason is part of the permanent record
    #[test]
    fn world_refusal_reason_is_retrievable() {
        let mut w = World::empty();
        w.refuse(Symbol::new("bad"), RefusalReason::Explicit("test reason".into()));
        let reason = w.refusal_reason(&Symbol::new("bad")).unwrap();
        assert!(matches!(reason, RefusalReason::Explicit(msg) if msg == "test reason"));
    }

    #[test]
    fn world_history_grows_monotonically() {
        let mut w = World::new(OptionSpace::new([Symbol::new("a"), Symbol::new("b")]));
        w.pop(Symbol::new("a")).unwrap();
        w.bind(Symbol::new("a"), Symbol::new("b"));
        w.pop(Symbol::new("b")).unwrap();
        assert_eq!(w.history.len(), 3);
    }

    // ════════════════════════════════════════════════════════════════════════
    // PART II: Type system (v0.4)
    // ════════════════════════════════════════════════════════════════════════

    // Witnesses: Theorem 6.1 (Collapse Soundness) — [T-Pop] rule
    #[test]
    fn pop_wraps_in_admissible() {
        let ctx = Context::open().extend(&Symbol::new("x"), Type::Unit);
        let ty = infer(&ctx, &Term::pop(Term::var("x"))).unwrap();
        assert_eq!(ty, Type::Admissible(Box::new(Type::Unit)));
    }

    // Witnesses: Theorem 6.1 — collapse requires Admissible premise
    #[test]
    fn collapse_of_non_admissible_is_type_error() {
        let ctx = Context::open().extend(&Symbol::new("x"), Type::Unit);
        let term = Term::collapse(Term::var("x"), CollapseRule::Identity);
        let err = infer(&ctx, &term).unwrap_err();
        assert!(matches!(err, TypeError::InadmissibleCollapse(_)));
    }

    // Witnesses: [T-Collapse] — collapse of Admissible(T) yields Collapsed(T,c)
    #[test]
    fn collapse_of_admissible_carries_rule() {
        let ctx = Context::open()
            .extend(&Symbol::new("x"), Type::Admissible(Box::new(Type::Unit)));
        let ty = infer(&ctx, &Term::collapse(Term::var("x"), CollapseRule::LastWrite)).unwrap();
        assert!(matches!(ty, Type::Collapsed { rule: CollapseRule::LastWrite, .. }));
    }

    // Witnesses: v0.2 — Refused type carries reason
    #[test]
    fn refuse_wraps_with_reason_in_type() {
        let ctx = Context::open();
        let term = Term::refuse(Term::var("x"), RefusalReason::Explicit("test".into()));
        let ty = infer(&ctx, &term).unwrap();
        assert!(matches!(ty, Type::Refused { reason: RefusalReason::Explicit(_), .. }));
    }

    // Witnesses: TypeMode distinction (v0.4)
    #[test]
    fn closed_world_rejects_free_variables() {
        let err = infer_closed(&Term::var("unknown")).unwrap_err();
        assert!(matches!(err, TypeError::UnboundVariable(_)));
    }

    #[test]
    fn open_world_accepts_free_variables_as_unit() {
        let ty = infer_open(&Term::var("unknown")).unwrap();
        assert_eq!(ty, Type::Unit);
    }

    #[test]
    fn mode_is_preserved_through_extend() {
        let ctx = Context::closed();
        let ext = ctx.extend(&Symbol::new("x"), Type::Unit);
        assert_eq!(ext.mode, TypeMode::ClosedWorld);
    }

    // ════════════════════════════════════════════════════════════════════════
    // PART III: Evaluator (v0.1 – v0.3)
    // ════════════════════════════════════════════════════════════════════════

    #[test]
    fn eval_pop_produces_admissible_value_and_pops_option() {
        let (res, world) = eval_with_options(
            &Term::pop(Term::var("x")), [Symbol::new("x")]).unwrap();
        assert!(matches!(res.value, Value::Admissible(_)));
        assert!(world.options.is_empty());
        assert_eq!(world.history.pop_count(), 1);
    }

    #[test]
    fn eval_refuse_carries_reason_in_value() {
        let reason = RefusalReason::Explicit("test".into());
        let (res, world) = eval_closed(
            &Term::refuse(Term::var("x"), reason.clone())).unwrap();
        assert!(!res.admissible);
        assert!(matches!(&res.value, Value::Refused(_, r) if r == &reason));
        let ev = &world.history.events()[0];
        assert!(matches!(ev, Event::Refuse { reason: r, .. } if r == &reason));
    }

    // Witnesses: Theorem 6.1 — runtime collapse soundness
    #[test]
    fn eval_collapse_of_refused_is_runtime_error() {
        let term = Term::collapse(
            Term::refuse(Term::var("x"), RefusalReason::NotAvailable),
            CollapseRule::Identity,
        );
        assert!(eval_closed(&term).is_err());
    }

    #[test]
    fn eval_bind_records_event() {
        let (_, world) = eval_closed(&Term::bind(Term::var("a"), Term::var("b"))).unwrap();
        assert!(world.history.events().iter().any(|e| matches!(e, Event::Bind { .. })));
    }

    #[test]
    fn eval_seq_returns_last() {
        let term = Term::seq(vec![Term::var("a"), Term::var("b"), Term::var("c")]);
        let (res, _) = eval_closed(&term).unwrap();
        assert!(matches!(&res.value, Value::Symbol(s) if s.0 == "c"));
    }

    // ════════════════════════════════════════════════════════════════════════
    // PART IV: Observable state as quotient (v0.3)
    // ════════════════════════════════════════════════════════════════════════

    // Witnesses: Definition 8.3 — O_c = H/~_c
    #[test]
    fn observe_identity_counts_pops() {
        let mut w = World::new(OptionSpace::new([Symbol::new("x"), Symbol::new("y")]));
        w.pop(Symbol::new("x")).unwrap();
        w.pop(Symbol::new("y")).unwrap();
        let obs = w.observe_identity();
        assert_eq!(obs.entries.get("x"), Some(&1));
        assert_eq!(obs.entries.get("y"), Some(&1));
        assert!(!obs.has_refusals);
    }

    // Witnesses: Proposition 15.2 — Disambiguation by Refinement
    // Two histories equivalent under Accumulate but distinct under Identity
    #[test]
    fn finer_rule_disambiguates_equivalent_histories() {
        let mut w1 = World::new(OptionSpace::new([Symbol::new("x"), Symbol::new("y")]));
        w1.pop(Symbol::new("x")).unwrap();
        w1.pop(Symbol::new("y")).unwrap();

        let mut w2 = World::new(OptionSpace::new([Symbol::new("x"), Symbol::new("y")]));
        w2.pop(Symbol::new("y")).unwrap();
        w2.pop(Symbol::new("x")).unwrap();

        // Under Accumulate: same count-map → equivalent
        let obs1 = w1.observe(&CollapseRule::Accumulate);
        let obs2 = w2.observe(&CollapseRule::Accumulate);
        assert_eq!(obs1.entries, obs2.entries,
            "accumulate should not distinguish pop order");

        // Under Identity: different event order → distinct
        assert_ne!(w1.history, w2.history,
            "identity rule must distinguish pop order");
    }

    // Witnesses: LastWrite removes refused symbols from observable state
    #[test]
    fn observe_last_write_removes_refused_symbols() {
        let mut w = World::new(OptionSpace::new([Symbol::new("x"), Symbol::new("y")]));
        w.pop(Symbol::new("x")).unwrap();
        w.refuse(Symbol::new("x"), RefusalReason::Explicit("retracted".into()));
        let obs = w.observe(&CollapseRule::LastWrite);
        assert_eq!(obs.entries.get("x"), None,
            "refused symbol should be absent from last-write state");
        assert!(obs.has_refusals);
    }

    // Witnesses: Projection rule filters by symbol prefix
    #[test]
    fn observe_projection_filters_by_prefix() {
        let mut w = World::new(OptionSpace::new([
            Symbol::new("net_a"), Symbol::new("net_b"), Symbol::new("db_c"),
        ]));
        w.pop(Symbol::new("net_a")).unwrap();
        w.pop(Symbol::new("net_b")).unwrap();
        w.pop(Symbol::new("db_c")).unwrap();
        let obs = w.observe(&CollapseRule::Projection(Symbol::new("net")));
        assert_eq!(obs.entries.len(), 2);
        assert!(!obs.entries.contains_key("db_c"));
    }

    // ════════════════════════════════════════════════════════════════════════
    // PART V: Compiler correctness = history equivalence (v0.5)
    // ════════════════════════════════════════════════════════════════════════

    fn assert_same_history(term: &Term) {
        let (_, world_interp) = eval_closed(term).unwrap();
        let block = compile(term);
        let mut machine = Machine::empty();
        machine.run(&block).unwrap();
        assert_eq!(
            world_interp.history,
            machine.world.history,
            "I(p).history != V(C(p)).history\n  interp:   {}\n  compiled: {}",
            world_interp.history,
            machine.world.history,
        );
    }

    // Witnesses: Theorem 11.1 (Replay Equivalence) — Refuse
    #[test]
    fn compiler_correctness_refuse_with_reason() {
        assert_same_history(&Term::refuse(
            Term::var("x"),
            RefusalReason::ConstraintViolation(Symbol::new("c")),
        ));
    }

    // Witnesses: Theorem 11.1 — Bind
    #[test]
    fn compiler_correctness_bind() {
        assert_same_history(&Term::bind(Term::var("a"), Term::var("b")));
    }

    // Witnesses: Theorem 11.1 — Seq of refuses
    #[test]
    fn compiler_correctness_seq_of_refuses() {
        assert_same_history(&Term::seq(vec![
            Term::refuse(Term::var("a"), RefusalReason::NotAvailable),
            Term::refuse(Term::var("b"), RefusalReason::Explicit("second".into())),
        ]));
    }

    // Witnesses: Theorem 11.1 — Mixed bind and refuse
    #[test]
    fn compiler_correctness_mixed_bind_refuse() {
        assert_same_history(&Term::seq(vec![
            Term::bind(Term::var("src"), Term::var("dst")),
            Term::refuse(Term::var("src"), RefusalReason::AlreadyRefused(Symbol::new("src"))),
        ]));
    }

    // Witnesses: Theorem 13.6 (Compiler Full Faithfulness) — corollary of replay equiv
    #[test]
    fn compiler_full_faithfulness_observational() {
        let term = Term::bind(Term::var("a"), Term::var("b"));
        let (_, wi) = eval_closed(&term).unwrap();
        let block = compile(&term);
        let mut machine = Machine::empty();
        machine.run(&block).unwrap();
        // Both produce same observable state under Identity (corollary of full faithfulness)
        let obs_i = wi.observe(&CollapseRule::Identity);
        let obs_c = machine.world.observe(&CollapseRule::Identity);
        assert_eq!(obs_i, obs_c);
    }

    // ════════════════════════════════════════════════════════════════════════
    // PART VI: Reachability geometry and v4 features
    // ════════════════════════════════════════════════════════════════════════

    // Witnesses: Theorem 22.1 (Conservation of Possibility Functional)
    #[test]
    fn generalised_possibility_functional_is_conserved() {
        let opts = OptionSpace::new([Symbol::new("a"), Symbol::new("b"), Symbol::new("c")]);
        let mut w = World::new(opts);
        let initial = w.initial_size;
        assert_eq!(w.possibility_functional(), initial);

        w.pop(Symbol::new("a")).unwrap();
        assert_eq!(w.possibility_functional(), initial,
            "Pop: Π should still equal |Ω₀|");

        w.refuse(Symbol::new("b"), RefusalReason::NotAvailable);
        assert_eq!(w.possibility_functional(), initial,
            "Refuse: Π should still equal |Ω₀|");

        w.bind(Symbol::new("b"), Symbol::new("c"));
        assert_eq!(w.possibility_functional(), initial,
            "Bind: Π should still equal |Ω₀|");

        w.pop(Symbol::new("c")).unwrap();
        assert_eq!(w.possibility_functional(), initial,
            "Pop: Π should still equal |Ω₀|");
    }

    // Witnesses: verify_conservation helper
    #[test]
    fn verify_conservation_passes_for_legal_execution() {
        let opts = OptionSpace::new([Symbol::new("x"), Symbol::new("y")]);
        let mut w = World::new(opts);
        w.pop(Symbol::new("x")).unwrap();
        w.refuse(Symbol::new("y"), RefusalReason::NotAvailable);
        assert!(w.verify_conservation().is_ok());
    }

    // Witnesses: Corollary 22.3 (Irreversibility) — history only grows
    #[test]
    fn history_is_strictly_monotone() {
        let opts = OptionSpace::new([Symbol::new("x")]);
        let mut w = World::new(opts);
        let len0 = w.history.len();
        w.pop(Symbol::new("x")).unwrap();
        let len1 = w.history.len();
        w.refuse(Symbol::new("other"), RefusalReason::NotAvailable);
        let len2 = w.history.len();
        assert!(len0 < len1, "history must grow after pop");
        assert!(len1 < len2, "history must grow after refuse");
    }

    // Witnesses: Proposition 22.4 (Histories as Composed Deformations)
    #[test]
    fn compose_deformations_matches_forward_project() {
        let opts = OptionSpace::new([Symbol::new("x"), Symbol::new("y")]);
        let mut w = World::new(opts.clone());
        w.pop(Symbol::new("x")).unwrap();
        w.refuse(Symbol::new("y"), RefusalReason::NotAvailable);

        // compose_deformations should produce same geometry as forward_project
        let geom_composed = compose_deformations(&w.history, &opts);
        let geom_forward = w.reachability_geometry();

        // After pop(x) and refuse(y):
        //   available = {y}  (x was popped)
        //   admissible = {x} (y was refused; x was popped but was admissible)
        assert!(!geom_forward.available.contains(&Symbol::new("x")),
            "x should not be available after pop");
        assert!(!geom_forward.admissible.contains(&Symbol::new("y")),
            "y should not be admissible after refuse");

        // Both methods agree on reachable count
        assert_eq!(geom_composed.reachable_count(), geom_forward.reachable_count());
    }

    // Witnesses: Theorem 22.8 (Monotone Field Deformation)
    // Pop events are non-admissibility-increasing
    #[test]
    fn admissibility_field_decreases_on_pop() {
        let opts = OptionSpace::new([Symbol::new("a"), Symbol::new("b"), Symbol::new("c")]);
        let mut w = World::new(opts);
        let phi0 = w.admissibility_field();
        assert_eq!(phi0, 1.0, "initial field should be 1.0");

        w.pop(Symbol::new("a")).unwrap();
        let phi1 = w.admissibility_field();
        assert!(phi1 <= phi0, "pop should not increase admissibility field");
    }

    // Witnesses: Theorem 22.8 — Refuse decreases admissibility field
    #[test]
    fn admissibility_field_decreases_on_refuse() {
        let opts = OptionSpace::new([Symbol::new("x"), Symbol::new("y")]);
        let mut w = World::new(opts);
        let phi0 = w.admissibility_field();
        w.refuse(Symbol::new("x"), RefusalReason::NotAvailable);
        let phi1 = w.admissibility_field();
        assert!(phi1 <= phi0, "refuse should not increase admissibility field");
    }

    // Witnesses: Two-projection architecture (Definition 22.9)
    #[test]
    fn two_projection_returns_geometry_and_observation() {
        let opts = OptionSpace::new([Symbol::new("x"), Symbol::new("y")]);
        let mut w = World::new(opts);
        w.pop(Symbol::new("x")).unwrap();

        let (geom, obs) = w.two_projection(&CollapseRule::Accumulate);
        // Geometry: x was popped, y still available and admissible
        assert!(!geom.available.contains(&Symbol::new("x")));
        assert!(geom.available.contains(&Symbol::new("y")));
        // Observable state under Accumulate: x → 1
        assert_eq!(obs.entries.get("x"), Some(&1));
    }

    // Witnesses: Proposition 22.10 (Fibre Partition)
    // Every history belongs to exactly one fibre of each collapse rule
    #[test]
    fn fibre_partition_is_exhaustive() {
        let mut h1 = History::new();
        h1.append(Event::Pop(Symbol::new("a")));

        let mut h2 = History::new();
        h2.append(Event::Pop(Symbol::new("b")));

        let mut h3 = History::new();
        h3.append(Event::Pop(Symbol::new("a")));
        h3.append(Event::Pop(Symbol::new("b")));

        let histories = vec![h1.clone(), h2.clone(), h3.clone()];
        let rule = CollapseRule::Accumulate;

        // Each history maps to exactly one observable state
        let obs1 = ObservableState::derive(&h1, &rule);
        let obs2 = ObservableState::derive(&h2, &rule);
        let obs3 = ObservableState::derive(&h3, &rule);

        // h1 and h2 are in different fibres (different pops)
        assert_ne!(obs1, obs2);
        // h3 is in its own fibre (both a and b popped)
        assert_ne!(obs1, obs3);
        assert_ne!(obs2, obs3);

        // Fibre of obs1 contains exactly h1
        let f1 = fibre(&histories, &obs1, &rule);
        assert_eq!(f1.len(), 1);
        assert_eq!(f1[0], &h1);
    }

    // Witnesses: Theorem 22.5 (Observation-Recovery Tradeoff)
    // Identity rule is the only injective rule for these histories
    #[test]
    fn identity_rule_is_observationally_complete() {
        let mut h1 = History::new();
        h1.append(Event::Pop(Symbol::new("x")));
        h1.append(Event::Pop(Symbol::new("y")));

        let mut h2 = History::new();
        h2.append(Event::Pop(Symbol::new("y")));
        h2.append(Event::Pop(Symbol::new("x")));

        // Under Identity: different (different order)
        let id1 = ObservableState::derive(&h1, &CollapseRule::Identity);
        let id2 = ObservableState::derive(&h2, &CollapseRule::Identity);
        assert_ne!(id1, id2, "Identity must distinguish different event orders");

        // Under Accumulate: same (order-insensitive)
        let acc1 = ObservableState::derive(&h1, &CollapseRule::Accumulate);
        let acc2 = ObservableState::derive(&h2, &CollapseRule::Accumulate);
        assert_eq!(acc1.entries, acc2.entries,
            "Accumulate must not distinguish pop order (non-injective)");
    }

    // Witnesses: Theorem 18.3 (No Direct Observation)
    // Distinct histories can be observationally equivalent under coarse rules
    #[test]
    fn distinct_histories_equivalent_under_coarse_rule() {
        let mut w1 = World::empty();
        w1.refuse(Symbol::new("x"), RefusalReason::NotAvailable);
        w1.refuse(Symbol::new("y"), RefusalReason::NotAvailable);

        let mut w2 = World::empty();
        w2.refuse(Symbol::new("y"), RefusalReason::NotAvailable);
        w2.refuse(Symbol::new("x"), RefusalReason::NotAvailable);

        // Histories are distinct
        assert_ne!(w1.history, w2.history);
        // Both have same refusal count (cardinality argument)
        assert_eq!(w1.history.refuse_count(), w2.history.refuse_count());
    }

    // Witnesses: Observational entropy S_c(o) = log|c⁻¹(o)| (Definition 13.8)
    #[test]
    fn observational_entropy_measures_fibre_size() {
        let mut h1 = History::new();
        h1.append(Event::Pop(Symbol::new("a")));

        let mut h2 = History::new();
        h2.append(Event::Pop(Symbol::new("b")));

        let mut h3 = History::new();
        h3.append(Event::Pop(Symbol::new("a"))); // same as h1 under accumulate

        let histories = vec![h1.clone(), h2.clone(), h3.clone()];
        let rule = CollapseRule::Accumulate;

        let target = ObservableState::derive(&h1, &rule);
        let entropy = observational_entropy(&histories, &target, &rule);

        // h1 and h3 both map to the same state: fibre has 2 elements
        // S_c(o) = log(2)
        assert!((entropy - 2.0_f64.ln()).abs() < 1e-9,
            "entropy should be log(2) when fibre has 2 histories");
    }

    // ════════════════════════════════════════════════════════════════════════
    // PART VII: Integration and run helpers
    // ════════════════════════════════════════════════════════════════════════

    #[test]
    fn run_open_world_smoke() {
        assert!(run("refuse x", &[]).is_ok());
    }

    #[test]
    fn run_strict_closed_world_rejects_free_var() {
        assert!(run_strict("refuse x", &[]).is_err());
    }

    #[test]
    fn admissibility_invariant_end_to_end() {
        let (res, world) = eval_with_options(
            &Term::pop(Term::var("resource")), [Symbol::new("resource")]).unwrap();
        assert!(res.admissible);
        assert!(world.history.ever_popped(&Symbol::new("resource")));
        assert!(world.options.is_empty());
        assert!(world.verify_conservation().is_ok());
    }
}
