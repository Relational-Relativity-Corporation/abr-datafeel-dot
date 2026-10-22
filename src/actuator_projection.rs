// actuator_projection.rs
//
// ActuatorProjection — maps RelationalState to bounded device commands,
// using the declared relational topology to determine which locus values
// drive which actuator channels.
//
// Every projection is explicit: which relational quantities are read,
// which are discarded, and what the hardware bounds are. Nothing is
// silently forwarded. Hardware limits are declared here and enforced
// before any command reaches the device.
//
// V0.3: The projection now reads per-relation OperatorOutput.sigma values
// from RelationalState.relation_outputs, replacing the previous approach
// of reading raw locus values. Both PrimaryDeltaSigma and ABR relations
// are evaluated — the gated sigma from ABR carries the coupling between
// the thermal source locus and the circadian interior locus.
//
// The haptic envelope (carrier Hz, base duration, amplitude scale) is
// read from the MDeclaration and applied when projecting haptic commands.
// This differentiates activity and rest profiles at the same carrier frequency.
//
// Channel routing by target locus ID:
//   "L-thermal" → ThermalCommand  (oppose delta)
//   "L-haptic"  → HapticCommand   (gated sigma → amplitude + envelope duration)
//   "L-red"     → OpticalCommand r channel (~626 nm, PBM window)
//   "L-green"   → OpticalCommand g channel (~530 nm)
//   "L-blue"    → OpticalCommand b channel (~466 nm, ipRGC pathway)

use thiserror::Error;

