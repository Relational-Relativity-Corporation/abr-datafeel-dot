# Experiment Protocol — DataFeel Dot Relational Demo

**Status: DRAFT — to be finalised before Phase 5**

## Objective

Demonstrate that the DataFeel Dot can coordinate thermal, haptic, and optical
outputs in response to an observed thermal disturbance, using Kernel V8
relational evaluation rather than three independently prescribed sequences.

## First experiment

1. Place the Dot in contact with an instrumented test surface at stable
   ambient temperature.
2. Introduce an external thermal disturbance (e.g., brief contact with a
   surface at a known temperature offset).
3. Record the complete Dot response under both conditions:
   - **Condition A:** Existing DataFeel controller.
   - **Condition B:** Relational implementation (this repository).
4. For Condition B, log must include: thermistor trace, operator intermediate
   values (Δ, Σ per step), and all actuator command sequences.

## Independent instrumentation

The Dot cannot independently measure its own actuator output. At least one
external instrument is required to verify that commanded actuator states
correspond to physical device behaviour.

- Thermal: external thermocouple or IR thermometer on the contact surface.
- Haptic: accelerometer or audio recording (170 Hz component).
- Optical: photodiode or camera capture.

## Conformance check before physical actuation

Before connecting to hardware, run:

```
cargo test --release
```

All tests in `tests/conformance.rs` and `tests/replay.rs` must pass.
`docs/hardware_limits.md` must be completed with DataFeel-confirmed values.
`declarations/m_declaration.json` must carry Origin's final signature.

## Recording format

Observations are recorded as JSON Lines in `data/observations/`.
Each line is a serialised `Observation`. The replay tests in `tests/replay.rs`
load these files to verify reproducibility.
