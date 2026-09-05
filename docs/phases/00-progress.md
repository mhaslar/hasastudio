# Phase 0 — implementation and verification progress

**Status: active; the phase gate has not passed.** The GitHub remote is now
attached at `mhaslar/hasastudio`. The owner replaced final-drift-only acceptance
with idle-machine maximum and percentile lateness bounds. ADRs 0017–0019
record the criterion, shared realtime boundary, and current stable toolchain.
No Phase 1 work or `00-summary.md` is introduced.

## Scheduling correction and idle measurement

- Added `rezie-rt`, with no domain types, for Mach time constraints, Windows
  MMCSS Pro Audio plus 1 ms timer resolution, Linux FIFO/timerfd/nice fallback,
  and a generic sleep-then-spin deadline waiter. Thread-affine guards restore
  prior state on drop/unwind. Phase 6 audio will reuse this crate.
- Kept crate-level `forbid(unsafe_code)` on engine/core/API/app. The future
  audio/rundown crates must also forbid unsafe. Only the foreign-interface
  crate list was extended, by explicit human review.
- Replaced the unsupported Rust 1.88.0 choice with official current stable
  **1.98.1**, verified against the 2026-09-03 stable manifest. Edition stays 2021.
- Full per-tick lateness samples are preallocated before startup, retained in
  index order and summarized after shutdown. Reports include nearest-rank
  p50/p99/p99.9/max and actual scheduling status/error codes.
- Hosted CI checks correctness only. Idle latency checks are explicit and
  require native priority success (or the permitted elevated Linux fallback).
- Local formatting, strict clippy, workspace tests, golden inventory and
  updated packaged macOS launch passed: 17 short tests and one compile-fail
  doctest, with the ten-minute test reserved for idle measurement. Windows
  and Linux realtime modules also type-check and pass target-specific clippy.
- Initial idle preflight found no compiler/linker processes but substantial
  unrelated CPU work. No calibration/acceptance run was taken under that
  load. After the owner paused active work, the one-minute pilot passed.
- `cargo xtask bench` then completed its release build and 15-second settling
  delay before the ten-minute idle run (2026-09-05, approximately
  17:58:17–18:08:17 UTC). No builds/tests ran alongside either measurement.
  All 30,001 ticks arrived in order with exact PTS; no indices were skipped.
  Mach time-constraint policy was confirmed. Lateness p50/p99/p99.9/max was
  **0.001500 / 0.016625 / 0.018250 / 0.036292 ms**; final drift was
  **0.001167 ms**. The draining sink dropped zero ticks; the stalled sink
  dropped exactly 29,999. All current local timing bounds passed.
  Full raw samples are in `docs/benchmarks/phase-0-idle-macos-aarch64.json`.
