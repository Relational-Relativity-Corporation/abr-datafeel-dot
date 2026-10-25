# abr-datafeel-dot

Relational multimodal control for the DataFeel Dot — Kernel V8 applied to thermal, haptic, and optical outputs as a single declared relational process.

**Metatron Dynamics, Inc. — Delaware C-Corp #10551645**

---

## What this implements

The DataFeel Dot has three output modalities: thermoelectric heating/cooling, a ~170 Hz resonant haptic actuator, and RGB LEDs. In conventional operation each modality follows its own prescribed sequence.

This repository implements a relational alternative: the modalities participate in a common declared structure, where Kernel V8 operators evaluate directed relational quantities from the thermistor observable and produce coordinated actuator commands.

**The current implementation runs actual frozen Kernel V8 operators from `irrs-kernel` at pinned commit `27b31ce`.** It is not a provisional simulation. The device interface is simulated using `MockDevice` — no physical DataFeel hardware is connected.

### Implemented

- `irrs-kernel` V8 execution: `operator_e` (Phase 1, step 0) and `operator_e_v5` (Phase 1 + Phase 2, step 1+), via pinned git dependency at authorized commit `27b31ce`.
- Declared thermal, haptic, and optical relationships expressed in `declarations/m_declaration_active.json` and `declarations/m_declaration_rest.json`.
- Observation → relational evaluation → actuator projection → command sequence, with provenance tagging (Physical / MockFixture).
- ABR operator gated by Origin-declared circadian schedules (L-activity and L-rest).
- Per-step sigma computed as: `sigma = r_persistence × gate` where `r_persistence` is the temporal rate of change in the E-field (E_current_t − E_prior_t) and `gate` is the declared schedule value at the current circadian phase.

### Not yet implemented

- Phase 5: Physical DataFeel hardware integration (blocked: awaiting DataFeel SDK and confirmed hardware limits).
- Phase 6: Comparative demonstration against the device's native control approach.
- IRRS admission: Kernel V8 execution is present, but independent IRRS admission has not been separately established for this application.

---

## Current verification status

| Criterion | Status |
|-----------|--------|
| Kernel V8 execution | **Present** — `irrs-kernel` rev `27b31ce` (pinned) |
| Test suite | **23/23 passing** — 19 conformance + 4 replay (run locally to confirm) |
| Device execution | **Simulated** — `MockDevice`, no physical hardware |
| Physical hardware validation | **OPEN** — Phase 5 not yet started |
| E-1 Session persistence | **OPEN** — `PersistenceState` is not persisted across process restarts |
| IRRS admission | **Not independently established** |
| Mathematical equivalence with prior provisional implementation | **Not asserted** — see `docs/kernel_execution_trace.md` |

### OPEN condition E-1

`PersistenceState` (the E-field snapshot carried between steps) is not persisted across process restarts. Each new `RelationalLoop` instance starts with `step_persistence = None`. Step 0 runs Phase 1 only; Phase 2 (R_persistence) is not available until step 1. This is explicitly documented and unresolved.

---

## Reproduction

Requires a stable Rust toolchain (rustc 1.70+).

```sh
# Clone the repository
git clone https://github.com/Relational-Relativity-Corporation/abr-datafeel-dot
cd abr-datafeel-dot

# Build (fetches irrs-kernel at rev 27b31ce from GitHub)
cargo build

# Run the test suite (no hardware required)
cargo test

# Run the circadian sweep demonstration (MockDevice, no hardware)
cargo run --bin datafeel_demo
```

All tests run against `MockDevice`. No DataFeel hardware is required.

### Expected test output

```
running 19 tests
test result: ok. 19 passed; 0 failed; ...   ← conformance

running 4 tests
test result: ok. 4 passed; 0 failed; ...    ← replay
```

### Expected demonstration output

The sweep prints peak actuator outputs for each circadian phase point (0.00 → 0.95, Δ = 0.05) across both the activity-associated and rest-associated profiles.

- **Activity profile (blue / daytime):** Output is non-zero between approximately 05:00 and 20:00 (L-activity schedule window), quiescent otherwise.
- **Rest profile (dim red / night):** Output is non-zero between approximately 19:00 and 07:00 (L-rest schedule window, wrapping across midnight), quiescent otherwise.
- **Co-activity map:** Profiles are complementary by design; no phase point produces CO_ACTIVE in both profiles simultaneously.

Numerical values reflect `sigma = r_persistence × gate` computed by Kernel V8. Because `r_persistence` is the per-step temperature difference (not accumulated across steps), output magnitudes are bounded by actual temperature change rates. See `docs/kernel_execution_trace.md` for a two-step numerical trace.

### Kernel dependency identification

```toml
irrs-kernel = { git = "https://github.com/Relational-Relativity-Corporation/abr-irrs", rev = "27b31ce" }
```

Kernel V8 operators are used unchanged. No operator logic is reproduced or modified in this repository.

---

## Repository structure

```
abr-datafeel-dot/
├── declarations/
│   ├── m_declaration_active.json  # Activity-associated profile (daytime, blue)
│   └── m_declaration_rest.json    # Rest-associated profile (night, dim red)
├── src/
│   ├── lib.rs
│   ├── device_interface.rs        # DeviceInterface trait + MockDevice
│   ├── observation.rs             # Observation type with provenance tagging
│   ├── declaration.rs             # MDeclaration parser and validation
│   ├── relational_loop.rs         # Core loop: read → evaluate → project → command
│   └── actuator_projection.rs     # RelationalState → bounded device commands
├── bin/
│   └── datafeel_demo.rs           # Demonstration binary (MockDevice, circadian sweep)
├── tests/
│   ├── conformance.rs             # Structural invariants (19 tests)
│   └── replay.rs                  # Reproducibility tests (4 tests)
└── docs/
    ├── hardware_limits.md          # DataFeel hardware limits (PLACEHOLDER — unconfirmed)
    ├── experiment_protocol.md      # First experiment protocol
    ├── verification_record.md      # Running verification log
    ├── kernel_execution_trace.md   # Mathematical execution trace (two-step evidence)
    └── datafeel_hardware_collaboration.md  # Hardware integration note for DataFeel
```

---

## Key invariants

- **Kernel V8 is a pinned dependency.** Used unchanged at authorized commit `27b31ce`; no operator logic is reproduced or modified here.
- **Prior state is never initialised synthetically.** The first loop step runs with `prior_state = None`. State accumulates only from completed Physical observations.
- **MockFixture observations do not update prior_state.** They are structurally tagged and excluded from the verification record.
- **Every projection is explicit.** Which relational quantities map to which actuator commands, and which are discarded, is declared — not implicit behaviour.
- **Hardware limits gate physical actuation.** `docs/hardware_limits.md` must be completed with DataFeel-confirmed values before Phase 5 begins.

---

## What we need from DataFeel (Phase 5 integration)

To connect physical hardware, the following information is required. See `docs/datafeel_hardware_collaboration.md` for detail.

- **Communication protocol / SDK** — how the Dot is addressed from a host process.
- **Thermistor** — data format, units (°C assumed), sampling rate, and noise characteristics.
- **Haptic actuator** — command interface, amplitude units, pulse duration units, and operating safety limits.
- **Optical (RGB)** — command interface, channel intensity range, and any duty-cycle constraints.
- **Thermal actuator** — command interface, power units, feedback mechanism (if any), and safety limits.
- **Actuator state readback** — whether measured (not just commanded) actuator state is available.

No specifications are assumed. Hardware limits in `docs/hardware_limits.md` are placeholders; they must be replaced with DataFeel-confirmed values before any physical actuation.
