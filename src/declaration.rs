// declaration.rs
//
// MDeclaration — parses declarations/m_declaration.json and holds the
// declared relational topology for the DataFeel Dot application.
//
// Origin declares the region, observable extents, loci, and relations.
// The parser enforces admissibility rules structurally: a relation claiming
// the A→B→R operator path without a grounded interior locus is rejected.

use std::path::Path;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum DeclarationError {
    #[error("failed to read declaration file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse declaration JSON: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("admissibility violation: {0}")]
    Inadmissible(String),
}

// ---------------------------------------------------------------------------
// Operator path
// ---------------------------------------------------------------------------

/// Which operator path applies to a relation.
///
/// These correspond to Kernel V8 operator definitions; evaluation is
/// provided by the provisional simulation in relational_loop.rs pending
/// the abr-kernel crate integration (Phase 3).
///
/// A→B→R requires a declared interior locus grounded in a physical
/// observable or an Origin-computed schedule. Primary Δ→Σ applies otherwise.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorPath {
    /// Primary difference and accumulation — no interior locus required.
    PrimaryDeltaSigma,
    /// Full relational path through a declared interior locus.
    ///
    /// The interior locus ID must resolve to a declared Locus with
    /// source = Device, Host, or Origin (grounded or schedule-derived).
    ///
    /// gate_inverted: when false, gated output = raw_sigma × B (direct).
    ///               when true,  gated output = raw_sigma × (1−B) (inverse).
    ///
    /// For V0.4 and later: prefer declaring an Origin locus whose schedule
    /// expresses the correct phase preference directly, keeping gate_inverted
    /// false. Reserve gate_inverted for cases where a single interior locus
    /// must drive complementary relations with opposing gate polarity.
    #[serde(rename = "abr")]
    ABR {
        interior_locus_id: String,
        #[serde(default)]
        gate_inverted: bool,
    },
}

// ---------------------------------------------------------------------------
// Haptic envelope
// ---------------------------------------------------------------------------

/// Haptic pulse envelope parameters for a declared profile.
///
/// The DataFeel Dot operates at 170 Hz as the mechanical carrier.
/// Profiles differentiate through amplitude scale and pulse duration,
/// not carrier frequency — carrier is fixed at the actuator's rated resonance.
///
/// These are experimental sensory configurations.
/// They are not established therapeutic prescriptions.
/// The 150–200 Hz operating range may be swept once physical actuator
/// measurements are available.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HapticEnvelope {
    /// Mechanical carrier frequency in Hz. 170.0 for the DataFeel Dot.
    pub carrier_hz: f32,
    /// Base pulse width in milliseconds, before amplitude scaling.
    /// Activity profile: shorter (assertive bursts).
    /// Rest profile:     longer (gentle, sustained).
    pub duration_ms_base: u32,
    /// Amplitude scale factor applied to the projected operator output.
    /// Activity profile: 1.0 (full scale).
    /// Rest profile:     0.5 (reduced to avoid arousal-level activation).
    pub amplitude_scale: f32,
}

