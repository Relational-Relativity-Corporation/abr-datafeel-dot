// actuator_projection.rs
//
// ActuatorProjection — maps RelationalState to bounded device commands.
//
// Every projection is explicit: which relational quantities are read,
// which are discarded, and what the hardware bounds are. Nothing is
// silently forwarded. Hardware limits are declared here and enforced
// before any command reaches the device.
//
// The projection rules below are initial declarations for Phase 4 testing.
// Origin must review and finalise these mappings against the actual
// DataFeel hardware limits (docs/hardware_limits.md) before physical actuation.

use thiserror::Error;

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
}

impl Default for HardwareLimits {
    fn default() -> Self {
        // Conservative placeholders — NOT confirmed DataFeel limits.
        Self {
            thermal_power_max: 0.8,
            haptic_amplitude_max: 0.7,
            haptic_duration_max_ms: 500,
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
    /// Mapping rationale (Phase 4 initial declarations, subject to Origin review):
    ///
    /// Thermal:
    ///   Read: delta_thermistor, sigma_thermistor
    ///   Rule: oppose the observed Δ — if temperature rose, cool; if fell, heat.
    ///         Magnitude proportional to |Δ|, capped at thermal_power_max.
    ///   Discard: optical state, haptic state.
    ///
    /// Haptic:
    ///   Read: delta_thermistor
    ///   Rule: amplitude = |Δ| normalised to [0, haptic_amplitude_max].
    ///         Active on any nonzero Δ; duration proportional to |Δ|.
    ///   Discard: sigma, optical state.
    ///
    /// Optical:
    ///   Read: sigma_thermistor
    ///   Rule: encode cumulative thermal drift as a colour shift.
    ///         Positive sigma (warming) → warm hue; negative (cooling) → cool hue.
    ///         Intensity scales with |sigma|.
    ///   Discard: delta, haptic state.
    pub fn project(&self, state: &RelationalState) -> Result<CommandSet, ProjectionError> {
        let delta = state.delta_thermistor.unwrap_or(0.0);
        let sigma = state.sigma_thermistor;

        // --- Thermal ---
        let thermal_power = if delta.abs() < f32::EPSILON {
            0.0
        } else {
            // Oppose the observed change, scaled by magnitude.
            let raw = -delta.signum() * (delta.abs() / 5.0).min(1.0);
            raw.clamp(-self.limits.thermal_power_max, self.limits.thermal_power_max)
        };
        let thermal = ThermalCommand::new(thermal_power)?;

        // --- Haptic ---
        let haptic_amplitude = (delta.abs() / 3.0)
            .min(self.limits.haptic_amplitude_max);
        let haptic_duration_ms = ((delta.abs() * 50.0) as u32)
            .min(self.limits.haptic_duration_max_ms);
        let haptic = HapticCommand::new(haptic_amplitude, haptic_duration_ms)?;

        // --- Optical ---
        // Warm hue (red dominant): sigma > 0 (cumulative warming)
        // Cool hue (blue dominant): sigma < 0 (cumulative cooling)
        let intensity = (sigma.abs() / 10.0).min(1.0);
        let (r, g, b) = if sigma >= 0.0 {
            ((intensity * 220.0) as u8, (intensity * 80.0) as u8, (intensity * 30.0) as u8)
        } else {
            ((intensity * 30.0) as u8, (intensity * 80.0) as u8, (intensity * 220.0) as u8)
        };
        let optical = OpticalCommand { r, g, b, timing_ms: 100 };

        Ok(CommandSet { thermal, haptic, optical })
    }
}
