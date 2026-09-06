# 0031 — Compare approved native colour goldens with explicit alpha

- **Status:** Accepted
- **Date:** 2026-09-06
- **Phase:** 1
- **Decided by:** agent within the approved golden-harness implementation
- **Affects:** xtask, rezie-colour-check, portable comparator tests

## Context

ADR 0030's ten Windows-origin reference files are approved. SPEC §7.6 requires
regenerated assets, native rendering, mean ΔE <1 and max ΔE <3, and failure
images. It does not name a ΔE variant or define alpha comparison. Ignoring
alpha would permit invisible RGB or opaque-output errors to escape detection.

## Options

### A — CIEDE2000 plus explicit alpha and two display backgrounds

Decode full 16-bit channels, compare rendered appearance over black and white
in linear light, then CIELAB/D65 with CIEDE2000. Also compare alpha numerically.
This checks translucent content over both extremes without treating transparent
hidden RGB as visible colour. Preserve raw linear diagnostics separately.

### B — Byte comparison or RGB-only comparison

Reject: backend rounding can differ and RGB-only comparison omits alpha.

## Decision

Use A. Each case must satisfy strict mean ΔE00 <1 and max ΔE00 <3 on **both**
backgrounds. Require maximum normalized alpha error ≤0.002, matching the
existing working-channel bound; report alpha mean/max independently. Retain
raw working comparison with the approved readback (maximum absolute error
≤0.002) as an additional numerical check, plus the renderer's ideal-pipeline
and observed-egress checks. These checks are cumulative, not substitutes.

Use the standard CIEDE2000 equations with unit parametric factors; validate
against published numerical pairs from
[Sharma, Wu and Dalal](https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/).
Use sRGB transfer/XYZ conversion and D65 reference white for Lab. Compositing
onto black/white happens only in the offline metric, not the engine pipeline.

Add the already approved pure-Rust `png` workspace dependency to xtask. No
new package or native dependency. `gen-assets` generates the existing exact
RGBA8 fixture from code. `golden` passes that PNG explicitly to the renderer;
input byte hashes prevent quietly changing the approved stimulus. Additional
asset scenes arrive with their consuming Phase 1 paths, not later-phase keying.

Default `golden` requires Windows 11 / RX 6800 XT / D3D12 identity, renders
fresh results, and records adapter/OS/backend and renderer/source hashes.
`--development` explicitly permits M4/Metal comparison but cannot produce a
normative pass. No hosted GPU success is inferred. `--output` names a new run
directory; default runs get unique target directories. Report JSON includes
all metrics and provenance. Failures preserve actual 16-bit PNGs/raw samples
and opaque colour-difference/alpha-difference images under
`target/golden-failures/<run>/`, including numerical diagnostic failures.
Reference hashes are validated before rendering. No command silently updates
references; future updates require a separately reviewed, hash-bound proposal.

## Consequences

Tests must reject wrong colours, alpha-only errors and tampered reference
files, preserve low bits of PNG16, check strict threshold boundaries, and
exercise failure artifacts. Tests using recorded data need no GPU. The
independent auditor can check historical source via an explicit git revision
so adding CLI plumbing does not invalidate previously committed evidence.
Actual normative golden success still requires a new Windows reference run.
The current control-side renderer is a diagnostic; steady-state integration,
preview, decode/fallback and NDI remain subsequent Phase 1 work.

## Verification

Run portable metric/PNG/provenance/failure tests, workspace checks and the
M4 development command. Then run the default command on the reference machine
and commit its report. Reference approval alone is not this test result.

## Revisit when

New scenes, sampling or overlays expose an inadequate comparator. Any
reference change requires review; any change to SPEC thresholds requires an ADR
and human decision. Phase 4 tracks error versus repeated-blend count.

## Native reference result

The owner supplied the fresh Windows run in `e252a0e`, measured implementation
`c915210`. [The audit](../testing/phase-1-golden-windows-x86_64/audit.json)
verifies all five renders (83,525 pixels) against the approved files: zero
mean/max ΔE00, zero alpha difference, zero raw-linear difference. Windows 11
build 26200, RX 6800 XT / D3D12 and driver 32.0.21045.5002 are recorded. Source
and reference-manifest hashes match the Windows CRLF checkout. The independent
renderer audit reconstructs the separate ideal/egress metrics from raw samples.
This passes this native golden inventory, not the remaining Phase 1 gate.

Git displayed some `.rgba16f.le` files as text. Explicit binary attributes
protect all raw readbacks and PNGs against newline conversion and text merges;
no approved file bytes are changed by this protection.
