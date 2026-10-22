// relational_loop.rs
//
// RelationalLoop — core loop: read → observe → evaluate → project → command.
//
// Kernel V8 integration: the evaluate_stub function below is a provisional
// simulation of the PrimaryΔΣ and ABR operator paths. It will be replaced
// when the abr-kernel crate is available as a pinned dependency (Phase 3).
// The provisional evaluator preserves the interface contract so that Phase 1–4
// conformance tests can run against the declared relational topology without
// requiring the kernel dependency. Output from evaluate_stub is NOT a validated
// Kernel V8 result.
//
// KEY INVARIANT: prior_state is NEVER initialised synthetically.
// The first step runs with prior_state = None. RelationalState accumulates
// only from completed Physical observations. MockFixture observations do not
// update prior_state.
//
// Per-locus tracking: the loop maintains raw sigma per relation across steps
// (in step_state.relation_outputs[rel.id].raw_sigma). This enables the ABR
// gate to be applied to the accumulated ungated sigma at each step, producing
// a gated output that reflects both the history of the source locus and the
// current state of the interior locus.

use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

use crate::device_interface::{ActuatorState, CommandSet, DeviceInterface, DeviceError};
use crate::observation::{Observation, ObsProvenance};
use crate::declaration::{MDeclaration, OperatorPath, LocusSource};
use crate::actuator_projection::{ActuatorProjection, ProjectionError};

// ---------------------------------------------------------------------------
// Host observables
// ---------------------------------------------------------------------------

/// Key–value map of observables provided by the host system.
///
/// These are loci declared with `LocusSource::Host` — values the device
/// itself cannot read but the host can supply (e.g. time_of_day_phase).
/// Also used as the clock source for `LocusSource::Origin` loci, which
/// derive their schedule-computed value from a named host observable.
///
/// V0.4: Missing entries for Host or Origin loci are no longer silently
/// substituted. The loop returns `LoopError::MissingHostObservable` if a
/// required host observable is absent from this map.
pub type HostObservables = HashMap<String, f32>;

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
    /// A Host or Origin locus requires an observable that was not supplied
    /// in the HostObservables map. Silent substitution is not performed.
    #[error("missing host observable: '{0}' — required by declaration but not supplied in host_obs")]
    MissingHostObservable(String),
}

// ---------------------------------------------------------------------------
// Operator output
// ---------------------------------------------------------------------------

/// Output from one provisional-simulation operator evaluation for a single relation.
///
/// raw_sigma: ungated accumulation of delta values — carries the history of
///            the source locus regardless of any ABR gate state.
/// sigma:     the output used by the projection — equals raw_sigma for
///            PrimaryDeltaSigma; for ABR, equals raw_sigma × gate(B).
#[derive(Debug, Clone, Default)]
pub struct OperatorOutput {
    /// Δ from the previous step for the source locus. None on first step.
    pub delta: Option<f32>,
    /// Ungated Σ — accumulates regardless of ABR interior locus value.
    pub raw_sigma: f32,
    /// Gated Σ — what the projection reads. For ABR: raw_sigma × gate(B).
    pub sigma: f32,
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
    /// Backward-compat field — derived from the first Device-source relation.
    pub delta_thermistor: Option<f32>,
    /// Σ (accumulation) backward-compat field — derived from R-01 sigma.
    pub sigma_thermistor: f32,
    /// Current raw value of each declared locus (thermistor_c, phase value, etc.).
    /// Key: locus ID.
    pub locus_values: HashMap<String, f32>,
    /// Per-relation operator outputs. Key: relation ID.
    /// The projection reads these to determine actuator driving values.
    pub relation_outputs: HashMap<String, OperatorOutput>,
}

