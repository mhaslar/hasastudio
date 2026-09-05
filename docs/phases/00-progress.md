# Phase 0 — implementation and verification progress

**Status: active; the phase gate has not passed.** The owner explicitly deferred
adding a GitHub repository. Windows/Linux execution, three-platform hosted CI
and reference-machine measurements remain outstanding. No Phase 1 work is
enabled. `00-summary.md` is reserved for the actual phase completion because
AGENTS.md treats that filename's existence as evidence of completion.

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
  ruling ADRs before any implementation code. Remote setup remains deferred.

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

The owner will add GitHub later. Until then, remote push and passing hosted CI
on Windows/macOS/Linux cannot be evidenced. The reference-machine nightly
runner also needs provisioning. These are remaining acceptance gates, not
successful checks or permission to start Phase 1.

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

Attach the owner-supplied remote, execute the prepared three-platform CI,
verify runnable bundles on Windows/Linux and run the reference-machine
measurements. Review all resulting evidence. Only after the gates pass, write
`docs/phases/00-summary.md` with final benchmarks and update the phase marker.
Do not implement Phase 1 in that completion change.
