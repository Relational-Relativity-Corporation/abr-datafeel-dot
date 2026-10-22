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
// The projection inspects the declaration's relations to route locus values
// to actuator channels. Recognised target loci:
//   "L-thermal" → ThermalCommand  (thermal feedback / homeostatic correction)
//   "L-haptic"  → HapticCommand   (vibrotactile amplitude and duration)
//   "L-red"     → OpticalCommand r channel (~626 nm)
//   "L-green"   → OpticalCommand g channel (~530 nm)
//   "L-blue"    → OpticalCommand b channel (~466 nm)
//
// Unknown target loci are silently ignored — the projection only acts on
// what it knows how to map. This keeps the projection stable as declarations
// evolve.
//
// The projection rules below are initial declarations for Phase 4 testing.
// Origin must review and finalise these mappings against the actual
// DataFeel hardware limits (docs/hardware_limits.md) before physical actuation.

use thiserror::Error;

use crate::declaration::{MDeclaration, OperatorPath};
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
    /// Iterates the declaration's relations to determine which locus values
    /// drive which actuator channels. For each relation:
    ///
    ///   PrimaryΔΣ  — the `from` locus's sigma/value in locus_values drives
    ///                the `to` locus's actuator channel.
    ///   ABR        — not evaluated in the Phase 1–4 stub; ABR relations are
    ///                recorded in the declaration but deferred to Phase 3.
    ///
    /// Channel mapping:
    ///   "L-thermal" → thermal power (oppose delta: cooling on warming, heating on cooling)
    ///   "L-haptic"  → haptic amplitude proportional to |driving value|
    ///   "L-red"     → r channel intensity proportional to driving value
    ///   "L-green"   → g channel intensity proportional to |driving value|
    ///   "L-blue"    → b channel intensity proportional to driving value
    ///
    /// All values are clamped to HardwareLimits before dispatch.
    pub fn project(
        &self,
        state: &RelationalState,
        decl: &MDeclaration,
    ) -> Result<CommandSet, ProjectionError> {
        let delta = state.delta_thermistor.unwrap_or(0.0);

        // Accumulate optical channel contributions across all relations.
        let mut r_sum: f32 = 0.0;
        let mut g_sum: f32 = 0.0;
        let mut b_sum: f32 = 0.0;
        let mut thermal_power: f32 = 0.0;
        let mut haptic_amplitude: f32 = 0.0;
        let mut haptic_duration_ms: u32 = 0;

        for rel in &decl.relations {
            // Phase 1–4 stub: only evaluate PrimaryDeltaSigma relations.
            if !matches!(rel.operator_path, OperatorPath::PrimaryDeltaSigma) {
                continue;
            }

            // Get the driving value for this relation's source locus.
            let driving = state.locus_values.get(&rel.from).copied().unwrap_or(0.0);

            match rel.to.as_str() {
                "L-thermal" => {
                    // Oppose the thermal delta.
                    if delta.abs() > f32::EPSILON {
                        let raw = -delta.signum() * (delta.abs() / 5.0).min(1.0);
                        thermal_power += raw;
                    }
                }
                "L-haptic" => {
                    // Amplitude proportional to |driving value|, scaled to [0, 1].
                    haptic_amplitude += (driving.abs() / 3.0).min(1.0);
                    haptic_duration_ms = haptic_duration_ms
                        .max((driving.abs() * 50.0) as u32);
                }
                "L-red" => {
                    // Intensity proportional to driving value (clamped positive).
                    r_sum += (driving / 10.0).clamp(0.0, 1.0);
                }
                "L-green" => {
                    g_sum += (driving.abs() / 10.0).min(1.0);
                }
                "L-blue" => {
                    // Intensity proportional to driving value (clamped positive).
                    b_sum += (driving / 10.0).clamp(0.0, 1.0);
                }
                _ => {
                    // Unknown target locus — ignore. Declaration may include
                    // loci not yet handled by this projection stub.
                }
            }
        }

        // --- Thermal ---
        let thermal_power =
            thermal_power.clamp(-self.limits.thermal_power_max, self.limits.thermal_power_max);
        let thermal = ThermalCommand::new(thermal_power)?;

        // --- Haptic ---
        let haptic_amplitude =
            haptic_amplitude.min(self.limits.haptic_amplitude_max);
        let haptic_duration_ms =
            haptic_duration_ms.min(self.limits.haptic_duration_max_ms);
        let haptic = HapticCommand::new(haptic_amplitude, haptic_duration_ms)?;

        // --- Optical ---
        let max_ch = self.limits.optical_intensity_max as f32;
        let r = (r_sum.min(1.0) * max_ch) as u8;
        let g = (g_sum.min(1.0) * max_ch) as u8;
        let b = (b_sum.min(1.0) * max_ch) as u8;
        let optical = OpticalCommand { r, g, b, timing_ms: 100 };

        Ok(CommandSet { thermal, haptic, optical })
    }
}
