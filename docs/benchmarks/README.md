# Phase 0 clock measurements

Current acceptance is defined by ADRs 0017 and 0021: on the otherwise idle
Windows 11 / RX 6800 XT production reference machine, ten minutes at 50 fps, no skipped indices, final drift and
maximum lateness below 20 ms, and p99.9 lateness below 5 ms. `cargo xtask bench`
finishes its release build before a 15-second settling period and measurement.
The measured executable uses Rust 1.98.1 and the shared rezie-rt native scheduler.

Reports named `phase-0-idle-<os>-<architecture>.json` contain every per-tick
lateness sample, in index order, plus nearest-rank p50/p99/p99.9/max, effective
scheduling policy, privileges/fallback errors, and separate correctness and
latency outcomes. Samples are preallocated atomic slots written by the clock;
percentile calculation and file I/O happen after it stops. No samples are
silently lost in an observer queue.

The macOS developer machine is Apple M4 / Mac16,12, 10 cores, 24 GiB RAM,
macOS 27.0 build 26A5416b. It is not the AMD RX 6800 XT reference machine.
A local successful idle run is local evidence, not Windows/Linux latency evidence.

## Idle result — 2026-09-05

After the owner paused active applications, the 60-second pilot passed with
confirmed Mach time-constraint scheduling and a 19 microsecond maximum.
The 1.5 ms finishing slack was retained (ADR 0018).

The ten-minute `cargo xtask bench` run followed its completed release build
and a 15-second settling delay, approximately 17:58:17–18:08:17 UTC. No builds
or tests ran concurrently. `phase-0-idle-macos-aarch64.json` records all 30,001
samples. It passed the then-current local bounds; ADR 0021 now classifies
this report as historical development evidence, not production acceptance:

| Measure | Result |
| --- | ---: |
| p50 lateness | 0.001500 ms |
| p99 lateness | 0.016625 ms |
| p99.9 lateness | 0.018250 ms |
| Maximum lateness | 0.036292 ms |
| Final drift | 0.001167 ms |
| Skipped indices / PTS errors | 0 / 0 |
| Draining sink drops / stalled sink drops | 0 / 29,999 |

The OS reported Mach time-constraint scheduling with a 20 ms period, 2 ms
computation budget and 3 ms constraint. This is one successful local run,
not a guarantee of future latency or other platforms' performance.

## Hosted correctness checks

Hosted CI uses `cargo xtask clock-check`, which checks counts, ordering, exact
PTS, and queue isolation, without any latency threshold. It records
`latency_passed: null`. Normative latency acceptance runs manually or through the reference workflow
on the designated Windows machine, idle with MMCSS and timer resolution
confirmed. Runner automation is not a Phase 0 closure condition.

## Superseded measurement

`phase-0-macos-aarch64.json` is preserved historical evidence from Rust 1.88.0,
the unprioritized 0.2 ms finishing-spin implementation, in a development build
while other compilation/checks overlapped. Its `passed: true` reflected the
old final-drift-only rule and is **not a pass under current acceptance**.
Final drift was 1.151 ms, but maximum lateness was 139.018 ms with 22 missed
deadlines. Concurrent compilation invalidated its measurement conditions;
it is not treated as the cause of the scheduling defect. The old report has
no full distribution and cannot supply missing percentile evidence.

These are clock/dispatch measurements; no compositor, encoder or GPU baseline
is implied. No previous-phase frame-time baseline exists.

## Manual calibration and production evidence

`cargo xtask clock-sweep` records a separate raw report per slack, metadata,
CSV summary and SVG curve. The default six 60-second trials cover 0, 0.5, 1,
1.5, 3 and 5 ms in a fixed non-monotonic order. CPU cost is measured with the
thread CPU clock (Unix) or GetThreadTimes (Windows), not estimated from elapsed
spin time. Queries add overhead; calibration is not acceptance. Preserve all
trials and any failed acceptance report. The M4 sweep is developmental;
Windows reference acceptance is the sole normative Phase 0 timing result.
The owner supplied the M4 sweep from clean revision `6218679` on 2026-09-05:
[raw reports and metadata](phase-0-slack-sweep-macos-aarch64/metadata.json),
[full summary](phase-0-slack-sweep-macos-aarch64/summary.csv), and
[lateness/CPU curve](phase-0-slack-sweep-macos-aarch64/curve.svg).
All six trials report 3,001 ticks without index/PTS errors and confirmed Mach
priority. All 18,006 samples were checked against the reported quantiles.
The 0.5 ms candidate reached a 20.666 µs maximum with 2.338% of one core spent
spinning; 1.5 ms used 7.330% without a clear latency improvement in this sweep.
The zero-slack maximum was 4.549125 ms. See ADR 0021 for all four percentiles
at every value, CPU costs and the interpretation limits. A finer manual M4
sweep below 0.5 ms is proposed before selecting its final constant.
The owner deferred Windows testing until later. Its sweep and final ten-minute
result remain pending; no reference acceptance or phase closure is claimed.
Instructions: [clock calibration](../user/clock-calibration.md).

From Phase 1, also record the M4 two-input, one-output 1080p50 diagnostic at
every phase gate with no performance threshold. It is explicitly separate
from production benchmarks, golden references and soak evidence.

## Conditional Phase 0 closure

ADRs 0022–0023 supersede the earlier provisional-Mac/no-closure statements:
macOS now uses the owner-approved 500 µs value. Finer M4 sampling is optional.
Phase 0 is conditionally closed, not fully verified. The Windows reference
sweep and ten-minute benchmark are its sole outstanding item, due at the
Phase 1 gate. Failure reopens Phase 0 and stops Phase 1 until rezie-rt is fixed.
See `docs/phases/OUTSTANDING.md`; no Windows value has been inferred from M4.
