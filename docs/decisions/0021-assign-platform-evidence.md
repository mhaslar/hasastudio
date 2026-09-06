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

### Windows evidence audit — 2026-09-06

Commit `20a1759627a918d4a91b5fb5123db0a5450dc5d3` contains the
[six Windows trials](../benchmarks/phase-0-slack-sweep-windows-x86_64/metadata.json),
measured at clean `c891d86` with Rust 1.98.1. A fresh remote fetch confirms
there is **no committed ten-minute Windows report or host sidecar**. This is
**incomplete evidence**, not acceptance and not a demonstrated latency failure.
Phase 0 remains conditionally closed; its single obligation stays OPEN. No
Windows slack is pinned and no Phase 1 implementation accompanies this audit.

Each trial contains 3,001 lateness samples (18,006 total), in the recorder's
index-addressed order for indices 0–3,000. Independently recomputed nearest-rank
p50/p99/p99.9/max, last-sample drift, maximum/deadline-miss counters, CPU totals
and CPU percentages agree with the reports and summary. Counts agree across
expected/received/emitted/samples. The final FrameTime is index 3,000 / PTS
60 seconds. The harness reports zero index and exact-rational-PTS errors,
zero draining-sink drops and 2,999 stalled-sink drops in every trial.
Individual observed FrameTime records are not serialized: ordering/exact PTS
are checked live by the harness, rather than independently replayable from
these JSON files. Raw lateness is not a sorted percentile-only export.

Every measured thread reports `policy: MmcssProAudio`, `realtime: true`,
`realtime_error: null`, `timer_resolution_ms: 1`, `timer_error: null`. The source
at the recorded revision sets these fields only after a non-null
AvSetMmThreadCharacteristicsW("Pro Audio") handle and a successful
(timeBeginPeriod(1) == 0) request. The guard retains both until after sampling.
This confirms successful API application from recorded status, not an inference
from low lateness or a claim of independently probed physical timer precision.

Host metadata identifies Windows 11 Pro 10.0.26200 (build 26200), Intel
Core i5-14600K (14 cores / 20 logical processors), RX 6800 XT driver
32.0.21045.5002 and 34,102,353,920 bytes of memory. **Power plan is unknown**:
neither Balanced nor High Performance is recorded. There is no contemporaneous
idle/utilization record or operator idle statement. A clean worktree and the
tool's settling delay do not prove idle operation. The original metadata
collector did not collect these fields; do not invent them retrospectively.

Windows lateness below is in microseconds. CPU is measured per 60-second
trial and percentages refer to one core; these are calibration observations,
not ten-minute acceptance. `latency_passed` is null in all six files.

| Slack (ms) | p50 (µs) | p99 (µs) | p99.9 (µs) | Max (µs) | Final (µs) | Spin CPU (s) | Spin CPU (%) | Thread CPU (%) |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 319.600 | 635.400 | 785.900 | 996.500 | 105.300 | 0.000000 | 0.000 | 0.104 |
| 0.5 | 0.600 | 126.500 | 212.800 | 366.700 | 0.800 | 0.000000 | 0.000 | 1.484 |
| 1 | 0.600 | 1.300 | 9.400 | 30.700 | 0.400 | 0.500000 | 0.833 | 2.500 |
| 1.5 | 0.700 | 2.800 | 45.500 | 57.100 | 0.900 | 1.156250 | 1.927 | 5.260 |
| 3 | 0.700 | 2.400 | 13.700 | 23.700 | 0.700 | 5.750000 | 9.583 | 12.109 |
| 5 | 0.700 | 1.800 | 10.200 | 41.500 | 0.500 | 12.000000 | 20.000 | 23.099 |

The smallest tested Windows value in the low-tail region is **1 ms**, a
candidate for a documented idle rerun, not a calibrated default. At 0.5 ms,
p99.9/max rise to 212.8/366.7 µs from 9.4/30.7 µs at 1 ms. Larger slack has
no consistent tail benefit (the 1.5 ms trial is worse than 1 ms). All short
trials happen to be below 5 ms / 20 ms; this does not establish ten-minute
acceptance or justify choosing the sleep-only baseline.

The Windows 0.5 ms trial reports **zero spin CPU** despite 2,621 finishing-spin
entries and 573.6806 ms of spin wall time. GetThreadTimes returns units of
100 ns but has OS accounting granularity; summing short per-segment differences
can lose CPU attribution. Treat 0% as the recorded counter result, not free
spinning. Whole-thread CPU is 1.484% there and 2.500% at 1 ms. At 1 ms the
spin counter reports 0.833% (0.5 CPU seconds); do not claim it is directly as
accurate as the M4's thread-clock measurement. Preserve both counters.

