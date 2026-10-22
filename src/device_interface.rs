// device_interface.rs
//
// DeviceInterface trait — the stable contract between the relational loop and
// any physical or mock implementation of the DataFeel Dot.
//
// Phase 1: MockDevice is the only implementation. A real DataFeel struct
// implementing this trait is added in Phase 5 once the SDK is available.
// The loop, kernel, and projection are unchanged by that swap.

use std::sync::{Arc, Mutex};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("device not connected")]
    NotConnected,
    #[error("command out of bounds: {0}")]
    OutOfBounds(String),
    #[error("io error: {0}")]
    Io(String),
}

// ---------------------------------------------------------------------------
// Command types
// ---------------------------------------------------------------------------

/// Thermal actuation command.
/// `power` is a fraction in [-1.0, 1.0]: negative = cool, positive = heat.
/// Hardware limits (from docs/hardware_limits.md) are enforced in
/// ActuatorProjection before this reaches the device.
#[derive(Debug, Clone, PartialEq)]
pub struct ThermalCommand {
    /// Normalised power fraction: -1.0 (full cool) to +1.0 (full heat).
    pub power: f32,
}

impl ThermalCommand {
    pub fn new(power: f32) -> Result<Self, DeviceError> {
        if !(-1.0..=1.0).contains(&power) {
            return Err(DeviceError::OutOfBounds(format!(
                "ThermalCommand power {power} outside [-1.0, 1.0]"
            )));
        }
        Ok(Self { power })
    }
}

/// Haptic actuation command.
/// DataFeel Dot resonant actuator: ~170 Hz, usable range ~150–200 Hz.
/// Amplitude and pulse duration are the software-controlled quantities.
#[derive(Debug, Clone, PartialEq)]
pub struct HapticCommand {
    /// Normalised amplitude: 0.0 (off) to 1.0 (maximum rated).
    pub amplitude: f32,
    /// Pulse duration in milliseconds.
    pub duration_ms: u32,
}

impl HapticCommand {
    pub fn new(amplitude: f32, duration_ms: u32) -> Result<Self, DeviceError> {
        if !(0.0..=1.0).contains(&amplitude) {
            return Err(DeviceError::OutOfBounds(format!(
                "HapticCommand amplitude {amplitude} outside [0.0, 1.0]"
            )));
        }
        Ok(Self { amplitude, duration_ms })
    }
}

/// Optical actuation command.
/// DataFeel Dot RGB LEDs: visible wavelengths approximately 466–626 nm.
#[derive(Debug, Clone, PartialEq)]
pub struct OpticalCommand {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    /// Duration this colour should be held, in milliseconds. 0 = indefinite.
    pub timing_ms: u32,
}

/// The full set of actuator commands for one loop step.
#[derive(Debug, Clone)]
pub struct CommandSet {
    pub thermal: ThermalCommand,
    pub haptic:  HapticCommand,
    pub optical: OpticalCommand,
}

/// Last-commanded actuator state — carried in each Observation so the
/// relational loop has a complete picture of device state at every step.
#[derive(Debug, Clone, Default)]
pub struct ActuatorState {
    pub thermal: Option<ThermalCommand>,
    pub haptic:  Option<HapticCommand>,
    pub optical: Option<OpticalCommand>,
}

// ---------------------------------------------------------------------------
// DeviceInterface trait
// ---------------------------------------------------------------------------

pub trait DeviceInterface: Send + Sync {
    /// Read the thermistor. Returns temperature in degrees Celsius.
    fn read_thermistor(&self) -> Result<f32, DeviceError>;

    /// Send a thermal actuation command.
    fn set_thermal(&self, cmd: ThermalCommand) -> Result<(), DeviceError>;

    /// Send a haptic actuation command.
    fn set_haptic(&self, cmd: HapticCommand) -> Result<(), DeviceError>;

    /// Send an optical actuation command.
    fn set_optical(&self, cmd: OpticalCommand) -> Result<(), DeviceError>;
}

// ---------------------------------------------------------------------------
// MockDevice
// ---------------------------------------------------------------------------

/// Mock implementation of DeviceInterface for Phase 1–4 software conformance
/// testing. Feeds a configurable thermistor sequence; records commands sent.
///
/// Observations produced through MockDevice are tagged ObsProvenance::MockFixture
/// and are structurally excluded from the verification record.
pub struct MockDevice {
    /// Thermistor readings returned in sequence; last value repeats when exhausted.
    thermistor_sequence: Vec<f32>,
    /// Current position in the sequence.
    cursor: Mutex<usize>,
    /// Log of all commands received, in order.
    pub command_log: Mutex<Vec<MockCommand>>,
}

#[derive(Debug, Clone)]
pub enum MockCommand {
    Thermal(ThermalCommand),
    Haptic(HapticCommand),
    Optical(OpticalCommand),
}

impl MockDevice {
    /// Construct a MockDevice with a fixed thermistor sequence.
    /// e.g. MockDevice::new(vec![36.5, 37.0, 37.8, 38.2, 37.5])
    pub fn new(thermistor_sequence: Vec<f32>) -> Arc<Self> {
        assert!(!thermistor_sequence.is_empty(), "thermistor sequence must not be empty");
        Arc::new(Self {
            thermistor_sequence,
            cursor: Mutex::new(0),
            command_log: Mutex::new(Vec::new()),
        })
    }

    /// Default mock: a gentle thermal disturbance sequence for initial testing.
    pub fn default_sequence() -> Arc<Self> {
        Self::new(vec![
            36.5, 36.6, 36.9, 37.3, 37.8, 38.2, 38.1, 37.7, 37.3, 36.9, 36.6, 36.5,
        ])
    }
}

impl DeviceInterface for MockDevice {
    fn read_thermistor(&self) -> Result<f32, DeviceError> {
        let mut cursor = self.cursor.lock().unwrap();
        let val = self.thermistor_sequence[*cursor];
        *cursor = (*cursor + 1).min(self.thermistor_sequence.len() - 1);
        Ok(val)
    }

    fn set_thermal(&self, cmd: ThermalCommand) -> Result<(), DeviceError> {
        self.command_log.lock().unwrap().push(MockCommand::Thermal(cmd));
        Ok(())
    }

    fn set_haptic(&self, cmd: HapticCommand) -> Result<(), DeviceError> {
        self.command_log.lock().unwrap().push(MockCommand::Haptic(cmd));
        Ok(())
    }

    fn set_optical(&self, cmd: OpticalCommand) -> Result<(), DeviceError> {
        self.command_log.lock().unwrap().push(MockCommand::Optical(cmd));
        Ok(())
    }
}