/// Full output of one loop step, including the state to carry forward
/// and the commands dispatched to the device.
#[derive(Debug, Clone)]
pub struct StepOutput {
    pub state:       RelationalState,
    pub commands:    CommandSet,
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
    /// Used by evaluate_stub to compute Δ, Σ, and per-relation raw sigmas.
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
    /// 2. Construct Observation (Physical or MockFixture per provenance).
    /// 3. Evaluate operator paths via provisional simulation (Phase 3: abr-kernel).
    /// 4. Project relational output to actuator commands.
    /// 5. Dispatch commands to device.
    /// 6. Update prior_state (Physical observations only).
    ///
    /// `host_obs` supplies values for Host-sourced loci declared in the M
    /// (e.g. `time_of_day_phase`) and the clock source for Origin loci.
    /// A missing entry for any required Host or Origin observable returns
    /// `LoopError::MissingHostObservable` — no silent substitution is performed.
    pub fn step(
        &mut self,
        provenance: ObsProvenance,
        host_obs: &HostObservables,
    ) -> Result<StepOutput, LoopError> {
        // 1. Read sensor.
        let thermistor_c = self.device.read_thermistor()?;

        // 2. Construct observation.
        let observation = match provenance {
            ObsProvenance::Physical =>
                Observation::physical(thermistor_c, self.actuator_state.clone()),
            ObsProvenance::MockFixture =>
                Observation::mock_fixture(thermistor_c, self.actuator_state.clone()),
        };

        // 3. Evaluate — provisional simulation (will be replaced by abr-kernel once available).
        let state = self.evaluate_stub(&observation, host_obs)?;

        // 4. Project to actuator commands.
        let commands = self.projection.project(&state, &self.declaration)?;

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

    /// Provisional simulation of operator evaluation for all declared relations.
    ///
    /// This function approximates PrimaryΔΣ and ABR operator behavior to support
    /// Phase 1–4 conformance testing. It is NOT a validated Kernel V8 implementation.
    /// It will be replaced in Phase 3 when abr-kernel is added as a pinned dependency.
    ///
    /// For each declared relation:
    ///
    ///   PrimaryΔΣ — reads the source locus value (Device or Host), computes
    ///               Δ from the prior step's value, accumulates Σ. Output = Σ.
    ///
    ///   ABR       — reads the source locus (A) and interior locus (B).
    ///               Computes raw Σ for A as above. Applies gate:
    ///                 gate_inverted=false → output = raw_Σ × B
    ///                 gate_inverted=true  → output = raw_Σ × (1−B)
    ///               For V0.4+, B is typically an Origin locus whose schedule
    ///               directly expresses the active phase, making gate_inverted
    ///               false for both activity and rest profiles.
    ///
    /// raw_sigma is preserved across steps in step_state.relation_outputs,
    /// so the ungated history of the source locus is always available.
    ///
    /// Returns Err(LoopError::MissingHostObservable) if any Host or Origin locus
    /// references an observable not present in host_obs.
    fn evaluate_stub(
        &self,
        obs: &Observation,
        host_obs: &HostObservables,
    ) -> Result<RelationalState, LoopError> {
        // 1. Resolve current raw value for every declared locus.
        //    Host and Origin loci require entries in host_obs — no silent substitution.
        let mut locus_values: HashMap<String, f32> = HashMap::new();
        for locus in &self.declaration.loci {
            let val = match &locus.source {
                LocusSource::Device => obs.thermistor_c,
                LocusSource::Actuator => 0.0,
                LocusSource::Host => {
                    host_obs.get(&locus.observable)
                        .copied()
                        .ok_or_else(|| LoopError::MissingHostObservable(locus.observable.clone()))?
                }
                LocusSource::Origin(schedule) => {
                    // Evaluate the declared trapezoidal schedule at the current
                    // host-supplied clock phase.
                    let phase = host_obs.get(&schedule.derives_from)
                        .copied()
                        .ok_or_else(|| LoopError::MissingHostObservable(schedule.derives_from.clone()))?;
                    schedule.value_at(phase)
                }
            };
            locus_values.insert(locus.id.clone(), val);
        }

        // 2. For each relation, compute the provisional operator output.
        let mut relation_outputs: HashMap<String, OperatorOutput> = HashMap::new();

        for rel in &self.declaration.relations {
            // Validation guarantees rel.from is declared; locus_values is complete.
            let current_from = locus_values.get(&rel.from).copied().unwrap_or(0.0);

            // Delta: difference from prior step's raw value for the source locus.
            let prior_from = self.step_state.as_ref()
                .and_then(|s| s.locus_values.get(&rel.from).copied());
            let delta = prior_from.map(|p| current_from - p);

            // Raw sigma: carry from step_state, then add this step's delta.
            let prior_raw_sigma = self.step_state.as_ref()
                .and_then(|s| s.relation_outputs.get(&rel.id))
                .map(|o| o.raw_sigma)
                .unwrap_or(0.0);
            let raw_sigma = prior_raw_sigma + delta.unwrap_or(0.0);

            // Gated sigma — operator-specific.
            // Validation guarantees interior_locus_id is declared and in locus_values.
            let sigma = match &rel.operator_path {
                OperatorPath::PrimaryDeltaSigma => raw_sigma,
                OperatorPath::ABR { interior_locus_id, gate_inverted } => {
                    let b = locus_values.get(interior_locus_id).copied().unwrap_or(0.0);
                    let gate = if *gate_inverted { 1.0 - b } else { b };
                    raw_sigma * gate
                }
            };

            relation_outputs.insert(rel.id.clone(), OperatorOutput { delta, raw_sigma, sigma });
        }

        // 3. Backward-compat fields — derived from the first Device-sourced relation.
        let (delta_thermistor, sigma_thermistor) = self.declaration.relations.iter()
            .find(|r| {
                self.declaration.locus(&r.from)
                    .map(|l| matches!(l.source, LocusSource::Device))
                    .unwrap_or(false)
            })
            .and_then(|r| relation_outputs.get(&r.id))
            .map(|o| (o.delta, o.raw_sigma))
            .unwrap_or((None, 0.0));

        Ok(RelationalState {
            source_observation: obs.clone(),
            delta_thermistor,
            sigma_thermistor,
            locus_values,
            relation_outputs,
        })
    }

    /// Returns a reference to the current prior state, if any.
    pub fn prior_state(&self) -> Option<&RelationalState> {
        self.prior_state.as_ref()
    }
}