For a like-duration comparison, M4 at its selected 0.5 ms gives
p50/p99/p99.9/max 2.000/16.166/19.291/20.666 µs and 2.338% spin / 2.479%
whole-thread CPU. Windows at candidate 1 ms gives
0.600/1.300/9.400/30.700 µs and recorded 0.833% spin / 2.500% whole-thread CPU.
These short trials differ in host, scheduling and accounting; Windows idle and
power-plan evidence is missing. The historical M4 ten-minute run at **1.5 ms**
has p50/p99/p99.9/max 1.500/16.625/18.250/36.292 µs, final 1.167 µs;
**there is no Windows ten-minute number to place beside it**.

Repeat the Windows six-value sweep with recorded power configuration and idle
telemetry, then collect the ten-minute unprofiled run at the reviewed **1,000 µs
candidate override** if the repeated curve still supports it. Keep original
files unchanged. Exact commands and expected evidence are in
[the rerun guide](../user/windows-clock-rerun.md). The unchanged gate requires
30,001 ticks with zero skipped indices/exact PTS, final drift and max lateness
strictly below 20 ms, p99.9 strictly below 5 ms, and confirmed native scheduling.
A genuine failed acceptance run reopens Phase 0 and stops Phase 1; missing
measurement is not relabeled a scheduler defect.

### Windows v2 audit — numerical pass, INADEQUATE idle evidence

The owner supplied commit `5282c5f`, merged as `d030d76`, measured from
`da52572` / Rust 1.98.1 on the same Windows build 26200, i5-14600K and RX
6800 XT (driver 32.0.21045.5002). Source status reports only evidence-file
changes/deletions, not runtime source changes. The old Windows sweep removed
by the v2 evidence commit is restored byte-for-byte from `20a1759`; historical
measurements and links must survive replacement runs.

**Ruling: INADEQUATE**, not a numerical timing failure. The sole Phase 0
obligation remains OPEN; Phase 0 is still conditionally closed and Windows
slack stays unset. No Phase 1 implementation resumes in this slice.

[The 600-second report](../benchmarks/phase-0-idle-windows-x86_64.json) has
30,001 raw lateness samples. Expected, received and emitted counts agree,
index/PTS error counters are zero, the final frame is index 30,000 / PTS
600 seconds, and sink drops are 0 / 29,999. Recomputed nearest-rank
p50/p99/p99.9/max are **0.200 / 0.900 / 25.500 / 128.500 µs**; final drift
is **0.300 µs**, with no 20 ms deadline misses. These meet every unchanged
numeric bound. Profiling is disabled. Raw individual PTS were not serialized
by that revision: the existing PTS claim rests on the live harness checks.
New reports retain actual received PTS under ADR 0027; never backfill v2.

All six v2 sweep reports independently validate at 3,001 samples each, with
matching percentiles/counts/final sample, zero index/PTS errors and matching
CPU summaries. Every sweep and acceptance thread reports MmcssProAudio,
realtime true, no realtime error, timer_resolution_ms 1, no timer error, and
the requested slack. Those fields require successful native API application.
The 1 ms acceptance override uses the same 2/3 ms computation/constraint
budget as its sweep. No threshold has been relaxed.

#### Power and idle evidence

Both [sweep](../benchmarks/windows-sweep-idle-evidence-v2/operator-notes.txt)
and [acceptance](../benchmarks/windows-acceptance-idle-evidence-v2/operator-notes.txt)
power-plan records show GUID `8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c`
(High Performance / Vysoký výkon) before and after. The owner confirms it
remained active throughout. A specific plan is now preferred, not mandatory:
any recorded plan is admissible if idle evidence supports the run. An
unavailable power setting is not grounds for failure.

The completed monitors retained 79 sweep and 122 acceptance snapshots, at
roughly five-second intervals (maximum gaps 11.52 / 10.56 seconds). Across
the complete monitor windows, formatted total CPU averages **10.91% / 11.23%**,
peaking at **23% / 24%**. In the approximate actual acceptance window it is
still **11.19%**, not just startup activity. Microsoft defines total Processor
Time as average usage across processors; these are machine percentages, not
one-core percentages. The sampled integers are not a continuous utilization
trace, and the current collector stores no raw counters to independently
check its formatted CPU figures.

Observed per-process CPU deltas over the monitor windows include:

| Process group | Sweep CPU seconds | Acceptance CPU seconds |
| --- | ---: | ---: |
| Taskmgr | 27.469 | 42.297 |
| Code | 10.297 | 15.281 |
| rezie-headless | 25.750 | 33.141 |
| powershell | 2.141 | 3.281 |
| msedge | 0.797 | 0.859 |

The notes say only Edge and VS Code were open in the background; Task Manager
is also active throughout. This is a discrepancy, not proof that the operator
was actively using it. About 169 process CPU fields are null in the first
acceptance snapshot, including System, MsMpEng/Defender and dwm. Readable
process deltas cannot account for the reported total CPU load. Null is not
zero. This record neither proves an otherwise idle host nor identifies the
missing competing workload. It would be wrong to assume all 11% is harmless
OS activity or the monitor, or to diagnose a specific service from this data.