impl Default for HapticEnvelope {
    fn default() -> Self {
        // Neutral defaults — used when declaration omits haptic_envelope.
        Self {
            carrier_hz: 170.0,
            duration_ms_base: 50,
            amplitude_scale: 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Boundary conditions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryConditions {
    /// Operating temperature range in °C [min, max].
    pub temperature_range_c: [f32; 2],
    /// Vibration frequency range in Hz [min, max].
    pub vibration_hz: [f32; 2],
    /// Optical wavelength range in nm [min, max].
    pub optical_nm: [f32; 2],
}

// ---------------------------------------------------------------------------
// Origin schedule
// ---------------------------------------------------------------------------

/// Declared schedule for an Origin-sourced locus.
///
/// An Origin locus represents a profile function computed deterministically
/// from a host-supplied clock observable (e.g. time_of_day_phase) and a
/// declared trapezoidal schedule. It is grounded in the host clock, not
/// synthetically constructed within the relational loop.
///
/// The schedule defines a cyclic trapezoidal window over normalised circadian
/// phase [0.0, 1.0] where 0.0 = midnight = 1.0. The window:
///   - rises linearly from 0 → 1 between rise_start and peak_start
///   - holds at 1.0 between peak_start and peak_end
///   - falls linearly from 1 → 0 between peak_end and fall_end
///   - is 0.0 between fall_end and rise_start (zero region)
///
/// Wrapping schedules (where rise_start > fall_end) are handled correctly:
/// the zero region is the daytime gap, and the active window spans midnight.
/// Use these for rest profiles whose peak occurs around 00:00.
///
/// All phase values must be in [0.0, 1.0). The ordering constraint is:
/// rise_start → peak_start → peak_end → fall_end (modulo 1.0, cyclic).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OriginSchedule {
    /// Name of the host observable this schedule derives from.
    /// Must be supplied by the host on every loop step (e.g. "time_of_day_phase").
    pub derives_from: String,
    /// Phase at which the profile value begins rising from 0.0.
    pub rise_start: f32,
    /// Phase at which the profile value reaches 1.0 (start of plateau).
    pub peak_start: f32,
    /// Phase at which the profile value begins falling from 1.0 (end of plateau).
    pub peak_end: f32,
    /// Phase at which the profile value reaches 0.0 (end of active window).
    pub fall_end: f32,
}

impl OriginSchedule {
    /// Evaluate the declared profile value at the given normalised circadian phase.
    ///
    /// Returns a value in [0.0, 1.0] using a cyclic trapezoidal window.
    /// Phase is treated modulo 1.0 — values 0.0 and 1.0 produce identical output,
    /// ensuring continuity across midnight.
    pub fn value_at(&self, phase: f32) -> f32 {
        let phase = phase.rem_euclid(1.0);

        // Shift coordinates so rise_start maps to 0.0.
        // rem_euclid handles the wrap-across-midnight case without branch logic.
        let shifted = (phase - self.rise_start).rem_euclid(1.0);

        let rise_width  = (self.peak_start - self.rise_start).rem_euclid(1.0);
        let peak_width  = (self.peak_end   - self.peak_start).rem_euclid(1.0);
        let fall_width  = (self.fall_end   - self.peak_end  ).rem_euclid(1.0);
        let total_active = rise_width + peak_width + fall_width;

        if shifted < rise_width {
            if rise_width < f32::EPSILON { 1.0 } else { shifted / rise_width }
        } else if shifted < rise_width + peak_width {
            1.0
        } else if shifted < total_active {
            let into_fall = shifted - rise_width - peak_width;
            if fall_width < f32::EPSILON { 0.0 } else { 1.0 - into_fall / fall_width }
        } else {
            0.0
        }
    }
}

// ---------------------------------------------------------------------------
// Locus
// ---------------------------------------------------------------------------

/// Observable source — where the locus value comes from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocusSource {
    /// Reading from a device sensor (thermistor, etc.).
    Device,
    /// Last-commanded actuator value (not an independent sensor reading).
    Actuator,
    /// Value from the host system / environment (e.g. time_of_day_phase).
    /// Treated as grounded for ABR admissibility — the host is not a physical
    /// sensor on the device, but its observables are not synthetically constructed
    /// within the relational loop itself.
    Host,
    /// Value computed by the framework from a declared schedule and a host
    /// clock observable. Grounded in the host-supplied time value; the schedule
    /// is the Origin's declaration of what the value means in context.
    ///
    /// Origin loci are admissible as ABR interior loci because their value
    /// is determined by a declared rule applied to a grounded clock observable,
    /// not by the relational loop's own output.
    Origin(OriginSchedule),
}

/// A declared locus — a specific observable quantity in the declared region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Locus {
    pub id: String,
    /// The name of the observable this locus tracks.
    pub observable: String,
    pub source: LocusSource,
    /// Scientific or physiological basis for this locus.
    /// Documents the established association between the observable and
    /// the biological or physical process it represents.
    /// Not a claim about device efficacy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub basis: Option<String>,
}

// ---------------------------------------------------------------------------
// Relation
// ---------------------------------------------------------------------------

