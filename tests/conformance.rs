// conformance.rs
//
// Phase 1–4 conformance tests.
//
// These tests verify the structural invariants of the relational loop
// independently of any hardware and independently of Kernel V8's internals.
// They run against MockDevice with synthetic observation sequences.
//
// Invariants tested:
//   1. DeviceInterface trait — MockDevice satisfies the contract.
//   2. MDeclaration validation — inadmissible declarations are rejected.
//   3. Operator path admissibility — A→B→R without grounded locus is rejected.
//   4. Provenance isolation — MockFixture observations do not update prior_state.
//   5. Projection bounds — commands stay within declared hardware limits.
//   6. No synthetic initialisation — prior_state starts as None.

use std::sync::Arc;
use abr_datafeel_dot::{
    actuator_projection::ActuatorProjection,
    declaration::{Locus, LocusSource, MDeclaration, OperatorPath, Relation, BoundaryConditions},
    device_interface::{DeviceInterface, MockDevice},
    observation::ObsProvenance,
    relational_loop::RelationalLoop,
};

fn minimal_declaration() -> MDeclaration {
    MDeclaration {
        region: "Test region".to_string(),
        boundary_conditions: BoundaryConditions {
            temperature_range_c: [-5.0, 45.0],
            vibration_hz: [150.0, 200.0],
            optical_nm: [466.0, 626.0],
        },
        loci: vec![
            Locus { id: "L-thermal".to_string(), observable: "thermistor_c".to_string(), source: LocusSource::Device, basis: None },
            Locus { id: "L-haptic".to_string(),  observable: "haptic_amplitude".to_string(), source: LocusSource::Actuator, basis: None },
        ],
        relations: vec![
            Relation {
                id: "R-01".to_string(),
                from: "L-thermal".to_string(),
                to: "L-haptic".to_string(),
                operator_path: OperatorPath::PrimaryDeltaSigma,
            },
        ],
        provenance: "Test — conformance.rs".to_string(),
    }
}

// ---------------------------------------------------------------------------
// 1. MockDevice satisfies DeviceInterface
// ---------------------------------------------------------------------------

#[test]
fn mock_device_reads_thermistor_sequence() {
    let device = MockDevice::new(vec![36.5, 37.0, 37.5]);
    assert!((device.read_thermistor().unwrap() - 36.5).abs() < 0.001);
    assert!((device.read_thermistor().unwrap() - 37.0).abs() < 0.001);
    assert!((device.read_thermistor().unwrap() - 37.5).abs() < 0.001);
    // Sequence exhausted — last value repeats.
    assert!((device.read_thermistor().unwrap() - 37.5).abs() < 0.001);
}

#[test]
fn mock_device_records_commands() {
    use abr_datafeel_dot::device_interface::{ThermalCommand, HapticCommand, OpticalCommand, MockCommand};
    let device = MockDevice::new(vec![36.5]);
    device.set_thermal(ThermalCommand::new(0.5).unwrap()).unwrap();
    device.set_haptic(HapticCommand::new(0.3, 100).unwrap()).unwrap();
    device.set_optical(OpticalCommand { r: 100, g: 50, b: 20, timing_ms: 50 }).unwrap();
    let log = device.command_log.lock().unwrap();
    assert_eq!(log.len(), 3);
    assert!(matches!(log[0], MockCommand::Thermal(_)));
    assert!(matches!(log[1], MockCommand::Haptic(_)));
    assert!(matches!(log[2], MockCommand::Optical(_)));
}

// ---------------------------------------------------------------------------
// 2. MDeclaration validation
// ---------------------------------------------------------------------------

#[test]
fn valid_declaration_loads_cleanly() {
    assert!(minimal_declaration().validate().is_ok());
}

#[test]
fn declaration_rejects_unknown_from_locus() {
    let mut decl = minimal_declaration();
    decl.relations[0].from = "L-nonexistent".to_string();
    assert!(decl.validate().is_err());
}

#[test]
fn declaration_rejects_unknown_to_locus() {
    let mut decl = minimal_declaration();
    decl.relations[0].to = "L-nonexistent".to_string();
    assert!(decl.validate().is_err());
}

#[test]
fn declaration_rejects_duplicate_locus_ids() {
    let mut decl = minimal_declaration();
    decl.loci.push(Locus {
        id: "L-thermal".to_string(),
        observable: "duplicate".to_string(),
        source: LocusSource::Device,
        basis: None,
    });
    assert!(decl.validate().is_err());
}

