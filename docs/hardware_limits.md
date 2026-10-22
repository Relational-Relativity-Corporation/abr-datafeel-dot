# Hardware Limits — DataFeel Dot

**Status: PLACEHOLDER — not confirmed with DataFeel**

This file must be completed with DataFeel-specified values before any
physical actuation. The placeholder values in `src/actuator_projection.rs`
(`HardwareLimits::default()`) are intentionally conservative and are not
derived from DataFeel documentation.

## Required from DataFeel

| Parameter | Unit | Confirmed value | Source |
|---|---|---|---|
| Thermal power maximum | fraction of rated | _pending_ | DataFeel SDK / datasheet |
| Thermal operating range | °C | _pending_ | |
| Vibration amplitude maximum | fraction of rated | _pending_ | |
| Vibration frequency range | Hz | ~150–200 (from Matthew Leaper description) | verbal |
| Vibration pulse duration maximum | ms | _pending_ | |
| RGB intensity maximum | 0–255 per channel | 255 (assumed) | |
| Maximum continuous operating time | s | _pending_ | |
| Safe surface temperature range | °C | _pending_ | |

## Existing safety mechanisms

DataFeel's existing hardware safety mechanisms must be identified and
preserved independently of the experimental relational treatment. The
relational projection layer does not replace hardware-level protection.

## Update protocol

When DataFeel provides confirmed limits:
1. Update this table with values and sources.
2. Update `HardwareLimits::default()` in `src/actuator_projection.rs`.
3. Re-run `cargo test` to verify all projection bounds tests pass.
4. Record update in `docs/verification_record.md`.
