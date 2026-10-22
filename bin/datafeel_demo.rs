// datafeel_demo.rs
//
// DataFeel Dot — V0.4 multimodal co-activity sweep.
//
// EVALUATOR NOTE: The relational loop uses a provisional simulation of the
// PrimaryΔΣ and ABR operator paths. It is NOT a Kernel V8 implementation.
// Output reflects the declared relational topology and Origin schedules;
// it is not a validated kernel result. See relational_loop.rs for detail.
//
// Sweeps the circadian phase (time_of_day_phase) from 0.0 → 0.95 in 20 steps
// (Δ = 0.05 per step, representing ~72 minutes of circadian time per step).
// At each phase point, the same thermal disturbance sequence is run through
// both the activity-associated and rest-associated profiles using the ABR
// operator gated by their respective Origin-declared profile loci.
//
// V0.4 changes from V0.3:
//   - Activity and rest profiles now use Origin-declared loci (L-activity,
//     L-rest) with explicit schedule boundaries, replacing the raw
//     time_of_day_phase gate. This corrects the midnight discontinuity
//     in V0.3 where phase=0.95 and phase=0.0 produced opposite gate values.
//   - Both profiles use gate_inverted=false; the Origin schedule itself
//     expresses the preferred phase window.
//   - Missing host observables now return an error (no silent fallback).
//   - ResonanceFlag::Resonant renamed to CoActive to clarify that this
//     refers to simultaneous command thresholds, not physiological resonance.
//
// The sweep reveals the AREA OF MULTIMODAL CO-ACTIVITY — where haptic AND
// optical outputs are simultaneously above threshold. Because both relations
// in each profile share the same Origin interior locus (L-activity or L-rest),
// the co-active window is determined by the declared schedule, not by the
// thermal input alone.
//
// Activity profile: sigma × L-activity   [peak: 06:00–18:00, rise: 05:00, fall: 20:00]
// Rest profile:     sigma × L-rest        [peak: 21:00–03:00, rise: 19:00, fall: 07:00]
//
// Scientific discipline: outputs are computed device commands (MockFixture).
// No hardware is connected. No physiological effects are claimed.
// These are experimental sensory configurations, not therapeutic prescriptions.
// Results describe where the relational topology produces joint modality output,
// not measured biological coupling.

use std::collections::HashMap;
use std::sync::Arc;