- After measurement, pushed implementation and raw evidence as `d1aac59` to
  `Github-HasaStudio/main`. GitHub started the
  [three-platform acceptance run](https://github.com/mhaslar/hasastudio/actions/runs/33983189864).
  That run records each platform's formatting, strict clippy, workspace tests
  (including native unwind cleanup), golden inventory, tick correctness,
  packaged GUI launch and artifact result. Hosted reports explicitly leave
  latency unevaluated; they do not replace the idle report above.
- That run exposed a Linux launch failure after a successful release build:
  the runner lacked `libxkbcommon-x11.so.0`. ADR 0020 records provisioning
  the already-selected X11 backend's runtime package, `libxkbcommon-x11-0`.
  No smoke condition or clock code changed. Follow-up run results are listed
  in the [acceptance workflow history](https://github.com/mhaslar/hasastudio/actions/workflows/ci.yml).
- GitHub's repository-runner query returned zero registered self-hosted
  runners. Nightly measurements need a trusted reference machine with both
  `self-hosted` and `rezie-reference` labels and appropriate RT permissions.
  No extra secrets or branch-protection changes were needed for this push.

The implementation and measurements below describe the earlier baseline and
are retained as history. The final-drift-only result is superseded, not current
latency acceptance evidence.

## Built

- Rust 1.88.0 / edition 2021 workspace with five working packages:
  rezie-core, rezie-api, rezie-engine, rezie-app and xtask. Cargo.lock is tracked.
- §6 domain data, transparent u32 IDs, programme settings and rational clock
  arithmetic. Runtime input residency is omitted from serialization. Data
  definitions do not open inputs, execute transitions or persist project files.
- An engine-owned project and snapshots; bounded command/reply channels;
  GetState, SetProjectName and Shutdown commands with correlated events.
  Invalid mutations leave state unchanged. Transport errors distinguish an
  uncertain outcome from an engine rejection.
- In-process and loopback WebSocket transports. The latter bounds messages,
  connections and request waits, supports port-zero harness discovery, and
  flushes in-flight replies during normal shutdown.
- Dedicated control and clock threads. The clock derives each timestamp from
  one monotonic origin and an absolute index. Its steady-state path contains
  no pixel payload, allocation, locks, I/O, logging or GPU resource creation.
  Per-sink bounded crossbeam queues overwrite their own oldest tick and count
  evictions. A stalled sink does not block any other sink.
- An empty eframe/wgpu application and headless executable. No GUI code or
  GPU package appears in the engine's normal dependency graph.
- Phase-gated HTTPS dependency fetching, streaming SHA-256 verification,
  atomic cache publication and cache revalidation. The real crossbeam-channel
  0.5.15 archive was downloaded and verified. The specific Windows FFmpeg
  7.1.1 LGPL shared bundle/hash is recorded but gated to Phase 1. No FFmpeg,
  SRT, CEF or NDI SDK was fetched or linked.
- Development bundle generation for macOS, Windows and Linux, plus an actual
  packaged GUI smoke test. macOS was built and launched locally.
- Three-platform GitHub acceptance workflow and a separate nightly workflow
  for a provisioned reference machine. These are prepared, not executed.
- Git initialized on `main`; first commit `884d92c` contains the spec and
  ruling ADRs before any implementation code. The remote was attached after that initial baseline.

## Local verification

On 2026-09-05, Apple M4 (Mac16,12), 10 CPU cores, 24 GiB RAM, macOS 27.0
(26A5416b), using the pinned Rust 1.88.0 toolchain:

- `cargo build --workspace`: passed.
- `cargo xtask ci`: passed, including dependency cache revalidation,
  `gen-assets`, `fmt --all -- --check`, strict clippy, workspace tests and golden
  scope verification. Thirteen short tests passed; one ten-minute integration
  test is explicitly ignored in the short suite and covered by `xtask bench`.
- Integration includes an actual headless subprocess: connect over WebSocket,
  read state, request shutdown, receive its reply and verify clean process exit.
- Golden inventory: zero compositor paths and zero pixel comparisons, explicitly
  reported. No image references were created or updated. Payload-free ticks
  are verified through the engine tests and benchmark instead.
- `cargo xtask dist --smoke`: passed. The macOS application initialized its
  native window/Metal backend, received an engine tick during a GUI update,
  exited cleanly and wrote the expected smoke marker.
- `cargo xtask bench`: passed the normative ten-minute final drift check.
  All 30,001 ticks arrived in order with exact PTS. Final drift was 1,150,958 ns
  (1.151 ms), below the strict 20 ms limit. The draining sink dropped zero
  ticks; the deliberately stalled sink evicted exactly 29,999 ticks.
  The run recorded 22 deadline misses and maximum lateness 139,018,291 ns
  (139.018 ms). Builds/checks overlapped part of the run; these observations
  are retained, not attributed conclusively to one cause or concealed by
  the passing final drift result. This is not a jitter-free run. See
  `docs/benchmarks/phase-0-macos-aarch64.json`. A developer-machine result is
  not a reference-machine result.

## Deferred and why

GPU FramePool, pixel payloads, compositing, media ingestion, media outputs,
DSP, operational resource promotion, project/rundown file persistence, and
production GUI controls belong to later phases. Shared-device rendering begins
in Phase 1 under the explicit human-approved shell exception. No placeholder
later-phase crates or unimplemented commands are introduced.

The local idle scheduling gate passed and the hosted workflow was triggered.
The reference-machine nightly runner still needs provisioning; local timing
evidence does not claim performance on that hardware or on Windows/Linux.

## Surprises and corrections

Disabling eframe defaults while selecting its wgpu feature compiled but did
not enable a native GPU backend. The first real packaged launch failed before
its first update. ADR 0016 records enabling Metal/D3D12/Vulkan explicitly; the
corrected packaged launch passed. This is why compile success was not treated
as application acceptance.

The environment initially had no Rust toolchain or Git repository. Rust was
installed into temporary directories without altering the user's shell setup.
The sandbox denied loopback sockets on the first test attempt; rerunning the
same tests with approved local socket access passed. Neither restriction was
hidden by skipping the WebSocket tests.

The spec's black-frame, codec fallback, resource-state, naming and clock-drift
ambiguities are recorded in ADRs 0001–0010 using the owner's rulings. The
original spec contained HasaStudio at lines 1, 21, 23, 27, 485, 618 and 770.
After amendments the occurrences are at lines 1, 21, 23, 27, 497, 630 and 782.

## Remaining phase gate

Hosted acceptance results are in the linked workflow history above. SPEC §14's nightly
reference benchmark/soak workflow cannot execute until its runner is
provisioned. The owner's amendment permits the idle local machine to satisfy
the Phase 0 clock criterion, but does not supply that nightly infrastructure.
Keep the phase active and record the missing runner explicitly. Only after
the remaining gates pass, write
`docs/phases/00-summary.md` with final benchmarks and update the phase marker.
Do not implement Phase 1 in that completion change.
