# DataFeel Hardware Collaboration Note

**Date:** 2026-10-24  
**Repository:** `abr-datafeel-dot` at commit `39d9ea1`  
**Origin:** Robin Macomber, Metatron Dynamics, Inc.  
**Purpose:** Engineering handoff to DataFeel — what exists, what the interface points are, and what is needed to connect physical hardware.

---

## 1. What has been implemented

`abr-datafeel-dot` implements a relational multimodal control process for the DataFeel Dot using actual frozen Kernel V8 operators from `irrs-kernel` (pinned commit `27b31ce`).

The control loop:

1. **Read** — A thermistor observation is read from the device (currently simulated by `MockDevice`).
2. **Evaluate** — Kernel V8 operators (`operator_e` / `operator_e_v5`) compute relational quantities from the declared topology and the current observation.
   - Step 0: Phase 1 only (`operator_e`) — no prior persistence available.
   - Step 1+: Phase 1 + Phase 2 (`operator_e_v5`) — temporal rate of change (R_persistence) is computed as E_current_t − E_prior_t.
3. **Gate** — Origin-declared circadian schedules (L-activity, L-rest) modulate the output: `sigma = r_persistence × gate`.
4. **Project** — `ActuatorProjection` maps sigma to bounded device commands:
   - Thermal: oppose the measured temperature rate of change (−signum(delta) × scale)
   - Haptic: amplitude ∝ |sigma| / 3.0, duration ∝ |sigma| × base_ms
   - Optical (blue): intensity ∝ sigma / 10.0 × 200

The process is declared — the relational topology, schedules, and projection formulas are all in version-controlled JSON and source files, not buried in fixed coefficients.

---

## 2. Currently simulated device interface

The `DeviceInterface` trait (`src/device_interface.rs`) abstracts the physical device:

```rust
pub trait DeviceInterface: Send + Sync {
    fn read_thermistor(&self) -> Result<f32, DeviceError>;
    fn send_haptic_command(&self, cmd: &HapticCommand) -> Result<(), DeviceError>;
    fn send_optical_command(&self, cmd: &OpticalCommand) -> Result<(), DeviceError>;
    fn send_thermal_command(&self, cmd: &ThermalCommand) -> Result<(), DeviceError>;
}
```

`MockDevice` implements this trait using a pre-loaded sequence of f32 temperatures and a no-op actuator sink.

To connect physical hardware: implement `DeviceInterface` for a `DotDevice` struct that calls the actual DataFeel SDK. No other code changes are required; the relational loop calls only this trait.

---

## 3. Sensor integration point

**Thermistor** enters at `read_thermistor() -> Result<f32, DeviceError>`.

- Assumed units: °C.
- Assumed range: approximately 20–45 °C (skin contact range).
- Assumed call timing: once per loop step; the caller controls step rate.
- Provenance tag: `ObsProvenance::Physical` for real hardware; `MockFixture` for simulated. Only Physical observations update the `prior_state` record.

**What we need from DataFeel:**
- Confirmed units (°C, °F, raw ADC?).
- Sampling rate range and any required settling delay.
- Expected noise floor and resolution.
- Whether the device returns a reading when not in contact (open-circuit behavior).

---

## 4. Actuator command points

Three command structs reach the device per loop step:

```rust
pub struct HapticCommand  { pub amplitude: f32, pub duration_ms: u32 }
pub struct OpticalCommand { pub r: u8, pub g: u8, pub b: u8, pub timing_ms: u32 }
pub struct ThermalCommand { pub power: f32 }  // −1.0 (cool) to +1.0 (heat)
```

Current V0.5 output range at peak disturbance (sigma ≈ 0.4):
- Haptic: amplitude ≈ 0.133 (dimensionless 0–1), duration_ms = 12
- Optical blue: b = 8 (0–200 scale)
- Thermal power: ≈ −0.04 to +0.04 (opposing thermal drift)

**What we need from DataFeel:**
- **Haptic:** Command format; whether amplitude is 0–1.0 or requires a different scale; pulse duration encoding; maximum safe amplitude and pulse rate; carrier frequency confirmation (170 Hz assumed).
- **Optical:** Command format; whether channel intensity is 0–255 or another scale; maximum duty cycle; optical power output at maximum.
- **Thermal:** Command format; power encoding (fractions or watts or raw); safety limits (maximum sustained power, temperature cutoff); whether there is a readback from the thermal element.
- **Actuator state readback:** Can the device report measured (not just commanded) actuator state? If so, L-haptic and L-blue could eventually be updated with actual state instead of the current synthetic 0.0 (Phase 5+ work).

---

## 5. Proposed first hardware integration test

**Objective:** Confirm that the implemented control process produces observable device behavior from a real thermistor observation, with no claim of therapeutic effect.

**Setup:**
- Host: any computer that can run the compiled Rust binary and communicate with the DataFeel Dot.
- Device: one DataFeel Dot with SDK accessible.
- Duration: ≈ 2 minutes.

**Procedure:**
1. Implement `DeviceInterface` for `DotDevice` using the DataFeel SDK.
2. Run the relational loop at the nominal step rate (once per second suggested initially).
3. Apply gentle thermal contact to the device thermistor for 5–10 seconds, then release.
4. Observe whether haptic and optical outputs are non-zero during the warming phase and return to zero after release.
5. Record the raw thermistor readings, the computed sigma values, and the issued commands.

**Acceptance criterion:** The issued commands are non-zero during the warming transient and return to near-zero within 2–3 steps after contact is removed. No specific sensation or physiological outcome is claimed.

**Comparison with native control:** After the above, run the device in its native control mode under the same thermal stimulus. Record what commands are issued. This produces a side-by-side command trace — not a controlled experiment, but a baseline for discussion.

---

## 6. What this is and is not

This repository implements a software control process with mathematically declared structure. It is intended to facilitate a collaborative engineering conversation about what a relational approach to multimodal sensory delivery could look like in practice.

It is not:
- A claim that this approach is superior to DataFeel's existing control software.
- A validated therapeutic device.
- A complete integration — Phase 5 (hardware) has not started.

The purpose of sharing is to identify whether the DataFeel hardware can serve as a substrate for this kind of relational control, and if so, what the collaboration would look like.

---

Maintained by: Generator (Claude Sonnet 4.6) — 2026-10-24  
Origin: Robin Macomber, Metatron Dynamics, Inc.
