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

## Observations to carry into the repeated-blend suite

The 16-bit Phase 1 run exposed 184 differing exported channel values between
D3D12 and Metal, each one 16-bit step, despite identical raw working samples.
At eight bits rounding hid this difference. Preserve the precision and judge
numerical/perceptual bounds; cross-backend byte identity is not an invariant.

The single-composite baseline reaches 193/65535 exported error against the
ideal pipeline, under one 8-bit code value. This reflects binary16 precision
(11 significant bits including the implicit leading bit) amplified by the
sRGB transfer in dark values, not a currently observed blend defect. Keep this
full-pipeline error distinct from the one-code-value egress-only error.

Measure both raw linear and full-pipeline export errors against blend count
1 through 10. The owner's expected trend is roughly linear accumulation;
report faster-than-linear growth early and investigate ordering/grouping,
precision and the blend path before accepting it. Finite-precision rounding
already limits exact associativity, so the growth curve is diagnostic evidence,
not by itself proof of a particular cause. Do not hide the trend in one final
maximum or change bounds after a failure. This is Phase 4 work, not new work now.