// ---------------------------------------------------------------------------
// 3. A→B→R admissibility
// ---------------------------------------------------------------------------

#[test]
fn abr_path_rejected_without_interior_locus() {
    let mut decl = minimal_declaration();
    decl.relations[0].operator_path = OperatorPath::ABR {
        interior_locus_id: "L-nonexistent".to_string(),
    };
    let err = decl.validate().unwrap_err();
    assert!(err.to_string().contains("not declared"));
}

#[test]
fn abr_path_rejected_with_actuator_interior_locus() {
    let mut decl = minimal_declaration();
    // L-haptic exists but is Actuator-sourced — not admissible as interior locus.
    decl.relations[0].operator_path = OperatorPath::ABR {
        interior_locus_id: "L-haptic".to_string(),
    };
    let err = decl.validate().unwrap_err();
    assert!(err.to_string().contains("not Device-sourced or Host-sourced"));
}

#[test]
fn abr_path_accepted_with_device_interior_locus() {
    let mut decl = minimal_declaration();
    // L-thermal is Device-sourced — admissible as interior locus.
    decl.relations[0].operator_path = OperatorPath::ABR {
        interior_locus_id: "L-thermal".to_string(),
    };
    assert!(decl.validate().is_ok());
}

// ---------------------------------------------------------------------------
// 4. Provenance isolation
// ---------------------------------------------------------------------------

#[test]
fn mock_fixture_observations_do_not_update_prior_state() {
    let decl = minimal_declaration();
    let device = MockDevice::default_sequence();
    let projection = ActuatorProjection::default();
    let mut loop_ = RelationalLoop::new(decl, projection, Arc::clone(&device) as Arc<_>);

    // Run several MockFixture steps.
    for _ in 0..5 {
        loop_.step(ObsProvenance::MockFixture, &Default::default()).unwrap();
    }

    // prior_state must still be None — MockFixture does not accumulate state.
    assert!(loop_.prior_state().is_none(),
        "prior_state must not accumulate from MockFixture observations");
}

#[test]
fn physical_observations_do_update_prior_state() {
    let decl = minimal_declaration();
    let device = MockDevice::default_sequence();
    let projection = ActuatorProjection::default();
    let mut loop_ = RelationalLoop::new(decl, projection, Arc::clone(&device) as Arc<_>);

    loop_.step(ObsProvenance::Physical, &Default::default()).unwrap();

    assert!(loop_.prior_state().is_some(),
        "prior_state must be set after a Physical observation");
}

// ---------------------------------------------------------------------------
// 5. Projection bounds
// ---------------------------------------------------------------------------

#[test]
fn projection_commands_stay_within_hardware_limits() {
    use abr_datafeel_dot::actuator_projection::HardwareLimits;
    use abr_datafeel_dot::relational_loop::RelationalState;
    use abr_datafeel_dot::device_interface::ActuatorState;
    use abr_datafeel_dot::observation::Observation;

    let limits = HardwareLimits::default();
    let projection = ActuatorProjection::new(HardwareLimits::default());

    // Construct a state with an extreme delta to test clamping.
    let obs = Observation::mock_fixture(45.0, ActuatorState::default());
    let mut locus_values = std::collections::HashMap::new();
    locus_values.insert("L-thermal".to_string(), 20.0_f32);
    locus_values.insert("L-haptic".to_string(), 20.0_f32);
    let state = RelationalState {
        source_observation: obs,
        delta_thermistor: Some(20.0),  // large disturbance
        sigma_thermistor: 20.0,
        locus_values,
    };

    let decl = minimal_declaration();
    let cmds = projection.project(&state, &decl).unwrap();
    assert!(cmds.thermal.power.abs() <= limits.thermal_power_max,
        "thermal power exceeded hardware limit");
    assert!(cmds.haptic.amplitude <= limits.haptic_amplitude_max,
        "haptic amplitude exceeded hardware limit");
    assert!(cmds.haptic.duration_ms <= limits.haptic_duration_max_ms,
        "haptic duration exceeded hardware limit");
}

// ---------------------------------------------------------------------------
// 6. No synthetic initialisation
// ---------------------------------------------------------------------------

#[test]
fn prior_state_starts_as_none() {
    let decl = minimal_declaration();
    let device = MockDevice::default_sequence();
    let projection = ActuatorProjection::default();
    let loop_ = RelationalLoop::new(decl, projection, Arc::clone(&device) as Arc<_>);

    assert!(loop_.prior_state().is_none(),
        "prior_state must be None before any observation");
}
