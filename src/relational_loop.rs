// relational_loop.rs
//
// RelationalLoop — core loop: read → observe → evaluate → project → command.
//
// Kernel V8 integration: evaluate_kernel runs the frozen irrs-kernel V8
// operators (operator_e for Phase 1 / operator_e_v5 for Phase 1+2) against
// the declared relational topology. Pinned to authorized commit 27b31ce.
//
// KEY INVARIANTS:
//
//   prior_state — NEVER initialised synthetically.
//   The first step runs with prior_state = None. prior_state accumulates
//   only from completed Physical observations. MockFixture observations do
//   not update prior_state under any circumstances.
//
//   step_persistence — advances from ALL steps (Physical and MockFixture),
//   providing E-field continuity for R_persistence computation. Parallels
//   step_state in purpose; distinct from prior_state because MockFixture
//   observations must not corrupt the Physical verification record.
//
// OPEN CONDITIONS (E-1 preserved):
//   E-1  Session persistence — PersistenceState is not persisted across
//        process restarts. Each new RelationalLoop instance starts without
//        prior persistence (step_persistence = None, step 0 = Phase 1 only).
//        This is an acknowledged open condition. Do not close without separate
//        authorisation.

use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

use irrs_kernel::operators::{
    DeclaredRelations, EdgeProvenance, NodeField,
    PersistenceState, operator_e, operator_e_v5,
};

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
/// Missing entries for Host or Origin loci are not silently substituted.
/// The loop returns `LoopError::MissingHostObservable` if a required host
/// observable is absent from this map.
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

/// Output from one Kernel V8 operator evaluation for a single relation.
///
/// e_current    — Phase 1 E-field value for this edge (spatial snapshot).
///                None on step 0 (impossible: kernel always returns a value;
///                kept Option for structural consistency).
/// r_persistence — Phase 2 temporal rate of change in E-field (E_t - E_{t-1}).
///                None on step 0 (no prior E-field to compare against).
/// gate         — ABR interior locus value (B), or 1.0 for PrimaryΔΣ.
/// sigma        — Gated output used by projection: r_persistence × gate.
///                0.0 on step 0 (r_persistence is None). f32 for projection compat.
#[derive(Debug, Clone, Default)]
pub struct OperatorOutput {
    /// Phase 1: E-field value for this edge.
    pub e_current: Option<f64>,
    /// Phase 2: R_persistence for this edge. None on step 0.
    pub r_persistence: Option<f64>,
    /// Gate value: activity/rest schedule value (ABR) or 1.0 (PrimaryΔΣ).
    pub gate: f64,
    /// Gated output used by projection. 0.0 on step 0.
    pub sigma: f32,
}

// ---------------------------------------------------------------------------
// Relational state
// ---------------------------------------------------------------------------

/// Output from one evaluation of the relational process.
///
/// Backward-compatible fields (delta_thermistor, sigma_thermistor) are
/// preserved: delta_thermistor now carries R_persistence for the first
/// Device-source relation; sigma_thermistor carries its gated sigma.
#[derive(Debug, Clone)]
pub struct RelationalState {
    /// The observation that produced this state.
    pub source_observation: Observation,
    /// R_persistence for the first Device-source relation, if persistence is
    /// available (None on step 0). Backward-compat field.
    pub delta_thermistor: Option<f32>,
    /// Gated sigma for the first Device-source relation. 0.0 on step 0.
    /// Backward-compat field.
    pub sigma_thermistor: f32,
    /// Current raw value of each declared locus. Key: locus ID.
    pub locus_values: HashMap<String, f32>,
    /// Per-relation kernel operator outputs. Key: relation ID.
    /// The projection reads these to determine actuator driving values.
    pub relation_outputs: HashMap<String, OperatorOutput>,
}