Record raw total CPU and per-process performance counters in an elevated
monitor before rerunning; reconcile the load, close Task Manager and pause
identified competing work. A short preflight diagnoses telemetry/background
load first; it is not a new latency threshold. Repeat the sweep and acceptance
with fresh paths and observed PTS only when idle operation is supportable.
[Exact commands](../user/windows-clock-rerun.md) retain all prior evidence.

#### Windows v2 curve alongside the M4 curve above

Lateness is µs; CPU percentages are of one core and have the Windows accounting
limitations below. Each row is one 60-second profiled trial, not acceptance.

| Slack (ms) | p50 | p99 | p99.9 | Max | Spin CPU (s) | Spin CPU (%) | Thread CPU (%) |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 301.900 | 588.900 | 757.100 | 967.900 | 0.000000 | 0.000 | 0.000 |
| 0.5 | 0.400 | 95.400 | 253.100 | 408.500 | 0.000000 | 0.000 | 0.312 |
| 1 | 0.500 | 1.200 | 4.800 | 6.000 | 0.546875 | 0.911 | 3.203 |
| 1.5 | 0.500 | 1.200 | 3.500 | 27.600 | 1.656250 | 2.760 | 5.781 |
| 3 | 0.500 | 1.400 | 19.600 | 24.000 | 6.140625 | 10.234 | 14.271 |
| 5 | 0.500 | 1.400 | 7.200 | 66.400 | 12.109375 | 20.182 | 22.812 |

Both Windows sweeps put 1,000 µs in the low-tail region before degradation at
500 µs. Retain it as the reviewed rerun candidate, not a pin under inadequate
idle evidence. The expectation that Windows must be weaker because of timer
granularity was wrong for these observed short-trial results: at 1 ms Windows
v2 p50/p99/p99.9 are 0.5/1.2/4.8 µs versus M4 at 0.5 ms 2.000/16.166/19.291 µs.
Windows wins all three there, with **13.5× lower p99**, roughly an order of
magnitude. However, the ten-minute Windows p99.9 **25.5 µs is higher than M4's
historical 18.25 µs** (M4 at 1.5 ms). Windows ten-minute p50/p99 0.2/0.9 µs
are better than M4 1.5/16.625 µs (18.5× at p99), but it is false to claim a
Windows advantage across all three ten-minute percentiles. Preserve the
run-length/slack/idleness differences instead of generalizing the short sweep.

#### Tail structure

Exactly **30 samples strictly exceed the 25.5 µs p99.9**, with 31 at or above
it. They occupy these programme-time regions:

- 11 in 131.28–136.68 s, including a burst around 135–137 s.
- 13 in 507.74–516.34 s, including a burst around 511 s.
- Six others at 53.10, 115.72, 364.76, 376.20, 427.66 and 468.16 s.

Per-minute counts are **1, 1, 11, 0, 0, 0, 2, 2, 13, 0**. Maximum lateness
128.5 µs occurs at index 21,383 / 427.66 s. Tail-event gaps span 0.1–228.08 s.
Within the bursts, several gaps are 0.1 s or its multiples, but there is no
single stable run-wide period. Two bursts cannot identify a periodic service;
this is clustered behavior, not evidence for purely independent random noise.
Five-second CPU sampling and the absence of an exact recorded tick-origin UTC
prevent confident event-to-process attribution. No ETW/scheduler trace exists.

Relative to the v2 1 ms short trial, p99.9 rises **4.8→25.5 µs** and max
**6.0→128.5 µs**; the tail appears in the longer observation while p99 improves
1.2→0.9 µs. It does not monotonically thicken through the run: minutes 3 and 9
have elevated tails, with quiet intervals between and afterward. The older
1 ms trial's 9.4/30.7 µs tail is also lower than the ten-minute tail. All are
comfortably within the numeric bounds, but cannot promise Phase 1 headroom
once decode/composite work and GPU contention exist. The tail event indices
and raw values are retained in [the audit data](../benchmarks/phase-0-windows-v2-tail-audit.json).

#### CPU accounting conclusion

Across both Windows sweeps, all nonzero spin/whole-thread CPU totals are
multiples of 15.625 ms. The v2 500 µs trial reports zero spin CPU despite 2,789
spin entries and 684.7486 ms spin wall time. GetThreadTimes units are 100 ns;
this host's accounting resolution is much coarser. There is no honest numeric
conversion that recovers the missing per-segment attribution. Keep those raw
columns, label them quantized and not accuracy-equivalent to M4, and do not
replace them with elapsed wall time. ADR 0027 documents the API limit and why
cycle counters cannot be substituted as nanoseconds.

References: [Microsoft counter semantics](https://learn.microsoft.com/en-us/windows/win32/perfctrs/collecting-performance-data)
and [raw WMI sampling](https://learn.microsoft.com/en-us/windows/win32/wmisdk/wmi-tasks--performance-monitoring).

## Revisit when

Production hardware/OS changes, a second production target is proposed, or
hardware-backed secondary correctness workers cannot be supplied.
