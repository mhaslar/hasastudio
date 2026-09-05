# 0021 — Assign production and correctness evidence to explicit platforms

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** human review
- **Affects:** SPEC §§0, 2, 13–14, AGENTS.md, phase gates, CI, benchmarks

## Context

The production machine is Windows 11 with an RX 6800 XT. The Apple M4 is a
development machine, not a production or performance target. The existing
Phase 0 ten-minute idle report was measured on M4; hosted correctness and
packaged GUI checks passed on all three operating systems at `fa59e28`
([run 33983882843](https://github.com/mhaslar/hasastudio/actions/runs/33983882843)).
GitHub still reports zero registered repository self-hosted runners.

## Options

### A — Treat supported platforms as interchangeable acceptance hardware

This permits Metal or a hosted software GPU to stand in for production
D3D12/Vulkan, and VideoToolbox to stand in for AMF. It conceals backend and
driver differences and makes performance comparisons invalid. Rejected.

### B — Separate production evidence from portable correctness

Tag every acceptance criterion now, keep hardware-specific correctness
coverage, and collect production performance and golden evidence on the
designated machine. Preserve a small, explicitly non-production Mac report.

## Decision

Adopt B, as directed by the owner:

| Target | Role and admissible evidence |
| --- | --- |
| Windows 11 / RX 6800 XT | Sole production reference. All normative performance numbers, benchmarks, soak runs and golden references originate here. Self-hosted runner labels: `self-hosted`, `rezie-reference`, with Windows routing. |
| Apple M4 / Metal | Development and light functional checks at 1–2 inputs. Never production. Correctness target, with the explicit reduced diagnostic benchmark below. |
| Windows / Linux / macOS CI | Build and correctness support. Hosted runners carry no performance criteria and provide no normative compositor evidence. |

Tag every §13 criterion `[Reference machine]`, `[Any platform]`, or `[CI]`.
Reference means the exact production host, not any Windows machine. Any
platform means portable correctness on a suitable supported platform; an
explicit list of platforms requires each listed platform. CI means supported
platform correctness pipelines, not that GPU-less hosted hardware can prove
hardware-specific behavior. Real GPU/codec/capture checks need appropriately
equipped CI workers; software fallback is a separate test, not evidence that
a hardware path ran. Missing workers must be reported, not treated as passes.

Golden-frame comparisons and reference generation are normative only on the
reference machine. Human review remains required for reference updates.
From Phase 1, an optional hosted lavapipe smoke check is non-blocking and must
never create/update golden references. Phase 0's pixel-free golden inventory
remains a blocking correctness check; it is not a compositor comparison.

No compositor or shader change passes a phase gate before running on the
reference machine, exercising the production D3D12/Vulkan backends shipped for that host.
Run affected paths on M4/Metal too: its success is necessary, never sufficient.
Check workgroup limits, texture-format support and precision behavior on the
actual backends, not merely successful shader compilation.

At every phase gate with a media pipeline (Phase 1 onward), record an M4
diagnostic benchmark: two inputs, one output, 1920×1080p50. It has no performance
threshold and cannot pass or fail production performance criteria. Record
configuration, backend, source/output types, driver/OS, toolchain, frame-time
distribution, drops and resource observations so regressions are visible.
Use only that phase's implemented source/output paths. Phase 0 has no pixel
inputs or media output, so this workload is inapplicable; do not implement
Phase 1 features to fabricate a Phase 0 two-input benchmark. This diagnostic
is the owner's explicit exception to reference-only benchmark collection.

Production encoding is AMF on Windows for H.264 and HEVC. From Phase 5,
hardware-encode acceptance and every encoder performance measurement require
AMF on the Windows reference machine. VAAPI/Linux and VideoToolbox/macOS
remain CI correctness targets with no performance criterion. Assert the
selected encoder and absence/fallback/error behavior; a passing VideoToolbox
test never satisfies production AMF acceptance. Codec licensing and the lack
of software HEVC remain unchanged.

## Consequences

The reference runner needs Windows 11, the RX 6800 XT, the AMD Adrenalin driver
stack (including AMF runtime), MSVC build tools/Windows SDK, the pinned Rust
toolchain and an interactive desktop for GUI checks. Record driver/backend
versions with results. Keep it otherwise idle for timing; serialize reference
runs and ensure no build or other workload overlaps the measured interval.
Hardware-dependent secondary CI coverage will also need suitable Linux/macOS
workers; the existing hosted matrix cannot prove VAAPI/VideoToolbox availability.

Amend SPEC §2, every §13 criterion, §14 and AGENTS.md to remove the obsolete
requirement that all golden/performance criteria pass on all hosted platforms.
Do not implement later-phase compositor, encoder or benchmark workloads now.

This policy supersedes ADR 0017's permission to use any idle local machine as
normative clock-latency evidence. Existing M4 clock samples remain valid
historical development measurements, not RX 6800 XT evidence. Do not delete,
relabel as production, or claim that the reference benchmark/soak ran.

Finishing slack becomes a per-platform setting. Sweep candidate values on M4
and Windows with the real native priority/wait path and preserve every trial's
samples, scheduling report and distribution. Record both sweeps here; the
prior M4 single-value pilot is not a sweep. Values remain explicitly provisional
until measured. Windows may require a larger value; do not relax the maximum
or percentile bound if correctly configured MMCSS cannot meet it. Linux is a
correctness target and carries no performance calibration claim.

The manual sweep uses 60 seconds per candidate at 50 Hz, in the fixed order
1.5, 0, 5.0, 0.5, 3.0, 1.0 ms to avoid simply increasing slack with elapsed time.
Zero slack is the sleep-only baseline; a custom candidate list allows refinement.
Preserve full per-tick samples and p50/p99/p99.9/max for every value. Measure
actual thread CPU time around finishing-spin segments (Unix thread CPU clock,
Windows GetThreadTimes), plus total clock-thread CPU time and spin wall time.
Report CPU nanoseconds and percentage of one core; elapsed spin wall time is
not CPU time. Profiling is enabled only for the sweep and is disclosed because
the CPU-clock queries add overhead. Ten-minute acceptance uses the selected
slack without per-spin profiling. No automatic selection or threshold waiver:
inspect the degradation curve and choose the smallest comfortably higher slack.
The M4 and Windows owners run the sweeps manually on otherwise idle machines.
At 50 Hz the computation budget is max(2 ms, slack + 0.5 ms) and the constraint is
computation + 1 ms; these actual budgets are recorded per value, and the
selected value uses the same budgeting rule in unprofiled acceptance. CPU
accounting overhead and budget changes must remain visible when interpreting
the curve. The commands record host/compiler/revision metadata and reject a
non-reference host for normative benchmark/soak, including manual execution.

POSIX failure examples in §13 (`SIGSTOP`, `kill -9`) use equivalent process
suspension/forcible termination on the Windows production host; keep the same
failure scenario and assertions.

## Verification

Audit every §13 acceptance clause for an explicit measurement tag; distinguish
numerical correctness tolerances from real-time performance bounds. Keep
hosted correctness blocking, and any future hosted compositor smoke advisory.
Require production evidence and the reduced Mac diagnostic at applicable gates.

**Phase 0 measurement pending:** the owner clarified the closure condition: its
newly reference-tagged clock bound has no reference report yet. Run the ten-minute clock benchmark on the designated host before closure. The owner clarified that this clock benchmark may be run
manually: runner provisioning and nightly automation do **not** block Phase 0.
No reference soak is required to close Phase 0. No M4-based exception was
granted. Until the manual reference clock result passes, retain the active
phase marker and do not create the completion summary.

A one-second-per-value functional tooling run found that the zero-slack
baseline's initial 0.5 ms computation budget did not read back as requested
on Mach (`realtime_error = 5`). That trial is not calibration evidence. Keep
the original proven 2 ms minimum computation budget for low-slack values,
with larger values receiving slack + 0.5 ms; record actual policy readback.
No inference about kernel clamping or production latency is made from this
short check. The corrected one-second functional sweep confirmed Mach time-constraint
readback at all six values, including zero. Its CSV/raw reports and SVG
rendering were checked; none of its measurements is calibration evidence.

### Slack calibration evidence

- M4: the owner supplied the six-value manual sweep on 2026-09-05, recorded
  in [the complete measurement directory](../benchmarks/phase-0-slack-sweep-macos-aarch64/metadata.json).
  It used clean revision `62186795d79dcb9cea06f6c1eeb1bed6a5d3241e`, Rust
  1.98.1, Apple M4 and macOS 27.0 (26A5416b). Each trial ran 60 seconds at
  50 Hz. All six reports contain 3,001 ticks, zero reported index/PTS errors,
  zero draining-sink drops, 2,999 stalled-sink drops, and confirmed Mach
  time-constraint scheduling. Recomputing nearest-rank quantiles from all
  18,006 raw lateness samples agrees with the reports and summary. CPU totals
  and percentages also agree. These are development calibration measurements;
  `latency_passed` remains null, not production acceptance.
- Windows 11 / RX 6800 XT: the owner explicitly deferred testing until later.
  The sweep and ten-minute idle acceptance remain pending. Hosted MMCSS
  correctness runs do not substitute for either; Phase 0 remains open.

Lateness is in microseconds. CPU percentages are relative to one core; spin
CPU seconds are actual measured CPU time over each 60-second trial.

| Slack (ms) | p50 (µs) | p99 (µs) | p99.9 (µs) | Max (µs) | Spin CPU (s) | Spin CPU (%) | Thread CPU (%) |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 31.875 | 57.291 | 103.583 | 4549.125 | 0.000 | 0.000 | 0.131 |
| 0.5 | 2.000 | 16.166 | 19.291 | 20.666 | 1.403 | 2.338 | 2.479 |
| 1 | 2.125 | 16.917 | 18.833 | 20.125 | 2.899 | 4.831 | 4.982 |
| 1.5 | 2.416 | 16.625 | 18.708 | 23.000 | 4.398 | 7.330 | 7.481 |
| 3 | 2.375 | 15.333 | 17.375 | 20.000 | 8.900 | 14.833 | 14.981 |
| 5 | 2.458 | 16.291 | 18.208 | 19.125 | 14.899 | 24.831 | 24.976 |

The [curve](../benchmarks/phase-0-slack-sweep-macos-aarch64/curve.svg) and
[CSV](../benchmarks/phase-0-slack-sweep-macos-aarch64/summary.csv) preserve the
comparison. In this sweep, 0.5 ms was the smallest tested positive slack and
already reached the low-lateness region. Higher slack increased CPU cost
without a clear latency benefit: 1.5 ms used 7.330% of one core for spin,
versus 2.338% at 0.5 ms. The zero-slack baseline had a 4.549125 ms maximum.
One 60-second trial per value cannot establish a long-run maximum or precisely
locate the degradation point between zero and 0.5 ms.

Keep 0.5 ms as a promising candidate, not a final calibrated constant. Before
pinning the smallest slack comfortably above degradation, refine the M4 sweep
at 0, 100, 250, 500, 750 and 1,000 µs (manual command in the calibration guide).
No runtime constant changes accompany this evidence commit. The existing
1.5 ms defaults remain explicitly provisional; no Windows value is inferred
from Metal/Mach measurements. The earlier M4 ten-minute report remains
historical development evidence at its original 1.5 ms configuration.

### Subsequent owner ruling (ADRs 0022–0023)

The owner selected macOS 0.5 ms immediately from this sweep. Finer sampling
is optional and does not block that choice. Windows remains uncalibrated and
normal startup must fail explicitly there; no Mac value is inherited. Linux
also has no calibrated default. Diagnostic tests supply explicit values.
The owner authorized conditional Phase 0 closure with its single Windows
reference obligation due at the Phase 1 gate. Earlier no-closure/provisional-Mac
statements above describe the preceding decision and are superseded to this
extent only. Production thresholds and measurement targets remain unchanged.

## Revisit when

Production hardware/OS changes, a second production target is proposed, or
hardware-backed secondary correctness workers cannot be supplied.