use abr_datafeel_dot::{
    actuator_projection::{ActuatorProjection, ResonanceFlag},
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
// Same sequence used at every phase point — the thermal history is held constant
// so that only the circadian gate (phase value) varies between sweep steps.

fn disturbance_sequence() -> Vec<f32> {
    vec![
        36.5, 36.5, 36.7, 37.0, 37.4, 37.8,   // baseline → peak disturbance
        37.5, 37.1, 36.9, 36.7, 36.6, 36.5,   // recovery
    ]
}

// ---------------------------------------------------------------------------
// Per-phase summary: peak commanded values across the disturbance sequence
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct PhaseSummary {
    phase: f32,
    // Activity profile peaks
    act_haptic_amp:   f32,
    act_haptic_ms:    u32,
    act_optical_r:    u8,
    act_optical_g:    u8,
    act_optical_b:    u8,
    act_resonance:    ResonanceFlag,
    // Rest profile peaks
    rest_haptic_amp:  f32,
    rest_haptic_ms:   u32,
    rest_optical_r:   u8,
    rest_optical_g:   u8,
    rest_optical_b:   u8,
    rest_resonance:   ResonanceFlag,
}

// ---------------------------------------------------------------------------
// Run one profile at a fixed phase, return the peak-output summary
// ---------------------------------------------------------------------------

fn run_phase(
    decl: MDeclaration,
    sequence: &[f32],
    phase: f32,
) -> (f32, u32, u8, u8, u8, ResonanceFlag) {
    let mut host_obs: HostObservables = HashMap::new();
    host_obs.insert("time_of_day_phase".to_string(), phase);

    let device = MockDevice::new(sequence.to_vec());
    let projection = ActuatorProjection::default();
    let mut loop_ = RelationalLoop::new(decl, projection, Arc::clone(&device) as Arc<_>);

    let mut peak_hap_amp:  f32 = 0.0;
    let mut peak_hap_ms:   u32 = 0;
    let mut peak_r:        u8  = 0;
    let mut peak_g:        u8  = 0;
    let mut peak_b:        u8  = 0;
    let mut best_resonance = ResonanceFlag::Quiescent;

    for _ in sequence.iter() {
        let out = loop_.step(ObsProvenance::MockFixture, &host_obs)
            .expect("step failed");

        let cmds = &out.commands;
        let flag = ActuatorProjection::resonance_flag(cmds);

        peak_hap_amp = peak_hap_amp.max(cmds.haptic.amplitude);
        peak_hap_ms  = peak_hap_ms.max(cmds.haptic.duration_ms);
        peak_r       = peak_r.max(cmds.optical.r);
        peak_g       = peak_g.max(cmds.optical.g);
        peak_b       = peak_b.max(cmds.optical.b);

        // Escalate co-activity flag.
        best_resonance = match (&best_resonance, &flag) {
            (_, ResonanceFlag::CoActive)   => ResonanceFlag::CoActive,
            (ResonanceFlag::Quiescent, _)  => flag.clone(),
            _                              => best_resonance,
        };
    }

    (peak_hap_amp, peak_hap_ms, peak_r, peak_g, peak_b, best_resonance)
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let seq = disturbance_sequence();

    // Load declarations.
    let active_decl_template = MDeclaration::load("declarations/m_declaration_active.json")
        .expect("m_declaration_active.json not found");
    let rest_decl_template = MDeclaration::load("declarations/m_declaration_rest.json")
        .expect("m_declaration_rest.json not found");

    println!();
    println!("DataFeel Dot — V0.4 Multimodal Co-Activity Sweep");
    println!("══════════════════════════════════════════════════════════════════════════════════════════════════");
    println!("Circadian phase sweep: 0.00 → 0.95, Δ = 0.05 (20 steps ≈ 72 min each)");
    println!("Thermal sequence: 12-step external disturbance → recovery, MockFixture, no hardware");
    println!("Operator: ABR — haptic (R-01) and optical (R-02) gated by Origin-declared profile locus");
    println!("Activity: sigma × L-activity  [schedule: 05:00–06:00 rise | 06:00–18:00 peak | 18:00–20:00 fall]");
    println!("Rest:     sigma × L-rest       [schedule: 19:00–21:00 rise | 21:00–03:00 peak | 03:00–07:00 fall]");
    println!("Evaluator: provisional simulation — not a validated Kernel V8 implementation");
    println!("══════════════════════════════════════════════════════════════════════════════════════════════════");
    println!();

    // Collect summaries across the sweep.
    let mut summaries: Vec<PhaseSummary> = Vec::new();

    let phases: Vec<f32> = (0..20).map(|i| i as f32 * 0.05).collect();

    for &phase in &phases {
        let (a_ha, a_hm, a_r, a_g, a_b, a_res) =
            run_phase(active_decl_template.clone(), &seq, phase);
        let (r_ha, r_hm, r_r, r_g, r_b, r_res) =
            run_phase(rest_decl_template.clone(), &seq, phase);

        summaries.push(PhaseSummary {
            phase,
            act_haptic_amp:  a_ha,
            act_haptic_ms:   a_hm,
            act_optical_r:   a_r,
            act_optical_g:   a_g,
            act_optical_b:   a_b,
            act_resonance:   a_res,
            rest_haptic_amp: r_ha,
            rest_haptic_ms:  r_hm,
            rest_optical_r:  r_r,
            rest_optical_g:  r_g,
            rest_optical_b:  r_b,
            rest_resonance:  r_res,
        });
    }

    // ── Activity profile sweep ───────────────────────────────────────────────
    println!("── Activity-associated profile: peak outputs per phase point ───────────────────────────────────");
    println!("  (Blue-enriched / 170 Hz / ABR gate=L-activity / amplitude×1.0 / pulse 30 ms base)");
    println!();
    println!("{:>6}  {:>8}  {:>7}  {:>7}  {:>10}  {:>11}",
        "phase", "time", "hap_amp", "hap_ms", "RGB(peak)", "co-activity");
    println!("{:─>6}  {:─>8}  {:─>7}  {:─>7}  {:─>10}  {:─>11}",
        "", "", "", "", "", "");

    for s in &summaries {
        let time_label = phase_to_time(s.phase);
        let res_label  = res_label(&s.act_resonance);
        println!("{:>6.2}  {:>8}  {:>7.3}  {:>7}  {:>10}  {:>11}",
            s.phase,
            time_label,
            s.act_haptic_amp,
            s.act_haptic_ms,
            format!("({},{},{})", s.act_optical_r, s.act_optical_g, s.act_optical_b),
            res_label,
        );
    }
    println!();

    // ── Rest profile sweep ───────────────────────────────────────────────────
    println!("── Rest-associated profile: peak outputs per phase point ────────────────────────────────────────");
    println!("  (Dim red / 170 Hz / ABR gate=L-rest / amplitude×0.5 / pulse 200 ms base)");
    println!();
    println!("{:>6}  {:>8}  {:>7}  {:>7}  {:>10}  {:>11}",
        "phase", "time", "hap_amp", "hap_ms", "RGB(peak)", "co-activity");
    println!("{:─>6}  {:─>8}  {:─>7}  {:─>7}  {:─>10}  {:─>11}",
        "", "", "", "", "", "");

    for s in &summaries {
        let time_label = phase_to_time(s.phase);
        let res_label  = res_label(&s.rest_resonance);
        println!("{:>6.2}  {:>8}  {:>7.3}  {:>7}  {:>10}  {:>11}",
            s.phase,
            time_label,
            s.rest_haptic_amp,
            s.rest_haptic_ms,
            format!("({},{},{})", s.rest_optical_r, s.rest_optical_g, s.rest_optical_b),
            res_label,
        );
    }
    println!();

    // ── Multimodal co-activity map ───────────────────────────────────────────
    println!("── Multimodal co-activity map ───────────────────────────────────────────────────────────────────");
    println!("  CO_ACTIVE = haptic AND optical are simultaneously above threshold at this phase");
    println!("  Thresholds: haptic amplitude > 0.10, any optical channel > 10 (out of 200 max)");
    println!("  These are relational-output thresholds, not physiological targets.");
    println!();
    println!("{:>6}  {:>8}  {:>12}  {:>12}  {:>10}",
        "phase", "time", "activity", "rest", "both active?");
    println!("{:─>6}  {:─>8}  {:─>12}  {:─>12}  {:─>10}",
        "", "", "", "", "");

    let mut active_coactive_count = 0usize;
    let mut rest_coactive_count   = 0usize;
    let mut both_coactive_count   = 0usize;

    for s in &summaries {
        let time_label = phase_to_time(s.phase);
        let act_r = res_label(&s.act_resonance);
        let rst_r = res_label(&s.rest_resonance);
        let both  = s.act_resonance == ResonanceFlag::CoActive
            && s.rest_resonance == ResonanceFlag::CoActive;
        let both_label = if both { "← BOTH" } else { "" };

        if s.act_resonance == ResonanceFlag::CoActive { active_coactive_count += 1; }
        if s.rest_resonance == ResonanceFlag::CoActive { rest_coactive_count += 1; }
        if both { both_coactive_count += 1; }

        println!("{:>6.2}  {:>8}  {:>12}  {:>12}  {}",
            s.phase, time_label, act_r, rst_r, both_label);
    }
    println!();

    // ── Summary ──────────────────────────────────────────────────────────────
    println!("── Sweep summary ────────────────────────────────────────────────────────────────────────────────");
    println!("  Steps with activity co-active:      {} / 20", active_coactive_count);
    println!("  Steps with rest co-active:          {} / 20", rest_coactive_count);
    println!("  Steps with simultaneous (both):     {} / 20", both_coactive_count);
    println!();
    println!("  V0.4 schedule note:");
    println!("  Activity co-active window: declared schedule 05:00–20:00.");
    println!("    rise_start=0.208 (05:00), peak_start=0.25 (06:00),");
    println!("    peak_end=0.75 (18:00), fall_end=0.833 (20:00).");
    println!("  Rest co-active window: declared schedule 19:00–07:00, wrapping across midnight.");
    println!("    rise_start=0.792 (19:00), peak_start=0.875 (21:00),");
    println!("    peak_end=0.125 (03:00), fall_end=0.292 (07:00).");
    println!("  Both schedules are cyclic: profile value at phase=0.0 equals phase=1.0.");
    println!("  The morning transition (rest→activity) is brief; with 20-point sweep");
    println!("  (Δ=0.05, ~72 min/step) it may fall between step boundaries.");
    println!();
    println!("══════════════════════════════════════════════════════════════════════════════════════════════════");
    println!("Evaluator: provisional simulation — not a validated Kernel V8 implementation.");
    println!("All outputs are computed device commands (MockFixture). No physiological effects are claimed.");
    println!("Experimental sensory configurations only. Not therapeutic prescriptions.");
    println!("Scientific basis: docs/verification_record.md");
    println!();
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Approximate local time from normalised circadian phase.
/// 0.0 = midnight, 0.25 = 06:00, 0.5 = noon, 0.75 = 18:00.
fn phase_to_time(phase: f32) -> String {
    let total_minutes = (phase * 24.0 * 60.0) as u32 % (24 * 60);
    let h = total_minutes / 60;
    let m = total_minutes % 60;
    format!("{:02}:{:02}", h, m)
}

fn res_label(flag: &ResonanceFlag) -> &'static str {
    match flag {
        ResonanceFlag::CoActive  => "CO_ACTIVE",
        ResonanceFlag::Partial   => "partial",
        ResonanceFlag::Quiescent => "quiet",
    }
}
