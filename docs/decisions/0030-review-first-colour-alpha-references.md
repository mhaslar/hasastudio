# 0030 — Review the first Windows colour/alpha reference images

- **Status:** Accepted
- **Date:** 2026-09-06
- **Phase:** 1
- **Decided by:** human review, hashes bound to `c9397f0`
- **Affects:** initial golden references, rezie-gpu colour path

## Precision ruling before freezing hashes

The owner conditionally accepted the scene design, but **did not approve the
8-bit candidate hashes**. Use 16-bit-per-channel sRGB RGBA PNG outputs. This
preserves much more of the working-frame precision at negligible fixture size;
there is no concrete reason to freeze the 8-bit exports. Keep the RGBA8 input
and all band/background values unchanged so this is the same scene, not a new
stimulus. Export directly from the GPU working result at 16-bit precision;
expanding existing 8-bit bytes would preserve the precision loss.

Serialize each actual working readback as a tightly packed `.rgba16f.le` file:
row-major top-to-bottom, left-to-right, interleaved RGBA, IEEE binary16 in
little-endian order, 8 bytes/pixel, no row padding. Preserve bits straight from
GPU readback, not values reconstructed from the CPU oracle or PNG. Record file
path, byte count, dimensions, format and SHA-256 in the JSON. This retains all
working precision, including information a normalized PNG cannot represent.

Keep the existing 0.002 absolute working-channel bound. Separately check
16-bit GPU egress against sRGB encoding of the **observed** working readback,
within two 16-bit code values, and record its error against the ideal full
pipeline as an additional statistic. This separates half-float intermediate
rounding from egress quantization; the old two-8-bit-code-value limit is not
silently rescaled into a coarse 16-bit tolerance. Add an independent offline
auditor that recomputes both numerical claims from the PNG and raw files.

Rerun on M4 and Windows, preserve the old 8-bit reports, and re-propose the
Windows **16-bit PNG and raw readback hashes** for final approval. No references
may be installed on the strength of the old hashes or conditional approval.

## Context (original 8-bit run)

The owner committed the RX 6800 XT / D3D12 colour diagnostic in `92829e8`.
All five cases pass, checking 83,525 pixels. The independent
[audit](../testing/phase-1-colour-windows-audit.json) verifies the committed
PNG pixels against a separate double-precision oracle. Maximum exported error
is one 8-bit code value; reported raw linear error is at most 0.000959691.
The raw linear readbacks were not serialized, so that second statistic is
producer-reported rather than independently reconstructed.

All five Windows outputs have identical decoded RGBA bytes to M4. This is a
coincidence for these scenes, not a general expectation or a change to the
perceptual golden policy. Simple arithmetic on these deterministic inputs
rounded identically on D3D12 and Metal. Phase 3's bilinear/Lanczos sampling is
expected to expose backend precision differences; differing bytes then are
not themselves a defect. Judge numerical and perceptual bounds, not equality.
The shader/probe/checker hashes match implementation `8659673`: Windows used
CRLF, M4 LF. No shader or checker edits are needed to explain the difference.

## Verified 16-bit Windows rerun

The owner supplied the new reference run in `7f01657`, using implementation
`79113fd` (hashes verified with CRLF checkout). The
[independent audit](../testing/phase-1-colour16-windows-x86_64/audit.json)
reconstructs all 83,525 pixel checks from the PNG16 and raw binary16 files:
maximum linear error 0.000959691; maximum egress error one 16-bit code value.
The ideal-full-pipeline export error is recorded separately, maximum 193/65535.
No linear statistic now rests solely on the producer's summary.

The raw working samples still match M4 exactly for these scenes. However, the
[16-bit export comparison](../testing/phase-1-colour16-platform-comparison.json)
already finds 184 differing channel values out of 334,100, each one 16-bit step
apart. This locates the observed difference at egress, with both backends
within the two-code-value bound; it does not establish the specific driver or
arithmetic cause. The original 8-bit byte equality hid these differences.
Do not wait until Phase 3 to remember that byte equality is not the criterion.

## Options

### A — Review and freeze the measured Windows outputs

Use the five original production-GPU PNGs as the first colour/alpha baseline,
identified by file and decoded-pixel hashes. Expand the suite as image sources,
preview and real media paths arrive during Phase 1.

### B — Regenerate a new set on the M4

Reject this: reference origin must remain the Windows production machine,
even when the current pixels happen to match Metal exactly.

## Proposed decision

Propose the **five 16-bit output PNGs and their raw linear readbacks** from
Windows evidence `7f01657`, measured code `79113fd`, with exact new hashes in the
[review sheet](../testing/phase-1-golden-candidates.md) as the initial
colour/alpha golden pixel content. The input-alpha PNG is a generated input,
not a golden output. Approval is bound to the file SHA-256 values in that
sheet and the measured source revision. The linked manifest lists all ten
source paths, destination paths, lengths and SHA-256 values. The previous
8-bit set remains historical and is not approved.

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

These scenes perform a **single composite operation**. Repeated-blend error
accumulation through ten stacked overlays (SPEC §6.3) must be checked in Phase 4;
see [Phase 4 notes](../phases/04-notes.md). It is not implemented early here.

These small deterministic scenes cover colour conversion and alpha; they do
not establish streaming preview, decode, NDI, allocation or timing acceptance.
Phase 1 remains open. M4 remains a necessary correctness target, and hosted
GPU smoke cannot create or replace normative references.

## Verification

The numerical diagnostic passed on both native GPUs. Independently decoded
all committed input/output PNGs, verified dimensions, sRGB tags and hashes,
recomputed all exported RGBA expectations and checked the linear midpoint and
hidden-RGB anchors. Inspect the linked Windows images with the band legend.
The 16-bit images and exact raw readbacks are now audited and proposed with
new hashes. Final human approval has been received and the ten files are installed. A
subsequent reference-machine golden comparison has now passed (ADR 0031,
Windows evidence `e252a0e`). Remaining Phase 1 acceptance still applies.

## Revisit when

Adding new rendering paths or changing expected pixels. Future reference
changes require the same explicit review; a failing test is not approval.

## Approval and installation

The owner explicitly approved the five PNGs and five raw readbacks identified
in `c9397f0`. All ten were installed byte-for-byte under
`tests/golden/phase-1/colour-alpha/`; `approved.json` records the source/destination
paths and hashes. Every installed length and SHA-256 was checked. This installs
approved content, not a claim that the new golden harness has passed.
