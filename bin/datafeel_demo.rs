// datafeel_demo.rs
//
// DataFeel Dot — V0.3 multimodal resonance sweep.
//
// Sweeps the circadian phase (time_of_day_phase) from 0.0 → 1.0 in 20 steps
// (Δ = 0.05 per step, representing ~72 minutes of circadian time per step).
// At each phase point, the same thermal disturbance sequence is run through
// both the activity-associated and rest-associated profiles using the ABR
// operator.
//
// The sweep reveals the AREA OF MULTIMODAL INTERSECTION — where haptic AND
// optical outputs are simultaneously expressing above a meaningful threshold.
// Because both relations in each profile share the same circadian interior
// locus (L-circadian as gate B), the resonance window is defined by the gate
// value, not by the thermal input alone.
//
// Activity profile (gate_inverted=false):  sigma × B → strong when B high (daytime)
// Rest profile    (gate_inverted=true):    sigma × (1−B) → strong when B low (night)
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

        // Escalate resonance flag.
        best_resonance = match (&best_resonance, &flag) {
            (_, ResonanceFlag::Resonant)   => ResonanceFlag::Resonant,
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
    println!("DataFeel Dot — V0.3 Multimodal Resonance Sweep");
    println!("══════════════════════════════════════════════════════════════════════════════════════════════════");
    println!("Circadian phase sweep: 0.00 → 0.95, Δ = 0.05 (20 steps ≈ 72 min each)");
    println!("Thermal sequence: 12-step external disturbance → recovery, MockFixture, no hardware");
    println!("Operator: ABR — both haptic (R-01) and optical (R-02) gated by L-circadian (B)");
    println!("Activity gate: sigma × B    (strong when B high / daytime, phase ~0.25–0.75)");
    println!("Rest gate:     sigma × (1−B) (strong when B low / night, phase ~0.0–0.25, 0.75–1.0)");
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
    println!("  (Blue-enriched / 170 Hz / ABR gate=B / amplitude×1.0 / pulse 30 ms base)");
    println!();
    println!("{:>6}  {:>8}  {:>7}  {:>7}  {:>10}  {:>11}",
        "phase", "time", "hap_amp", "hap_ms", "RGB(peak)", "resonance");
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
    println!("  (Dim red / 170 Hz / ABR gate=(1−B) / amplitude×0.5 / pulse 200 ms base)");
    println!();
    println!("{:>6}  {:>8}  {:>7}  {:>7}  {:>10}  {:>11}",
        "phase", "time", "hap_amp", "hap_ms", "RGB(peak)", "resonance");
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

    // ── Multimodal resonance map ─────────────────────────────────────────────
    println!("── Multimodal resonance map ─────────────────────────────────────────────────────────────────────");
    println!("  RESONANT = haptic AND optical are simultaneously above threshold at this phase");
    println!("  Thresholds: haptic amplitude > 0.10, any optical channel > 10 (out of 200 max)");
    println!("  These are relational-output thresholds, not physiological targets.");
    println!();
    println!("{:>6}  {:>8}  {:>12}  {:>12}  {:>10}",
        "phase", "time", "activity", "rest", "both active?");
    println!("{:─>6}  {:─>8}  {:─>12}  {:─>12}  {:─>10}",
        "", "", "", "", "");

    let mut active_resonant_count = 0usize;
    let mut rest_resonant_count   = 0usize;
    let mut both_resonant_count   = 0usize;

    for s in &summaries {
        let time_label = phase_to_time(s.phase);
        let act_r = res_label(&s.act_resonance);
        let rst_r = res_label(&s.rest_resonance);
        let both  = s.act_resonance == ResonanceFlag::Resonant
            && s.rest_resonance == ResonanceFlag::Resonant;
        let both_label = if both { "← BOTH" } else { "" };

        if s.act_resonance == ResonanceFlag::Resonant { active_resonant_count += 1; }
        if s.rest_resonance == ResonanceFlag::Resonant { rest_resonant_count += 1; }
        if both { both_resonant_count += 1; }

        println!("{:>6.2}  {:>8}  {:>12}  {:>12}  {}",
            s.phase, time_label, act_r, rst_r, both_label);
    }
    println!();

    // ── Summary ──────────────────────────────────────────────────────────────
    println!("── Sweep summary ────────────────────────────────────────────────────────────────────────────────");
    println!("  Phases with activity resonance:  {} / 20", active_resonant_count);
    println!("  Phases with rest resonance:      {} / 20", rest_resonant_count);
    println!("  Phases with simultaneous (both): {} / 20", both_resonant_count);
    println!();
    println!("  Circadian biology note:");
    println!("  Activity resonance window corresponds to the daytime gate (B = phase, high midday).");
    println!("  Rest resonance window corresponds to the biological night gate (1−B, high at midnight).");
    println!("  The windows are complementary by construction — the same thermal disturbance");
    println!("  signal activates different modality combinations depending on circadian phase.");
    println!();
    println!("══════════════════════════════════════════════════════════════════════════════════════════════════");
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
        ResonanceFlag::Resonant  => "RESONANT",
        ResonanceFlag::Partial   => "partial",
        ResonanceFlag::Quiescent => "quiet",
    }
}