/// Full output of one loop step.
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
    /// MockFixture observations do not update prior_state under any circumstances.
    prior_state: Option<RelationalState>,

    /// Most recent step state, regardless of provenance.
    /// Provides per-step continuity for locus_values (not used in kernel path,
    /// retained for structural consistency and potential future use).
    step_state: Option<RelationalState>,

    /// E-field persistence from the most recent step (all provenances).
    /// Used to compute R_persistence (Phase 2) on the next step.
    /// None until the first step completes (E-1 open condition).
    step_persistence: Option<PersistenceState>,

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
            step_persistence: None,
            actuator_state: ActuatorState::default(),
        }
    }

    /// Execute one step of the relational loop.
    ///
    /// 1. Read thermistor from device.
    /// 2. Construct Observation (Physical or MockFixture per provenance).
    /// 3. Evaluate Kernel V8 operators via evaluate_kernel.
    /// 4. Project relational output to actuator commands.
    /// 5. Dispatch commands to device.
    /// 6. Update persistence and state fields.
    ///
    /// `host_obs` supplies values for Host-sourced loci declared in the M
    /// (e.g. `time_of_day_phase`) and the clock source for Origin loci.
    /// A missing entry for any required Host or Origin observable returns
    /// `LoopError::MissingHostObservable`.
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

        // 3. Evaluate — Kernel V8.
        let (state, new_persistence) = self.evaluate_kernel(&observation, host_obs)?;

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

        // 6. step_persistence advances from ALL steps (including MockFixture)
        //    to maintain E-field continuity for R_persistence on the next step.
        //    step_state advances similarly.
        //    prior_state advances only on Physical — this is the invariant.
        self.step_persistence = Some(new_persistence);
        self.step_state = Some(state.clone());
        if observation.provenance.is_physical() {
            self.prior_state = Some(state.clone());
        }

        Ok(StepOutput { state, commands, observation })
    }

    /// Evaluate Kernel V8 operators against the declared relational topology.
    ///
    /// Step 0 (step_persistence = None): runs operator_e (Phase 1 only).
    ///   E_current[e] = thermistor - actuator_target (for isolated topology).
    ///   R_persistence is not available; sigma = 0.0 for all relations.
    ///
    /// Step 1+ (step_persistence = Some): runs operator_e_v5 (Phase 1 + Phase 2).
    ///   R_persistence[e] = E_current_t - E_prior_t (temporal rate of change).
    ///   sigma = R_persistence × gate (ABR interior locus value or 1.0).
    ///
    /// Kernel topology:
    ///   Nodes: loci referenced as from/to in relations, excluding Origin loci
    ///          (Origin loci are gates only — not part of the spatial topology).
    ///   Edges: one per relation, all with EdgeProvenance::Coupling.
    ///   Actuator nodes: always 0.0 (Phase 5+ deferred — avoids dimensional
    ///          mismatch between thermistor °C and haptic amplitude 0–1).
    ///
    /// Returns (RelationalState, PersistenceState). Caller stores PersistenceState
    /// in self.step_persistence for the next step. PersistenceState is not stored
    /// in RelationalState because EdgeField (which it contains) does not derive Clone.
    fn evaluate_kernel(
        &self,
        obs: &Observation,
        host_obs: &HostObservables,
    ) -> Result<(RelationalState, PersistenceState), LoopError> {
        // 1. Resolve locus values.
        let mut locus_values: HashMap<String, f32> = HashMap::new();
        for locus in &self.declaration.loci {
            let val = match &locus.source {
                LocusSource::Device   => obs.thermistor_c,
                LocusSource::Actuator => 0.0, // Phase 5+: use actual actuator state
                LocusSource::Host => {
                    host_obs.get(&locus.observable)
                        .copied()
                        .ok_or_else(|| LoopError::MissingHostObservable(locus.observable.clone()))?
                }
                LocusSource::Origin(schedule) => {
                    let phase = host_obs.get(&schedule.derives_from)
                        .copied()
                        .ok_or_else(|| LoopError::MissingHostObservable(schedule.derives_from.clone()))?;
                    schedule.value_at(phase)
                }
            };
            locus_values.insert(locus.id.clone(), val);
        }

        // 2. Build kernel topology.
        //    Nodes: loci referenced in relation from/to, excluding Origin loci.
        //    Origin loci are gates only — excluded from the spatial edge graph.
        let referenced: std::collections::HashSet<&str> = self.declaration.relations.iter()
            .flat_map(|r| [r.from.as_str(), r.to.as_str()])
            .collect();
        let node_loci: Vec<_> = self.declaration.loci.iter()
            .filter(|l| referenced.contains(l.id.as_str())
                && !matches!(l.source, LocusSource::Origin(_)))
            .collect();
        let node_map: HashMap<&str, usize> = node_loci.iter().enumerate()
            .map(|(i, l)| (l.id.as_str(), i))
            .collect();
        let n_nodes = node_loci.len();

        // Build edges from relations.
        let mut edges: Vec<(usize, usize)> = Vec::new();
        let mut edge_provenance: Vec<EdgeProvenance> = Vec::new();
        let mut rel_to_edge: HashMap<&str, usize> = HashMap::new();
        for rel in &self.declaration.relations {
            if let (Some(&src), Some(&tgt)) = (
                node_map.get(rel.from.as_str()),
                node_map.get(rel.to.as_str()),
            ) {
                let idx = edges.len();
                edges.push((src, tgt));
                edge_provenance.push(EdgeProvenance::Coupling);
                rel_to_edge.insert(rel.id.as_str(), idx);
            }
        }

        // NodeField: one component, n_nodes values. Actuator nodes always 0.0.
        let node_vals: Vec<f64> = node_loci.iter()
            .map(|l| locus_values.get(&l.id).copied().unwrap_or(0.0) as f64)
            .collect();
        let f = NodeField::new(vec![node_vals]);
        let declared_rel = DeclaredRelations::from_edges_with_provenance(
            n_nodes, edges, edge_provenance,
        );
        let pairs: &[(usize, usize)] = &[];
        let cc: &[f64] = &[];
        let rho_base = self.declaration.rho_base;
        let chi_0   = self.declaration.chi_0;

        // 3. Run kernel: Phase 1 only on step 0, Phase 1+2 on step 1+.
        let (e_current_field, r_persistence_vals) = match &self.step_persistence {
            None => {
                // Step 0: no prior E-field — Phase 1 only.
                let ec = operator_e(&f, &declared_rel, pairs, cc, rho_base, chi_0);
                (ec, None::<Vec<f64>>)
            }
            Some(ps) => {
                // Step 1+: Phase 1 + Phase 2 (persistence).
                let (ec, pout) = operator_e_v5(&f, &declared_rel, pairs, cc, ps, rho_base, chi_0);
                let n_edges = declared_rel.n_edges();
                let r_p: Vec<f64> = (0..n_edges)
                    .map(|e| pout.r_persistence[0][e])
                    .collect();
                (ec, Some(r_p))
            }
        };

        // 4. Build per-relation operator outputs.
        let mut relation_outputs: HashMap<String, OperatorOutput> = HashMap::new();
        for rel in &self.declaration.relations {
            let edge_idx = rel_to_edge.get(rel.id.as_str()).copied();
            let e_current = edge_idx.map(|e| e_current_field.spatial[0][e]);

            // Gate: ABR interior locus value (schedule-derived), or 1.0 for PrimaryΔΣ.
            let gate = match &rel.operator_path {
                OperatorPath::PrimaryDeltaSigma => 1.0_f32,
                OperatorPath::ABR { interior_locus_id, gate_inverted } => {
                    let b = locus_values.get(interior_locus_id).copied().unwrap_or(0.0);
                    if *gate_inverted { 1.0 - b } else { b }
                }
            };

            // r_persistence and sigma: only available from step 1+.
            let (r_persistence, sigma) = match (edge_idx, &r_persistence_vals) {
                (Some(e), Some(r_p)) => {
                    let rp = r_p[e];
                    (Some(rp), (rp as f32) * gate)
                }
                _ => (None, 0.0_f32),
            };

            relation_outputs.insert(rel.id.clone(), OperatorOutput {
                e_current,
                r_persistence,
                gate: gate as f64,
                sigma,
            });
        }

        // 5. Backward-compat fields: derived from the first Device-source relation.
        let (delta_thermistor, sigma_thermistor) = self.declaration.relations.iter()
            .find(|r| {
                self.declaration.locus(&r.from)
                    .map(|l| matches!(l.source, LocusSource::Device))
                    .unwrap_or(false)
            })
            .and_then(|r| relation_outputs.get(&r.id))
            .map(|o| (o.r_persistence.map(|v| v as f32), o.sigma))
            .unwrap_or((None, 0.0));

        let state = RelationalState {
            source_observation: obs.clone(),
            delta_thermistor,
            sigma_thermistor,
            locus_values,
            relation_outputs,
        };

        Ok((state, PersistenceState::from_prior(e_current_field)))
    }

    /// Returns a reference to the current prior state, if any.
    pub fn prior_state(&self) -> Option<&RelationalState> {
        self.prior_state.as_ref()
    }
}
