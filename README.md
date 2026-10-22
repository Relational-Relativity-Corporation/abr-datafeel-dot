# abr-datafeel-dot

Relational multimodal control for the DataFeel Dot — Kernel V8 applied to thermal, haptic, and optical outputs as a single declared relational process.

**Metatron Dynamics, Inc. — Delaware C-Corp #10551645**

---

## What this does

The DataFeel Dot has three output modalities: thermoelectric heating/cooling, a ~170 Hz resonant haptic actuator, and RGB LEDs. In conventional operation each modality follows its own prescribed sequence.

This repository implements a relational alternative: the modalities participate in a common declared structure, where Kernel V8 operators evaluate directed differences and accumulation from the thermistor observable and produce coordinated actuator commands. The device does not need hardware modification.

## Repository structure

```
abr-datafeel-dot/
├── declarations/
│   └── m_declaration.json       # Origin-declared M (region, loci, relations)
├── src/
│   ├── lib.rs
│   ├── device_interface.rs      # DeviceInterface trait + MockDevice
│   ├── observation.rs           # Observation type with provenance tagging
│   ├── declaration.rs           # MDeclaration parser and validation
│   ├── relational_loop.rs       # Core loop: read → evaluate → project → command
│   └── actuator_projection.rs  # RelationalState → bounded device commands
├── bin/
│   └── datafeel_demo.rs         # Demo binary (MockDevice, Phases 1–4)
├── tests/
│   ├── conformance.rs           # Structural invariants
│   └── replay.rs                # Reproducibility tests
├── data/observations/           # Recorded physical runs (JSON Lines)
└── docs/
    ├── hardware_limits.md       # DataFeel hardware limits (PLACEHOLDER)
    ├── experiment_protocol.md   # First experiment protocol
    └── verification_record.md  # Running verification log
```

## Build and test (Phases 1–4 — no hardware required)

Requires a stable Rust toolchain (1.70+).

```
cargo build
cargo test
cargo run --bin datafeel_demo
```

All tests run against `MockDevice`. No DataFeel hardware is required for Phases 1–4.

## Build phases

| Phase | Description | Status |
|-------|-------------|--------|
| 1 | Hardware interface layer (trait + mock) | Buildable now |
| 2 | M declaration + relational topology | Buildable now |
| 3 | Kernel V8 integration (pinned dep) | Buildable now |
| 4 | Actuator projection | Buildable now |
| 5 | Real hardware integration | Blocked: awaiting DataFeel SDK |
| 6 | Comparative demonstration | Blocked: requires Phase 5 |

## Key invariants

- **Kernel V8 is a pinned dependency.** It is used unchanged; no operator logic is reproduced or modified in this repository.
- **Prior state is never initialised synthetically.** The first loop step runs with `prior_state = None`. State accumulates only from completed Physical observations.
- **MockFixture observations do not accumulate state.** They are structurally tagged and excluded from the verification record.
- **Every projection is explicit.** Which relational quantities map to which actuator commands, and which are discarded, is declared — not implicit behaviour.
- **Hardware limits gate physical actuation.** `docs/hardware_limits.md` must be completed with DataFeel-confirmed values before Phase 5 begins.

## What we need from DataFeel

- Can custom control logic execute on the Dot, or must it run on a connected host?
- Can we read thermistor measurements and send thermal, vibration, and RGB commands directly?
- What are the available sampling rates, command latencies, and hardware safety limits?
- Does the Dot expose any additional sensors or actuator feedback beyond thermistor and the three output modalities?
