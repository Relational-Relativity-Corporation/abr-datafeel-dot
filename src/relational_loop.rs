// relational_loop.rs
//
// RelationalLoop — core loop: read → observe → evaluate → project → command.
//
// Kernel V8 integration is stubbed here pending the abr-kernel crate being
// added as a pinned dependency (Phase 3). The stub preserves the interface
// so that Phase 1–2 conformance tests can run against the full loop structure.
//
// KEY INVARIANT: prior_state is NEVER initialised synthetically.
// The first step runs with prior_state = None. RelationalState accumulates
// only from completed Physical observations. MockFixture observations do not
// update prior_state.

use std::sync::Arc;
use thiserror::Error;

use crate::device_interface::{ActuatorState, CommandSet, DeviceInterface, DeviceError};
use crate::observation::{Observation, ObsProvenance};
use crate::declaration::MDeclaration;
use crate::actuator_projection::{ActuatorProjection, ProjectionError};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum LoopError {
    #[error("device error: {0}")]
    Device(#[from] DeviceError),
    #[error("projection error: {0}")]
    Projection(#[from] ProjectionError),
    #[error("kernel error: {0}")]
    Kernel(String),
}

// ---------------------------------------------------------------------------
// Relational state
// ---------------------------------------------------------------------------

/// Output from one evaluation of the relational process.
/// Carried forward as prior_state on the next step.
///
/// When abr-kernel is wired in (Phase 3), this will hold the kernel's
/// full operator result. For now it carries the minimal state needed
/// for the loop to run and tests to pass.
#[derive(Debug, Clone)]
pub struct RelationalState {
    /// The observation that produced this state.
    pub source_observation: Observation,
    /// Δ (difference) from the previous thermistor reading, if one exists.
    pub delta_thermistor: Option<f32>,
    /// Σ (accumulation) of Δ values over completed physical steps.
    pub sigma_thermistor: f32,
}

/// Full output of one loop step, including the state to carry forward
/// and the commands dispatched to the device.
#[derive(Debug, Clone)]
pub struct StepOutput {
    pub state:    RelationalState,
    pub commands: CommandSet,
    /// Observation that was the input to this step.
    pub observation: Observation,
}

// ---------------------------------------------------------------------------
// RelationalLoop
// ---------------------------------------------------------------------------

pub struct RelationalLoop {
    pub declaration: MDeclaration,
    pub projection:  ActuatorProjection,
    pub device:      Arc<dyn DeviceInterface>,

    /// Accumulated state from completed Physical steps only.
    /// None until the first Physical observation completes successfully.
    /// This is the verification record anchor — MockFixture observations
    /// do not update it under any circumstances.
    prior_state: Option<RelationalState>,

    /// Most recent step state, regardless of provenance.
    /// Used by evaluate_stub to compute Δ and Σ continuity within a run.
    /// Distinct from prior_state: this is not a verification record.
    step_state: Option<RelationalState>,

    /// Current actuator state — updated after each successful command dispatch.
    actuator_state: ActuatorState,
}

impl RelationalLoop {
    pub fn new(
        declaration: MDeclaration,
        projection: ActuatorProjection,
        device: Arc<dyn DeviceInterface>,
    ) -> Self {
        Self {
            declaration,
            projection,
            device,
            prior_state: None,
            step_state: None,
            actuator_state: ActuatorState::default(),
        }
    }

    /// Execute one step of the relational loop.
    ///
    /// 1. Read thermistor from device.
    /// 2. Construct Observation (Physical or MockFixture per device type).
    /// 3. Evaluate Kernel V8 operators against declared topology and prior state.
    /// 4. Project relational output to actuator commands.
    /// 5. Dispatch commands to device.
    /// 6. Update prior_state (Physical observations only).
    pub fn step(&mut self, provenance: ObsProvenance) -> Result<StepOutput, LoopError> {
        // 1. Read sensor.
        let thermistor_c = self.device.read_thermistor()?;

        // 2. Construct observation.
        let observation = match provenance {
            ObsProvenance::Physical =>
                Observation::physical(thermistor_c, self.actuator_state.clone()),
            ObsProvenance::MockFixture =>
                Observation::mock_fixture(thermistor_c, self.actuator_state.clone()),
        };

        // 3. Evaluate — Kernel V8 stub (Phase 3 will replace this block).
        let state = self.evaluate_stub(&observation);

        // 4. Project to actuator commands.
        let commands = self.projection.project(&state)?;

        // 5. Dispatch commands.
        self.device.set_thermal(commands.thermal.clone())?;
        self.device.set_haptic(commands.haptic.clone())?;
        self.device.set_optical(commands.optical.clone())?;

        // Update tracked actuator state.
        self.actuator_state.thermal = Some(commands.thermal.clone());
        self.actuator_state.haptic  = Some(commands.haptic.clone());
        self.actuator_state.optical = Some(commands.optical.clone());

        // 6. step_state always advances — provides Δ/Σ continuity for evaluate_stub.
        //    prior_state advances only on Physical — this is the invariant.
        self.step_state = Some(state.clone());
        if observation.provenance.is_physical() {
            self.prior_state = Some(state.clone());
        }

        Ok(StepOutput { state, commands, observation })
    }

    /// Kernel V8 stub — implements Primary Δ→Σ for the thermistor locus.
    /// Replaced in Phase 3 when abr-kernel is added as a pinned dependency.
    fn evaluate_stub(&self, obs: &Observation) -> RelationalState {
        // Use step_state for continuity: delta and sigma advance every step
        // regardless of provenance. prior_state is the verification anchor
        // (Physical only) and is not consulted here.
        let delta = self.step_state.as_ref().map(|prior| {
            obs.thermistor_c - prior.source_observation.thermistor_c
        });

        let sigma = match (&self.step_state, delta) {
            (Some(prior), Some(d)) => prior.sigma_thermistor + d,
            _ => 0.0,
        };

        RelationalState {
            source_observation: obs.clone(),
            delta_thermistor: delta,
            sigma_thermistor: sigma,
        }
    }

    /// Returns a reference to the current prior state, if any.
    pub fn prior_state(&self) -> Option<&RelationalState> {
        self.prior_state.as_ref()
    }
}
