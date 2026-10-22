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

/// Which Kernel V8 operator path applies to a relation.
///
/// A→B→R requires a declared interior locus grounded in a physical
/// observable. Primary Δ→Σ applies otherwise.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorPath {
    /// Primary difference and accumulation — no interior locus required.
    PrimaryDeltaSigma,
    /// Full relational path through a declared interior locus.
    ///
    /// The interior locus ID must resolve to a declared Locus with
    /// source = Device or Host (grounded observable).
    ///
    /// gate_inverted: when false, gated output = raw_sigma × B (direct).
    ///               when true,  gated output = raw_sigma × (1−B) (inverse).
    ///
    /// Use gate_inverted = true for profiles where the interior locus value
    /// is LOW during the relevant phase (e.g. rest profile, where circadian
    /// phase is low at night but the rest-mode output should be high).
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
    ///   exists and has source = Device or Host (not Actuator).
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

            // If A→B→R, the interior locus must exist and be Device- or Host-sourced.
            if let OperatorPath::ABR { interior_locus_id, .. } = &rel.operator_path {
                match locus_index.get(interior_locus_id.as_str()) {
                    None => {
                        return Err(DeclarationError::Inadmissible(format!(
                            "relation {} claims A→B→R path but interior locus '{}' is not declared",
                            rel.id, interior_locus_id
                        )));
                    }
                    Some(locus) if !matches!(locus.source, LocusSource::Device | LocusSource::Host) => {
                        return Err(DeclarationError::Inadmissible(format!(
                            "relation {} claims A→B→R path but interior locus '{}' is not Device-sourced \
                             or Host-sourced — an Actuator locus is not a grounded physical observable",
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
