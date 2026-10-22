# Verification Record — abr-datafeel-dot

## Scientific basis — V0.2 profile declarations

The following published observations ground the locus declarations in
`m_declaration_active.json` and `m_declaration_rest.json`. These are
documented biological associations, not claims that the DataFeel Dot
produces the described physiological effects at the doses and conditions
used in cited experiments.

### Optical — blue channel (~466 nm), activity-associated profile

- Brainard GC et al. (2001). "Action spectrum for melatonin regulation in
  humans: evidence for a novel circadian photoreceptor." *J Neurosci*
  21(16):6405–6412. Peak sensitivity of the melatonin suppression response
  near 464 nm. 466 nm is within this range.
- Thapan K et al. (2001). "An action spectrum for melatonin suppression:
  evidence for a novel non-rod, non-cone photoreceptor system in humans."
  *J Physiol* 535(Pt 1):261–267. Confirms short-wavelength sensitivity of the
  circadian photoreception pathway.

### Optical — red channel (~626 nm), rest-associated profile

- Hamblin MR (2016). "Shining light on the head: Photobiomodulation for brain
  disorders." *BBA Clin* 6:113–124. Reviews cytochrome c oxidase as the
  primary photoacceptor for PBM in the 600–900 nm range.
- Karu TI (2010). "Mitochondrial signaling in mammalian cells activated by red
  and near-IR radiation." *Photochem Photobiol* 84(5):1091–1099. Documents the
  photochemical basis of the 600–700 nm therapeutic window.

### Haptic — vibrotactile (170 Hz)

- Bolanowski SJ et al. (1988). "Four channels mediate the mechanical aspects of
  touch." *J Acoust Soc Am* 84(5):1680–1694. Documents Pacinian corpuscle
  sensitivity range. Pacinian units are broadly tuned across 40–800 Hz, with
  peak sensitivity near 200–300 Hz; 170 Hz is within the activation range.

### Circadian context — biological timing and tissue repair

- Hoyle NP et al. (2017). "Circadian actin dynamics drive rhythmic fibroblast
  mobilization during wound healing." *Sci Transl Med* 9(415):eaal2774.
  Fibroblast migration into wounds varied with circadian phase. Retrospective
  analysis of human burn records found an association between daytime injury and
  faster healing than nighttime injury. Observational, not evidence that changing
  device light exposure reproduces the difference.
- Adrover JM et al. (2020) and subsequent reviews (see 2026 Frontiers review
  cited in project notes): circadian timing organises immune-cell recruitment,
  inflammatory resolution, and tissue remodeling.

### Thermal — skin temperature and sleep onset

- Van Someren EJ (2006). "Mechanisms and functions of coupling between sleep and
  temperature." *Prog Brain Res* 153:309–324. Distal skin temperature rises at
  sleep onset as core body temperature decreases; this coupling is implicated in
  sleep initiation. The rest profile boundary conditions (20–37 °C) reflect the
  physiological range associated with this transition, not a specific therapeutic
  target temperature.

---

## Phase 1–4: Software Conformance

| Date | Action | Result | Notes |
|------|--------|--------|-------|
| 2026-10-21 | Initial scaffold (V0.1) | 17/17 tests pass | Phases 1–4 buildable |
| 2026-10-21 | V0.2: host locus, two profiles, comparative demo | 17/17 pass | cargo test conformance + replay |
| 2026-10-21 | V0.3: ABR operator, circadian gate, multimodal resonance sweep | 17/17 pass | cargo test conformance + replay; demo sweep output verified |
| 2026-10-21 | V0.4: Origin-declared schedules, CoActive rename, MissingHostObservable error, midnight continuity | 23/23 pass | 19 conformance + 4 replay; 20-step sweep verified; no simultaneous CO_ACTIVE (schedules complementary by design); evaluator provisional simulation label confirmed |

## Phase 5: Hardware Integration

_Pending DataFeel SDK and confirmed hardware limits._

## Phase 6: Comparative Demonstration

_Pending Phase 5 completion._

---

Maintained by: Robin Macomber, Metatron Dynamics, Inc.
