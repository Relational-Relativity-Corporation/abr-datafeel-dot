# Kernel Execution Trace — abr-datafeel-dot V0.5

**Commit:** `39d9ea1`  
**Kernel:** `irrs-kernel` rev `27b31ce` (frozen V8)  
**Date:** 2026-10-24  
**Profile:** Activity-associated (`m_declaration_active.json`)  
**Circadian phase:** 0.5 (noon — L-activity = 1.0, gate = 1.0)

---

## Purpose

This document records the actual mathematical execution path through Kernel V8 operators for two consecutive steps of the DataFeel Dot control loop. It is evidence of what is computed, not a claim about physiological effect.

This trace replaces the previous provisional accumulated-Σ implementation. The mathematical distinction is documented explicitly in §7.

---

## 1. Source observation and declared provenance

**Thermal disturbance sequence (from `datafeel_demo.rs`):**

```
[36.5, 36.5, 36.7, 37.0, 37.4, 37.8, 37.5, 37.1, 36.9, 36.7, 36.6, 36.5]
```

Units: °C.  
Provenance: `ObsProvenance::MockFixture` (no physical hardware).  
Each value is read by `MockDevice::read_thermistor()`, which returns values in sequence.

The trace below covers **Step 0** (thermistor = 36.5 °C) and **Step 1** (thermistor = 36.5 °C), followed by **Step 2** (thermistor = 36.7 °C) which produces the first non-zero R_persistence.

---

## 2. Relevant loci and relations from M

**Active declaration (`m_declaration_active.json`, `rho_base = 0.5`, `chi_0 = 1.0`):**

| Locus ID | Source | Role |
|----------|--------|------|
| L-thermal | Device (thermistor_c) | Kernel node, value = thermistor reading |
| L-circadian | Host (time_of_day_phase) | Not a kernel node; source for L-activity |
| L-activity | Origin (schedule from time_of_day_phase) | **Gate only** — not a kernel node |
| L-haptic | Actuator | Kernel node, value = 0.0 (see §8, OPEN) |
| L-blue | Actuator | Kernel node, value = 0.0 (see §8, OPEN) |

**Kernel node assignment (loci referenced in relation from/to, excluding Origin loci):**

| Node index | Locus ID | Value (step 0 / step 1 / step 2) |
|------------|----------|-----------------------------------|
| 0 | L-thermal | 36.5 / 36.5 / 36.7 |
| 1 | L-haptic | 0.0 / 0.0 / 0.0 |
| 2 | L-blue | 0.0 / 0.0 / 0.0 |

**Relations:**

| Relation | from | to | Operator | Interior (gate) |
|----------|------|----|----------|-----------------|
| R-01 | L-thermal (node 0) | L-haptic (node 1) | ABR | L-activity |
| R-02 | L-thermal (node 0) | L-blue (node 2) | ABR | L-activity |

**Edges:**

| Edge index | src node | tgt node | Provenance |
|------------|----------|----------|------------|
| 0 | 0 (L-thermal) | 1 (L-haptic) | Coupling |
| 1 | 0 (L-thermal) | 2 (L-blue) | Coupling |

---

## 3. Values passed into `operator_e` and `operator_e_v5`

**Common across all steps:**

```
rho_base = 0.5
chi_0    = 1.0
pairs    = []       (no cross-family coupling)
cc       = []       (no coupling coefficients)
EdgeProvenance: Coupling for both edges
```

**Step 0 — `operator_e` (Phase 1 only, no prior persistence):**

```
NodeField::new([[36.5, 0.0, 0.0]])   ← one component, three nodes
DeclaredRelations: 3 nodes, 2 edges [(0,1), (0,2)], both Coupling
```

Caller: `operator_e(&f, &declared_rel, &[], &[], 0.5, 1.0)`

**Step 1 — `operator_e_v5` (Phase 1 + Phase 2):**

```
NodeField::new([[36.5, 0.0, 0.0]])
PersistenceState: from_prior(e_current_field_step0)
                  e_current_field_step0.spatial[0][0] = 36.5
                  e_current_field_step0.spatial[0][1] = 36.5
```

Caller: `operator_e_v5(&f, &declared_rel, &[], &[], &persistence_state, 0.5, 1.0)`

**Step 2 — `operator_e_v5`:**

```
NodeField::new([[36.7, 0.0, 0.0]])
PersistenceState: from step 1, e_prior.spatial[0][0] = 36.5, [0][1] = 36.5
```

---

## 4. Calculated `e_current`, `r_persistence`, `gate`, and `sigma`

**Topology class:** Isolated. For the declared edges, there are no shared adjacency nodes between L-thermal and L-haptic (or L-blue). The kernel's ρ-terms are zero for isolated edges, giving:

```
E_current[edge] = A_node_value − R_node_value
```

where A_node = L-thermal (thermistor_c) and R_node = L-haptic or L-blue (both 0.0).

### Step 0

| Field | Edge 0 (R-01, haptic) | Edge 1 (R-02, blue) |
|-------|----------------------|---------------------|
| e_current | 36.5 | 36.5 |
| r_persistence | **None** (no prior) | **None** |
| gate | 1.0 (L-activity at noon) | 1.0 |
| **sigma** | **0.0** | **0.0** |

`step_persistence` after step 0: `PersistenceState::from_prior(e_field_step0)`  
E-field stored: `[36.5, 36.5]` (one per edge, component 0)

### Step 1

Thermistor = 36.5 °C (no change from step 0).

| Field | Edge 0 (R-01, haptic) | Edge 1 (R-02, blue) |
|-------|----------------------|---------------------|
| e_current | 36.5 | 36.5 |
| r_persistence | 36.5 − 36.5 = **0.0** | 0.0 |
| gate | 1.0 | 1.0 |
| **sigma** | **0.0** | **0.0** |