/// A declared directed relation between two loci.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: String,
    /// Source locus ID.
    pub from: String,
    /// Target locus ID.
    pub to: String,
    /// Operator path — validated against declared loci on load.
    pub operator_path: OperatorPath,
    /// Optional description of the relational coupling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// MDeclaration
// ---------------------------------------------------------------------------

fn default_rho_base() -> f64 { 0.5 }
fn default_chi_0()    -> f64 { 1.0 }

/// The full declared M for the DataFeel Dot application.
/// Parsed from declarations/m_declaration.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MDeclaration {
    /// Plain-language description of the declared physical region.
    pub region: String,
    pub boundary_conditions: BoundaryConditions,
    pub loci: Vec<Locus>,
    pub relations: Vec<Relation>,
    /// Haptic pulse envelope — differentiates profiles at same carrier frequency.
    /// Defaults to neutral values when omitted.
    #[serde(default)]
    pub haptic_envelope: HapticEnvelope,
    /// Origin statement — who declared this M and when.
    pub provenance: String,
    /// Domain-specific ρ ceiling declared by M.
    /// Dimensionless; constrains the kernel's E-field normalisation.
    /// Required by irrs-kernel operator_e. Default: 0.5.
    #[serde(default = "default_rho_base")]
    pub rho_base: f64,
    /// Domain-specific reference scale χ₀ declared by M.
    /// Units match the source locus (°C for thermal). Default: 1.0.
    #[serde(default = "default_chi_0")]
    pub chi_0: f64,
}

impl MDeclaration {
    /// Load and validate an M declaration from a JSON file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, DeclarationError> {
        let content = std::fs::read_to_string(path)?;
        let decl: MDeclaration = serde_json::from_str(&content)?;
        decl.validate()?;
        Ok(decl)
    }

    /// Validate admissibility rules:
    /// - Every relation's `from` and `to` must resolve to a declared locus.
    /// - A relation using OperatorPath::ABR must name an interior locus that
    ///   exists and has source = Device, Host, or Origin (not Actuator).
    /// - No locus ID is duplicated.
    pub fn validate(&self) -> Result<(), DeclarationError> {
        // Check for duplicate locus IDs.
        let mut seen = std::collections::HashSet::new();
        for locus in &self.loci {
            if !seen.insert(&locus.id) {
                return Err(DeclarationError::Inadmissible(format!(
                    "duplicate locus id: {}", locus.id
                )));
            }
        }

        let locus_index: std::collections::HashMap<&str, &Locus> =
            self.loci.iter().map(|l| (l.id.as_str(), l)).collect();

        for rel in &self.relations {
            // Resolve from/to.
            if !locus_index.contains_key(rel.from.as_str()) {
                return Err(DeclarationError::Inadmissible(format!(
                    "relation {} references unknown locus: {}", rel.id, rel.from
                )));
            }
            if !locus_index.contains_key(rel.to.as_str()) {
                return Err(DeclarationError::Inadmissible(format!(
                    "relation {} references unknown locus: {}", rel.id, rel.to
                )));
            }

            // If A→B→R, the interior locus must exist and be Device-, Host-, or Origin-sourced.
            if let OperatorPath::ABR { interior_locus_id, .. } = &rel.operator_path {
                match locus_index.get(interior_locus_id.as_str()) {
                    None => {
                        return Err(DeclarationError::Inadmissible(format!(
                            "relation {} claims A→B→R path but interior locus '{}' is not declared",
                            rel.id, interior_locus_id
                        )));
                    }
                    Some(locus) if !matches!(
                        locus.source,
                        LocusSource::Device | LocusSource::Host | LocusSource::Origin(_)
                    ) => {
                        return Err(DeclarationError::Inadmissible(format!(
                            "relation {} claims A→B→R path but interior locus '{}' is not \
                             Device-, Host-, or Origin-sourced — an Actuator locus is not \
                             a grounded observable",
                            rel.id, interior_locus_id
                        )));
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    /// Look up a locus by ID.
    pub fn locus(&self, id: &str) -> Option<&Locus> {
        self.loci.iter().find(|l| l.id == id)
    }

    /// Look up a relation by ID.
    pub fn relation(&self, id: &str) -> Option<&Relation> {
        self.relations.iter().find(|r| r.id == id)
    }
}
