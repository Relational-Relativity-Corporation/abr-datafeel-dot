// datafeel_demo.rs
//
// Main binary — runs the relational multimodal loop against MockDevice
// and prints the step trace to stdout.
//
// Phase 1–4: uses MockDevice and ObsProvenance::MockFixture.
// Phase 5: swap in a real DataFeel implementor and ObsProvenance::Physical.

use std::sync::Arc;
use abr_datafeel_dot::{
    actuator_projection::ActuatorProjection,
    declaration::MDeclaration,
    device_interface::MockDevice,
    observation::ObsProvenance,
    relational_loop::RelationalLoop,
};

fn main() {
    // Load M declaration.
    let decl = MDeclaration::load("declarations/m_declaration.json")
        .expect("failed to load m_declaration.json");

    println!("M Declaration loaded: {}", decl.region);
    println!("Relations declared: {}", decl.relations.len());
    println!("{}", "-".repeat(60));

    // Phase 1–4: MockDevice with a thermal disturbance sequence.
    let device = MockDevice::default_sequence();
    let projection = ActuatorProjection::default();
    let mut loop_ = RelationalLoop::new(decl, projection, Arc::clone(&device) as Arc<_>);

    // Run 12 steps (length of the default mock sequence).
    for step_num in 1..=12 {
        // MockFixture provenance — this is a software conformance run, not
        // physical hardware. Change to ObsProvenance::Physical for Phase 5.
        match loop_.step(ObsProvenance::MockFixture) {
            Ok(output) => {
                let obs  = &output.observation;
                let st   = &output.state;
                let cmds = &output.commands;

                println!(
                    "Step {:>2} | thermistor: {:>5.2}°C | Δ: {:>+6.2} | Σ: {:>+6.2}",
                    step_num,
                    obs.thermistor_c,
                    st.delta_thermistor.unwrap_or(0.0),
                    st.sigma_thermistor,
                );
                println!(
                    "         thermal: {:>+5.2} | haptic: amp={:.2} dur={}ms | rgb=({},{},{})",
                    cmds.thermal.power,
                    cmds.haptic.amplitude,
                    cmds.haptic.duration_ms,
                    cmds.optical.r,
                    cmds.optical.g,
                    cmds.optical.b,
                );
            }
            Err(e) => {
                eprintln!("Step {} error: {}", step_num, e);
                break;
            }
        }
    }

    println!("{}", "-".repeat(60));
    println!("Commands dispatched: {}", device.command_log.lock().unwrap().len());
    println!("Prior state held: {}", loop_.prior_state().is_some());
    println!();
    println!("Note: prior_state is None throughout a MockFixture run.");
    println!("Physical observations accumulate state; mock fixtures do not.");
}
