// replay.rs
//
// Replay tests — verify that equivalent inputs produce equivalent operator
// traces across runs. Reproducibility is a required verification criterion.
//
// A replay test loads a recorded observation sequence from data/observations/,
// runs it through the loop, and checks that the operator trace matches the
// recorded expected output. This catches any nondeterminism in the loop or
// projection logic.
//
// Phase 1–4: the replay infrastructure is established here. Actual recorded
// sequences are added once physical hardware runs are available (Phase 5+).

use std::sync::Arc;
use abr_datafeel_dot::{
    actuator_projection::ActuatorProjection,
    declaration::MDeclaration,
    device_interface::MockDevice,
    observation::ObsProvenance,
    relational_loop::RelationalLoop,
};

/// Helper: run a thermistor sequence through the loop and return
/// (r_persistence, sigma) pairs for each step.
///
/// Uses m_declaration_active.json (activity profile) with time_of_day_phase=0.5
/// (noon), so L-activity = 1.0 and gate = 1.0 throughout. This makes sigma
/// equal to r_persistence directly, giving clean numeric verification.
fn run_sequence(temps: Vec<f32>) -> Vec<(f32, f32)> {
    let decl = MDeclaration::load("declarations/m_declaration_active.json")
        .expect("m_declaration_active.json must exist for replay tests");
    let device = MockDevice::new(temps.clone());
    let projection = ActuatorProjection::default();
    let mut loop_ = RelationalLoop::new(decl, projection, Arc::clone(&device) as Arc<_>);
    // time_of_day_phase = 0.5 → noon → L-activity = 1.0 → gate = 1.0.
    let mut host_obs = std::collections::HashMap::new();
    host_obs.insert("time_of_day_phase".to_string(), 0.5_f32);
    let mut results = Vec::new();
    for _ in 0..temps.len() {
        let output = loop_.step(ObsProvenance::MockFixture, &host_obs).unwrap();
        results.push((
            output.state.delta_thermistor.unwrap_or(0.0),
            output.state.sigma_thermistor,
        ));
    }
    results
}

#[test]
fn same_sequence_produces_same_trace() {
    let temps = vec![36.5, 36.9, 37.3, 37.8, 38.2, 37.9, 37.5, 37.1];
    let trace_a = run_sequence(temps.clone());
    let trace_b = run_sequence(temps);
    for (a, b) in trace_a.iter().zip(trace_b.iter()) {
        assert!((a.0 - b.0).abs() < 0.0001, "delta mismatch: {} vs {}", a.0, b.0);
        assert!((a.1 - b.1).abs() < 0.0001, "sigma mismatch: {} vs {}", a.1, b.1);
    }
}

#[test]
fn flat_sequence_produces_zero_delta_and_sigma() {
    let temps = vec![37.0; 6];
    let trace = run_sequence(temps);
    // First step has no prior — delta is 0.0 by convention (None → 0.0).
    // All subsequent steps should also have zero delta on a flat sequence.
    for (delta, sigma) in trace {
        assert!(delta.abs() < 0.0001, "expected zero delta, got {delta}");
        assert!(sigma.abs() < 0.0001, "expected zero sigma, got {sigma}");
    }
}

#[test]
fn monotone_rising_sequence_produces_positive_sigma() {
    let temps = vec![36.0, 36.5, 37.0, 37.5, 38.0];
    let trace = run_sequence(temps);
    let final_sigma = trace.last().unwrap().1;
    assert!(final_sigma > 0.0,
        "expected positive sigma for rising sequence, got {final_sigma}");
}

#[test]
fn monotone_falling_sequence_produces_negative_sigma() {
    let temps = vec![38.0, 37.5, 37.0, 36.5, 36.0];
    let trace = run_sequence(temps);
    let final_sigma = trace.last().unwrap().1;
    assert!(final_sigma < 0.0,
        "expected negative sigma for falling sequence, got {final_sigma}");
}
