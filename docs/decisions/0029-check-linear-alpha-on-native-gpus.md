# 0029 — Check linear alpha on native GPUs before creating goldens

- **Status:** Accepted
- **Date:** 2026-09-06
- **Phase:** 1
- **Decided by:** agent, within the human-approved reference-first sequence
- **Affects:** rezie-gpu, diagnostic PNG assets

## Context

The RX 6800 XT ownership report passes 20,000 acquire/share/release cycles
with no additional texture/view creation during reuse. It does not execute
shaders. Phase 1 next needs deterministic PNG alpha over a colour source on
D3D12 and Metal before reference images are proposed for human review.

## Options

### A — Explicit boundary conversion and numerical diagnostic

Exercise GPU sRGB ingest, linear premultiplication, alpha-over and sRGB egress
separately. Read back both the working result and exported bytes, checking
against a double-precision CPU oracle. This detects cancelling conversion
errors that an exported image alone could hide.

### B — Start by approving screenshots from Metal

This would not validate the production backend, and would make an unreviewed
implementation its own oracle. Reject it.

## Decision

Use A. Working textures remain exclusively Rgba16Float. Packed RGBA8 buffers
exist only at PNG ingest/egress; diagnostic readback is not a CPU frame type
or a preview transport. Use separate compute entry points with portable 8×8
workgroups and explicit bounds checks. Use the standard sRGB transfer from
[CSS Color 4](https://www.w3.org/TR/css-color-4/#color-conversion-code).

All GPU creation belongs to a control-side FramePool method, including
diagnostic staging buffers, pipelines and command recording. The method
borrows the pool exclusively until submission/readback completes and retains
all frame leases through completion. Check texture plus staging payload
bytes against its budget; pipeline/driver-private memory is not measurable
by that payload budget. This synchronous diagnostic is never a composite
thread API and makes no steady-state allocation claim.

Expose the already locked pure-Rust `png = 0.18.1` as a workspace dependency
(MIT OR Apache-2.0). It adds no native SDK. Diagnostic fixtures are generated
from code, encoded as 8-bit sRGB RGBA PNG, then decoded before GPU ingest.
These initial fixtures are diagnostic inputs, not approved golden references.
The normative asset/golden harness remains a subsequent step in Phase 1.

## Consequences

Test opaque/transparent pixels (including hidden RGB), half-alpha over black
and white, alpha ramps, coloured backgrounds and translucent backgrounds.
Odd dimensions test dispatch bounds and readback row padding. Check raw linear
premultiplied channels within 0.002 absolute error and exported channels
within two 8-bit code values. These numerical diagnostic tolerances do not
replace SPEC §7.6's perceptual golden thresholds.

Reports include native adapter/backend, shader hash, per-case errors and PNG
outputs. Output directories must be new, so a rerun cannot erase evidence.
M4 output is development evidence only. The same diagnostic must run on RX
6800 XT before proposing production golden candidates. Human review remains
required before installing references. Decode, NDI and the five-minute
allocation gate remain later steps within Phase 1; no phase closure is claimed.

## Verification

Portable tests cover analytical midpoint/alpha cases and half-float decoding.
Run `rezie-colour-check` on M4/Metal and RX 6800 XT/D3D12, preserve reports
and inspect PNGs. GPU success is measured, never inferred from hosted builds.

## Revisit when

Integrating the streaming compositor: prepare reusable resources on the control
thread and submit without per-tick allocation or diagnostic readback. Extend
ingest metadata handling before accepting non-sRGB or non-RGBA8 PNG sources.