`step_persistence` after step 1: E-field = `[36.5, 36.5]`

### Step 2

Thermistor = 36.7 °C (first temperature change in sequence).

| Field | Edge 0 (R-01, haptic) | Edge 1 (R-02, blue) |
|-------|----------------------|---------------------|
| e_current | 36.7 | 36.7 |
| r_persistence | 36.7 − 36.5 = **0.2** | 0.2 |
| gate | 1.0 | 1.0 |
| **sigma** | **0.2** (as f32) | **0.2** |

`step_persistence` after step 2: E-field = `[36.7, 36.7]`

**Peak sigma in 12-step sequence** occurs at step 4 and step 5 (temperature rise 37.0 → 37.4 → 37.8):

```
step 4: r_persistence = 37.4 − 37.0 = 0.4, sigma = 0.4
step 5: r_persistence = 37.8 − 37.4 = 0.4, sigma = 0.4
```

---

## 5. Values passed into actuator projection

`ActuatorProjection::project(&state, &decl)` reads `state.relation_outputs[rel.id].sigma` for each relation.

**Step 2 inputs to projection:**

```
state.delta_thermistor = Some(0.2)        ← R_persistence for R-01 (f32)
state.sigma_thermistor = 0.2              ← sigma for R-01 (f32)

relation_outputs["R-01"].sigma = 0.2
relation_outputs["R-02"].sigma = 0.2

decl.haptic_envelope.amplitude_scale = 1.0
decl.haptic_envelope.duration_ms_base = 30
```

---

## 6. Resulting commands

**Step 2 actuator commands (thermistor step 36.5 → 36.7, sigma = 0.2):**

```
Thermal:
  delta = 0.2 °C (positive → warming)
  thermal_power = −signum(0.2) × (0.2 / 5.0).min(1.0) = −0.04
  ThermalCommand { power: −0.04 }   ← oppose the warming

Haptic (R-01 → L-haptic):
  raw_amp = (0.2 / 3.0).min(1.0) × 1.0 (amplitude_scale) = 0.0667
  haptic_amplitude = 0.0667
  raw_dur = (0.2 × 30) as u32 = 6 ms
  HapticCommand { amplitude: 0.0667, duration_ms: 6 }

Optical / blue (R-02 → L-blue):
  b_sum = (0.2 / 10.0).clamp(0, 1) = 0.02
  blue = (0.02 × 200) as u8 = 4
  OpticalCommand { r: 0, g: 0, b: 4, timing_ms: 100 }
```

**Peak commands in 12-step sequence** (sigma = 0.4 at steps 4–5):

```
Haptic: amplitude = (0.4 / 3.0) × 1.0 = 0.133, duration_ms = 12
Optical: blue = (0.4 / 10.0) × 200 = 8
```

These match the demonstration output at phase = 0.5:
```
0.50  12:00  0.133  12  (0,0,8)  partial
```

Co-activity flag: **partial** (haptic 0.133 > 0.10 threshold, but blue = 8 < 10 optical threshold).

---

## 7. Persistence state carried into the next step

After each step, `RelationalLoop.step_persistence` is set to:

```rust
PersistenceState::from_prior(e_current_field)
```

This stores the current E-field for use as the prior E-field in the next call to `operator_e_v5`. It is not part of `RelationalState` (which must be Clone; `EdgeField` does not derive Clone).

E-field values stored after step 2: `[36.7, 36.7]` (edge 0, edge 1, component 0).

---

## 8. Mathematical distinction from prior implementation

The previous provisional implementation (`evaluate_stub`) computed:

```
raw_sigma += delta_from_prior_step    # accumulated Σ across all steps
sigma = raw_sigma × gate
```

The current Kernel V8 implementation (`evaluate_kernel`) computes:

```
r_persistence = E_current_t − E_prior_t    # per-step rate of change
sigma = r_persistence × gate
```

These are **not mathematically equivalent.** The prior implementation's `raw_sigma` grew across steps (unbounded accumulation). The current implementation's `sigma` is bounded by the actual per-step temperature change rate. Mathematical equivalence between the two is not asserted and has not been verified.

As a direct consequence: in the V0.5 demonstration, peak `sigma ≈ 0.4` (largest single-step temperature difference in the disturbance sequence), yielding `haptic_amplitude ≈ 0.133` and `optical_blue = 8`. This falls below the CO_ACTIVE optical threshold (> 10) and produces a **partial** co-activity flag across all activity-window phase points. Whether the prior provisional implementation reached CO_ACTIVE has not been compared.

---

## 9. OPEN conditions in this execution path

**E-1 — Session persistence:** `PersistenceState` is not persisted across process restarts. Each new `RelationalLoop` starts with `step_persistence = None`. Step 0 cannot compute R_persistence; sigma is 0 on the first step regardless of thermistor value. This is disclosed and unresolved.

**Actuator node values — synthetic zero (Phase 5+ deferred):** L-haptic and L-blue are declared as `LocusSource::Actuator` but are always passed to the kernel as 0.0, regardless of the actual last-commanded actuator value. This avoids dimensional mismatch (thermistor °C vs haptic amplitude 0–1) but means the kernel's R-node values do not reflect actual actuator state. As a result, `E_current[edge] = thermistor_c − 0.0 = thermistor_c` for all edges. This is disclosed; correction is deferred to Phase 5.

**IRRS admission:** Kernel V8 execution is present. Independent IRRS admission for this application has not been separately established.

---

Maintained by: Generator (Claude Sonnet 4.6) — 2026-10-24  
Origin: Robin Macomber, Metatron Dynamics, Inc.
