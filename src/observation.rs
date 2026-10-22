// observation.rs
//
// Observation — a single timestamped sensor reading with provenance tag.
//
// Provenance is structural: MockFixture observations cannot be promoted to
// physical validation records. The relational loop only accumulates
// prior_state from Physical observations.

use std::time::SystemTime;
use crate::device_interface::ActuatorState;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Provenance
// ---------------------------------------------------------------------------

/// Observation provenance — enforces the distinction between physical
/// hardware readings and synthetic test fixtures at the type level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObsProvenance {
    /// Reading from a physical DataFeel Dot in an instrumented environment.
    Physical,
    /// Synthetic sequence from MockDevice — excluded from verification records.
    MockFixture,
}

impl ObsProvenance {
    pub fn is_physical(&self) -> bool {
        matches!(self, Self::Physical)
    }
}

// ---------------------------------------------------------------------------
// Observation
// ---------------------------------------------------------------------------

/// A single completed observation of the device at one point in time.
///
/// The relational loop requires a completed prior observation to initialise
/// RelationalState. There is no synthetic initialisation: if no prior
/// Physical observation exists, the loop runs its first step with `None`
/// and derives state from the result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    /// Wall-clock time of this reading.
    #[serde(with = "system_time_serde")]
    pub timestamp: SystemTime,

    /// Thermistor reading in degrees Celsius.
    pub thermistor_c: f32,

    /// Actuator state at the time of this observation
    /// (the last commanded values, or None if no command has been sent yet).
    pub actuator_state: ActuatorState,

    /// Provenance tag — determines eligibility for the verification record.
    pub provenance: ObsProvenance,
}

impl Observation {
    pub fn physical(thermistor_c: f32, actuator_state: ActuatorState) -> Self {
        Self {
            timestamp: SystemTime::now(),
            thermistor_c,
            actuator_state,
            provenance: ObsProvenance::Physical,
        }
    }

    pub fn mock_fixture(thermistor_c: f32, actuator_state: ActuatorState) -> Self {
        Self {
            timestamp: SystemTime::now(),
            thermistor_c,
            actuator_state,
            provenance: ObsProvenance::MockFixture,
        }
    }
}

// ---------------------------------------------------------------------------
// SystemTime serde shim (std::time::SystemTime has no built-in serde support)
// ---------------------------------------------------------------------------

mod system_time_serde {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(t: &SystemTime, s: S) -> Result<S::Ok, S::Error> {
        let secs = t.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO).as_secs_f64();
        secs.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<SystemTime, D::Error> {
        let secs = f64::deserialize(d)?;
        Ok(UNIX_EPOCH + Duration::from_secs_f64(secs))
    }
}

// Serde derives for ActuatorState and its fields — needed for observation serialisation.
// These impls live here because ActuatorState is defined in device_interface.rs
// and we only need serialisation in the context of persisting observations.

impl Serialize for ActuatorState {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("ActuatorState", 3)?;
        st.serialize_field("thermal_power", &self.thermal.as_ref().map(|c| c.power))?;
        st.serialize_field("haptic_amplitude", &self.haptic.as_ref().map(|c| c.amplitude))?;
        st.serialize_field("haptic_duration_ms", &self.haptic.as_ref().map(|c| c.duration_ms))?;
        st.serialize_field("optical_rgb", &self.optical.as_ref().map(|c| [c.r, c.g, c.b]))?;
        st.end()
    }
}

impl<'de> Deserialize<'de> for ActuatorState {
    fn deserialize<D: serde::Deserializer<'de>>(_d: D) -> Result<Self, D::Error> {
        // Minimal deserialisation for replay tests — extend as needed.
        Ok(ActuatorState::default())
    }
}