use crate::declaration::MDeclaration;
use crate::device_interface::{
    CommandSet, HapticCommand, OpticalCommand, ThermalCommand, DeviceError,
};
use crate::relational_loop::RelationalState;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum ProjectionError {
    #[error("command out of bounds: {0}")]
    OutOfBounds(#[from] DeviceError),
    #[error("projection rule error: {0}")]
    Rule(String),
}

// ---------------------------------------------------------------------------
// Hardware limits
// ---------------------------------------------------------------------------

/// Hardware operating limits — placeholder values.
/// MUST be replaced with DataFeel-specified values from docs/hardware_limits.md
/// before any physical actuation. These are intentionally conservative.
pub struct HardwareLimits {
    /// Maximum thermal power fraction (absolute value). Symmetric: [-max, +max].
    pub thermal_power_max: f32,
    /// Maximum haptic amplitude fraction.
    pub haptic_amplitude_max: f32,
    /// Maximum haptic pulse duration in ms.
    pub haptic_duration_max_ms: u32,
    /// Maximum optical channel intensity (0–255).
    pub optical_intensity_max: u8,
}

impl Default for HardwareLimits {
    fn default() -> Self {
        // Conservative placeholders — NOT confirmed DataFeel limits.
        Self {
            thermal_power_max: 0.8,
            haptic_amplitude_max: 0.7,
            haptic_duration_max_ms: 500,
            optical_intensity_max: 200,
        }
    }
}

// ---------------------------------------------------------------------------
// Resonance
// ---------------------------------------------------------------------------

/// Co-activity flag — indicates whether haptic and optical command outputs
/// are simultaneously above a meaningful threshold in this step.
///
/// This is a computed observation from the declared relational topology's
/// outputs, not a claim about physiological effect. It marks where both
/// modalities are jointly expressing above threshold in the circadian sweep.
///
/// Renamed from `Resonant` in V0.4 to avoid implying physiological resonance
/// — this flag refers only to simultaneous command thresholds.
#[derive(Debug, Clone, PartialEq)]
pub enum ResonanceFlag {
    /// Both haptic and at least one optical channel are above threshold.
    CoActive,
    /// Only one modality is above threshold.
    Partial,
    /// Neither modality is above threshold.
    Quiescent,
}

// ---------------------------------------------------------------------------
// ActuatorProjection
// ---------------------------------------------------------------------------

pub struct ActuatorProjection {
    pub limits: HardwareLimits,
}

impl Default for ActuatorProjection {
    fn default() -> Self {
        Self { limits: HardwareLimits::default() }
    }
}

impl ActuatorProjection {
    pub fn new(limits: HardwareLimits) -> Self {
        Self { limits }
    }

    /// Project a RelationalState to a CommandSet.
    ///
    /// Iterates all declared relations (PrimaryΔΣ and ABR), reads their
    /// OperatorOutput.sigma from state.relation_outputs, and routes to
    /// the appropriate actuator channel based on the target locus ID.
    ///
    /// The haptic envelope from the declaration modulates duration and
    /// amplitude scale — this is how activity and rest profiles produce
    /// different haptic character at the same carrier frequency.
    pub fn project(
        &self,
        state: &RelationalState,
        decl: &MDeclaration,
    ) -> Result<CommandSet, ProjectionError> {
        let delta = state.delta_thermistor.unwrap_or(0.0);

        let mut r_sum: f32 = 0.0;
        let mut g_sum: f32 = 0.0;
        let mut b_sum: f32 = 0.0;
        let mut thermal_power: f32 = 0.0;
        let mut haptic_amplitude: f32 = 0.0;
        let mut haptic_duration_ms: u32 = 0;

        let envelope = &decl.haptic_envelope;

        for rel in &decl.relations {
            // Read the operator output for this relation.
            // For relations not yet in relation_outputs (e.g. on first step),
            // fall back to 0.0 driving value.
            let output = state.relation_outputs.get(&rel.id);
            let sigma = output.map(|o| o.sigma).unwrap_or(0.0);

            match rel.to.as_str() {
                "L-thermal" => {
                    // Oppose the thermal delta regardless of operator path.
                    if delta.abs() > f32::EPSILON {
                        let raw = -delta.signum() * (delta.abs() / 5.0).min(1.0);
                        thermal_power += raw;
                    }
                }
                "L-haptic" => {
                    // Amplitude proportional to |gated sigma|, scaled by profile
                    // amplitude_scale and duration_ms_base from the envelope.
                    let raw_amp = (sigma.abs() / 3.0).min(1.0) * envelope.amplitude_scale;
                    haptic_amplitude += raw_amp;

                    // Duration: envelope base × sigma magnitude, capped at max.
                    let raw_dur = (sigma.abs() * envelope.duration_ms_base as f32) as u32;
                    haptic_duration_ms = haptic_duration_ms.max(raw_dur);
                }
                "L-red" => {
                    // Intensity proportional to gated sigma (clamped positive).
                    r_sum += (sigma / 10.0).clamp(0.0, 1.0);
                }
                "L-green" => {
                    g_sum += (sigma.abs() / 10.0).min(1.0);
                }
                "L-blue" => {
                    // Intensity proportional to gated sigma (clamped positive).
                    b_sum += (sigma / 10.0).clamp(0.0, 1.0);
                }
                _ => {
                    // Unknown target locus — ignore silently.
                }
            }
        }

        // --- Thermal ---
        let thermal_power =
            thermal_power.clamp(-self.limits.thermal_power_max, self.limits.thermal_power_max);
        let thermal = ThermalCommand::new(thermal_power)?;

        // --- Haptic ---
        let haptic_amplitude = haptic_amplitude.min(self.limits.haptic_amplitude_max);
        let haptic_duration_ms = haptic_duration_ms.min(self.limits.haptic_duration_max_ms);
        let haptic = HapticCommand::new(haptic_amplitude, haptic_duration_ms)?;

        // --- Optical ---
        let max_ch = self.limits.optical_intensity_max as f32;
        let r = (r_sum.min(1.0) * max_ch) as u8;
        let g = (g_sum.min(1.0) * max_ch) as u8;
        let b = (b_sum.min(1.0) * max_ch) as u8;
        let optical = OpticalCommand { r, g, b, timing_ms: 100 };

        Ok(CommandSet { thermal, haptic, optical })
    }

    /// Compute the multimodal resonance flag for a given command set.
    ///
    /// Resonance thresholds (conservative — not derived from clinical evidence):
    ///   Haptic: amplitude > 0.10 (10% of normalised max)
    ///   Optical: any channel > 10 (out of 200 max intensity)
    ///
    /// These are instrument thresholds for identifying the intersection region
    /// in the sweep output. They are not therapeutic targets.
    pub fn resonance_flag(commands: &CommandSet) -> ResonanceFlag {
        let haptic_active  = commands.haptic.amplitude > 0.10;
        let optical_active = commands.optical.r > 10
            || commands.optical.g > 10
            || commands.optical.b > 10;

        match (haptic_active, optical_active) {
            (true, true)  => ResonanceFlag::CoActive,
            (true, false) | (false, true) => ResonanceFlag::Partial,
            (false, false) => ResonanceFlag::Quiescent,
        }
    }
}
