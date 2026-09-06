# Phase 0 — Foundation (conditionally closed)

**Conditionally closed by explicit owner ruling (ADR 0023), not fully verified.**
The next phase is Phase 1. Exactly one obligation remains in
[OUTSTANDING.md](OUTSTANDING.md): the Windows 11 / RX 6800 XT clock benchmark,
including that platform's slack sweep, blocked on adequate idle evidence and
**due at the Phase 1 gate**. M4 measurements do not satisfy that obligation.
If the Windows measurement fails, Phase 0 reopens and Phase 1 work stops until
rezie-rt is fixed. No further conditional closure is permitted while it is open.

## Windows evidence review — 2026-09-06

The committed Windows sweep is internally consistent, but no ten-minute
Windows report is committed. Power plan and idle evidence are missing. This
is incomplete evidence, so conditional closure and the single OPEN obligation
remain unchanged; no Windows default is pinned. At candidate 1 ms, the
60-second Windows p50/p99/p99.9/max are 0.600/1.300/9.400/30.700 µs, final
0.400 µs, recorded spin CPU 0.833% and whole-thread CPU 2.500%. All six trials
confirm successful MMCSS Pro Audio and timeBeginPeriod(1). See the full curve,
CPU-accounting limits and M4 comparison in [ADR 0021](../decisions/0021-assign-platform-evidence.md#windows-evidence-audit--2026-09-06).
A successful short calibration trial is not ten-minute acceptance.

## Windows v2 review — still not fully verified

The ten-minute v2 report meets all numeric bounds: p50/p99/p99.9/max
0.200/0.900/25.500/128.500 µs, final drift 0.300 µs. Scheduling and High
Performance are confirmed. Idle evidence remains INADEQUATE: roughly 11%
reported total CPU is not reconciled with the partially readable per-process
counters. Thirty samples exceed p99.9, mostly in two clusters. ADR 0021 records
the full audit. No obligation is paid and no timing bound is relaxed.

## Built

- Rust 1.98.1 / edition 2021 workspace: rezie-core, rezie-api, rezie-engine,
  rezie-app, rezie-rt and xtask. Engine state is authoritative; the empty GUI
  sends commands/reads events, and the same engine runs without GUI dependencies.
- Domain types, exact rational programme PTS, in-process and loopback WebSocket
  commands/events, snapshots, bounded command queues, orderly shutdown and a
  dedicated clock dispatching payload-free FrameTime ticks. Each sink has its
  own bounded queue, oldest-tick eviction and drop accounting.
- Safe thread-affine realtime configuration with OS-specific priority and
  sleep/spin waiting. RAII restores scheduling/timer state, including unwind.
  rezie-rt is the only new unsafe-permitted crate; engine/core/API/app forbid
  unsafe. Phase 6 audio will reuse this generic boundary.
- macOS's default finishing slack is **500 µs**, selected by the owner from
  the measured curve. Windows/Linux have no calibrated default and return an
  explicit startup error. Calibration and correctness harnesses pass explicit
  diagnostic values; no platform silently inherits Mac tuning.
- Phase-gated native-dependency manifest and real HTTPS fetch/hash verification,
  exercised against crossbeam-channel 0.5.15. FFmpeg 7.1.1 LGPL is pinned for
  its consuming phase; CEF is never fetched before Phase 10 and NDI SDK is never
  fetched. The developer bundle is built and actually launched on all platforms.
- Public GitHub repository with ci-fast, ci-full and reference workflows,
  nextest plus compile-fail doctests, Rust caches, documentation path filters,
  ref-keyed cancellation and a fast prerequisite before the platform matrix.
  External contributors all require Actions approval. Reference has no PR
  trigger, runs trusted main only, and requires repository-scoped registration.
  The currently absent reference runner is not claimed to be provisioned.

## Verification

Local formatting, strict Clippy, nextest (19 executable tests), the thread-affinity
compile-fail doctest and the Phase 0 golden inventory passed. The idle ten-minute
test is deliberately excluded from hosted nextest. Hosted correctness asserts
counts/order/exact PTS and sink isolation, never latency. Windows/Linux native
scheduling modules also passed cross-target Clippy.

The restructured hosted matrix passed on Windows, macOS and Linux at `a0e44b4`
([run 33990508830](https://github.com/mhaslar/hasastudio/actions/runs/33990508830)),
including native cleanup/correctness tests and actual packaged GUI launches.
The cold critical path was 12m41s including the 2m41s fast prerequisite;
the prior uncached critical path was 17m21s (17m20s slowest job). The same-revision warm-cache run 33991160147 passed in 5m09s,
including a 58s fast prerequisite, and is recorded in the CI timing evidence. Those timings exclude external
approval and queue delays and do not establish reference-machine performance.

The Phase 0 golden command verifies zero compositor paths and no pixel
references. No GPU/compositor/codec result is implied by that inventory.

## What the benchmarks said

The initial unprioritized run retained 22 deadline misses and 139.018 ms maximum
lateness despite only 1.151 ms final drift. It overlapped compilation and is
not valid idle acceptance. Absolute-origin timestamps make final drift alone
an inadequate jitter test; the owner's corrected bound now includes maximum
and p99.9 lateness, with all raw samples preserved.

After native priority and waiting fixes, the historical M4 ten-minute run at
1.5 ms slack recorded 30,001 contiguous ticks, exact PTS, zero draining-sink
drops and 29,999 stalled-sink drops. Its p50/p99/p99.9/max lateness was
**1.500 / 16.625 / 18.250 / 36.292 µs**; final drift was **1.167 µs**.
This is historical development evidence under ADR 0021, not Windows acceptance.

The owner's six 60-second M4 trials produced 18,006 checked raw samples with
Mach priority confirmed at every value. Lateness below is in microseconds;
CPU is actual finishing-spin time expressed as a percentage of one core.

| Slack (ms) | p50 | p99 | p99.9 | Max | Spin CPU |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 31.875 | 57.291 | 103.583 | 4549.125 | 0.000% |
| 0.5 | 2.000 | 16.166 | 19.291 | 20.666 | 2.338% |
| 1 | 2.125 | 16.917 | 18.833 | 20.125 | 4.831% |
| 1.5 | 2.416 | 16.625 | 18.708 | 23.000 | 7.330% |
| 3 | 2.375 | 15.333 | 17.375 | 20.000 | 14.833% |
| 5 | 2.458 | 16.291 | 18.208 | 19.125 | 24.831% |

Higher slack provided no clear latency benefit in this sweep. The owner chose
0.5 ms now; finer sampling below it is optional, not a second outstanding item.
Profiling adds CPU-clock queries; the eventual reference acceptance run disables
that instrumentation. Raw reports, complete CPU accounting and the curve are
in [the M4 measurement directory](../benchmarks/phase-0-slack-sweep-macos-aarch64/metadata.json)
and [ADR 0021](../decisions/0021-assign-platform-evidence.md).

CI wall times are separate engineering diagnostics, recorded in
[CI timing evidence](../ci/timing-evidence.json); they are never media-performance
benchmarks. No production GPU, encoder or five-minute allocation result exists.

## Deferred and why

Pixels, FramePool, shared GUI/compositor device, file/image/colour sources and
NDI output begin in Phase 1. Mixes, overlays, DSP, recording, rundown execution,
HTML and full production controls remain in their specified later phases.
No CPU frame type, pixel placeholder or later-phase command implementation was
introduced in Foundation. The reference clock obligation is the sole conditional
phase debt; absent runner automation is separate infrastructure, not a second item.

## Surprises and corrections

An empty eframe application compiled without selecting native GPU backends but
failed at runtime. Enabling the intended Metal/D3D12/Vulkan backends fixed it.
Linux CI later exposed a missing libxkbcommon-x11 runtime; installing that
existing backend dependency fixed the actual launch. The sandbox's loopback
socket denial was reported and tests ran with approved socket access, not skipped.

The initial Rust 1.88 pin was an environment accident and was replaced by the
verified 1.98.1 release with an ADR. Its literal compiler/manifest provenance is
recorded in ADR 0019; the owner confirmed the point release and retained the pin.

The public CI restructure initially hit the repository's local-only action
policy, scheduling zero jobs. A four-commit action allowlist fixed that without
blanket third-party allowances. The next fast run found a missing fetch-deps
prerequisite and prevented the entire matrix from starting. The fetch was
restored; no assertion was removed and trusted-main caches now survive failures.
