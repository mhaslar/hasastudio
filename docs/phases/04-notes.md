# Phase 4 — Deferred validation notes

Planning only. Phase 4 is not started.

## Repeated-blend precision (SPEC §6.3; ADR 0030)

Phase 1's colour/alpha suite tests one composite operation. When overlay
channels exist, add a deterministic scene stacking **ten overlays** with
mixed opacity/colour over a known background. Retain every stage in linear
premultiplied Rgba16Float and compare the final raw linear readback against
an independent high-precision oracle. Record error after each layer so
accumulation is visible; set an explicit justified bound in the Phase 4 ADR,
not a tolerance chosen after observing a failure. Include transparent,
near-transparent and opaque layers and a discriminating linear-light blend.

Create/review the 16-bit PNG reference on the Windows RX 6800 XT, preserve
raw binary16 results, and also run the Metal correctness check. Apply the
normative perceptual limits as well as the numerical accumulation check.
Do not infer correctness of ten layers from Phase 1's single-operation pass.
Backend byte equality is not required. This scene belongs in Phase 4 when
overlays exist; no overlay implementation is authorized ahead of that phase.
