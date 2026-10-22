// datafeel_demo.rs
//
// DataFeel Dot — V0.2 comparative profile demonstration.
//
// Runs the same 12-step thermal disturbance sequence through both the
// activity-associated and rest-associated profiles and prints a side-by-side
// trace of their calculated outputs.
//
// No hardware required. All steps use MockDevice with MockFixture provenance.
// Host observables (time_of_day_phase) are set to fixed representative values:
//   Activity profile: 0.60  (mid-afternoon, ~14:24 local)
//   Rest profile:     0.08  (pre-dawn, ~01:55 local)
//
// Scientific discipline: outputs shown are computed device commands, not
// measured physiological effects. No cross-modal efficacy is claimed.

use std::collections::HashMap;
use std::sync::Arc;

use abr_datafeel_dot::{
    actuator_projection::ActuatorProjection,
    declaration::MDeclaration,
    device_interface::MockDevice,
    observation::ObsProvenance,
    relational_loop::{HostObservables, RelationalLoop},
};

// ---------------------------------------------------------------------------
// Thermal disturbance sequence
// ---------------------------------------------------------------------------
// Simulates a brief external thermal contact followed by return to baseline.
// Temperatures in °C.

fn disturbance_sequence() -> Vec<f32> {
    vec![
        36.5, 36.5, 36.7, 37.0, 37.4, 37.8,   // baseline → peak disturbance
        37.5, 37.1, 36.9, 36.7, 36.6, 36.5,   // recovery
    ]
}

// ---------------------------------------------------------------------------
// Run one profile
// ---------------------------------------------------------------------------

struct StepTrace {
    step:            usize,
    thermistor_c:    f32,
    delta:           f32,
    sigma:           f32,
    thermal_power:   f32,
    haptic_amplitude: f32,
    haptic_ms:       u32,
    r:               u8,
    g:               u8,
    b:               u8,
}

fn run_profile(
    decl: MDeclaration,
    sequence: Vec<f32>,
    host_obs: HostObservables,
) -> Vec<StepTrace> {
    let device = MockDevice::new(sequence.clone());
    let projection = ActuatorProjection::default();
    let mut loop_ = RelationalLoop::new(decl, projection, Arc::clone(&device) as Arc<_>);

    let mut traces = Vec::new();
    for (i, _) in sequence.iter().enumerate() {
        let out = loop_.step(ObsProvenance::MockFixture, &host_obs)
            .expect("step failed");

        traces.push(StepTrace {
            step:             i + 1,
            thermistor_c:     out.observation.thermistor_c,
            delta:            out.state.delta_thermistor.unwrap_or(0.0),
            sigma:            out.state.sigma_thermistor,
            thermal_power:    out.commands.thermal.power,
            haptic_amplitude: out.commands.haptic.amplitude,
            haptic_ms:        out.commands.haptic.duration_ms,
            r:                out.commands.optical.r,
            g:                out.commands.optical.g,
            b:                out.commands.optical.b,
        });
    }
    traces
}

// ---------------------------------------------------------------------------
// Print comparative trace
// ---------------------------------------------------------------------------

fn print_header() {
    println!();
    println!("DataFeel Dot — V0.2 Comparative Profile Trace");
    println!("══════════════════════════════════════════════════════════════════════════════════════════════");
    println!("Thermal sequence: external disturbance → recovery (12 steps, MockFixture, no hardware)");
    println!("Host observables: Activity phase = 0.60 (14:24 local) | Rest phase = 0.08 (01:55 local)");
    println!("══════════════════════════════════════════════════════════════════════════════════════════════");
    println!();
}

fn print_profile_trace(label: &str, traces: &[StepTrace]) {
    println!("── {} ─────────────────────────────────────────────────────────────────", label);
    println!("{:>4}  {:>8}  {:>6}  {:>7}  {:>8}  {:>6}  {:>7}  {:>5}",
        "step", "temp(°C)", "Δ(°C)", "Σ(°C)", "therm_pwr", "hap_amp", "hap_ms", "RGB");
    println!("{:─>4}  {:─>8}  {:─>6}  {:─>7}  {:─>9}  {:─>7}  {:─>7}  {:─>17}",
        "", "", "", "", "", "", "", "");

    for t in traces {
        println!("{:>4}  {:>8.2}  {:>+6.2}  {:>+7.2}  {:>+9.3}  {:>7.3}  {:>7}  ({:>3},{:>3},{:>3})",
            t.step,
            t.thermistor_c,
            t.delta,
            t.sigma,
            t.thermal_power,
            t.haptic_amplitude,
            t.haptic_ms,
            t.r, t.g, t.b,
        );
    }
    println!();
}

fn print_comparison(active: &[StepTrace], rest: &[StepTrace]) {
    println!("── Cross-profile comparison: optical channel dominance ──────────────────────────────────────");
    println!("{:>4}  {:>8}  {:>12}  {:>12}  {:>14}",
        "step", "temp(°C)", "active R,G,B", "rest R,G,B", "dominant channel");
    println!("{:─>4}  {:─>8}  {:─>12}  {:─>12}  {:─>14}", "", "", "", "", "");

    for (a, r) in active.iter().zip(rest.iter()) {
        let active_dom = channel_label(a.r, a.g, a.b);
        let rest_dom   = channel_label(r.r, r.g, r.b);
        let note = if active_dom != rest_dom {
            format!("active={active_dom} / rest={rest_dom}")
        } else {
            format!("both={active_dom}")
        };
        println!("{:>4}  {:>8.2}  {:>12}  {:>12}  {}",
            a.step,
            a.thermistor_c,
            format!("({},{},{})", a.r, a.g, a.b),
            format!("({},{},{})", r.r, r.g, r.b),
            note,
        );
    }
    println!();
}

fn channel_label(r: u8, g: u8, b: u8) -> &'static str {
    if r == 0 && g == 0 && b == 0 {
        "none"
    } else if r >= g && r >= b {
        "red"
    } else if b >= r && b >= g {
        "blue"
    } else {
        "green"
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let seq = disturbance_sequence();

    // Load declarations.
    let active_decl = MDeclaration::load("declarations/m_declaration_active.json")
        .expect("m_declaration_active.json not found");
    let rest_decl = MDeclaration::load("declarations/m_declaration_rest.json")
        .expect("m_declaration_rest.json not found");

    // Host observables: fixed representative values for the demo.
    let mut active_host: HostObservables = HashMap::new();
    active_host.insert("time_of_day_phase".to_string(), 0.60);  // ~14:24 local

    let mut rest_host: HostObservables = HashMap::new();
    rest_host.insert("time_of_day_phase".to_string(), 0.08);    // ~01:55 local

    // Run both profiles.
    let active_trace = run_profile(active_decl, seq.clone(), active_host);
    let rest_trace   = run_profile(rest_decl,   seq.clone(), rest_host);

    // Print.
    print_header();
    print_profile_trace("Activity-associated profile (blue-enriched, circadian active)", &active_trace);
    print_profile_trace("Rest-associated profile (dim red, circadian rest)", &rest_trace);
    print_comparison(&active_trace, &rest_trace);

    println!("══════════════════════════════════════════════════════════════════════════════════════════════");
    println!("All outputs are computed device commands (MockFixture). No physiological effects are claimed.");
    println!("Scientific basis: docs/verification_record.md");
    println!();
}
