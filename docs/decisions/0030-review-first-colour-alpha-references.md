# 0030 — Review the first Windows colour/alpha reference images

- **Status:** Proposed
- **Date:** 2026-09-06
- **Phase:** 1
- **Decided by:** awaiting human review
- **Affects:** initial golden references, rezie-gpu colour path

## Context

The owner committed the RX 6800 XT / D3D12 colour diagnostic in `92829e8`.
All five cases pass, checking 83,525 pixels. The independent
[audit](../testing/phase-1-colour-windows-audit.json) verifies the committed
PNG pixels against a separate double-precision oracle. Maximum exported error
is one 8-bit code value; reported raw linear error is at most 0.000959691.
The raw linear readbacks were not serialized, so that second statistic is
producer-reported rather than independently reconstructed.

All five Windows outputs have identical decoded RGBA bytes to M4. This is an
observation about this run, not a change to the perceptual golden policy.
The shader/probe/checker hashes match implementation `8659673`: Windows used
CRLF, M4 LF. No shader or checker edits are needed to explain the difference.

## Options

### A — Review and freeze the measured Windows outputs

Use the five original production-GPU PNGs as the first colour/alpha baseline,
identified by file and decoded-pixel hashes. Expand the suite as image sources,
preview and real media paths arrive during Phase 1.

### B — Regenerate a new set on the M4

Reject this: reference origin must remain the Windows production machine,
even when the current pixels happen to match Metal exactly.

## Proposed decision

Approve the **five output PNGs** linked in the
[review sheet](../testing/phase-1-golden-candidates.md) as the initial
colour/alpha golden pixel content. The input-alpha PNG is a generated input,
not a golden output. Approval is bound to the file SHA-256 values in that
sheet and the source revision above.

After approval, install byte-for-byte copies under
`tests/golden/phase-1/colour-alpha/` with source/report provenance. Do not
regenerate or substitute outputs during installation. Until then the images
remain candidates in `docs/testing/`; no reference files are installed.

## Consequences

The normative `xtask gen-assets`/`golden` harness is the next implementation
step. It must regenerate the input from code, render these scenes on the
reference machine, compare with SPEC §7.6's mean ΔE <1 / max ΔE <3, and write
actual/difference images on failure. Checking alpha must remain explicit;
transparent output cannot be validated by ignoring its alpha channel.
Approval of pixels does not constitute a passing golden comparison.

These small deterministic scenes cover colour conversion and alpha; they do
not establish streaming preview, decode, NDI, allocation or timing acceptance.
Phase 1 remains open. M4 remains a necessary correctness target, and hosted
GPU smoke cannot create or replace normative references.

## Verification

The numerical diagnostic passed on both native GPUs. Independently decoded
all committed input/output PNGs, verified dimensions, sRGB tags and hashes,
recomputed all exported RGBA expectations and checked the linear midpoint and
hidden-RGB anchors. Inspect the linked Windows images with the band legend.
Human approval and a subsequent reference-machine golden comparison are pending.

## Revisit when

Adding new rendering paths or changing expected pixels. Future reference
changes require the same explicit review; a failing test is not approval.
