# Foundation application

Phase 0 opens an empty HasaStudio window and starts a headless engine in the
same process. Closing the window joins the engine threads. There are no media
controls yet. Every engine operation is available through the shared API.

The programme clock dispatches timestamps, not pixels. The configured domain
types describe later production features but do not activate them.

## Build and launch

Install Rust through rustup. `rust-toolchain.toml` selects stable 1.98.1 with
rustfmt and clippy. A platform C linker/SDK is needed by eframe: Xcode command
line tools on macOS, MSVC build tools on Windows, and a C toolchain plus
X11/Wayland development packages on Linux. HTTPS dependency fetching uses
`curl` (`curl.exe` on Windows).

Run `cargo xtask fetch-deps` first, then `cargo xtask gen-assets` and
`cargo build --workspace --locked`. Launch with `cargo run -p rezie-app`.

`cargo xtask dist` creates a release bundle in `dist/<os>-<architecture>/`:
a macOS `.app`, Windows portable executable directory, or Linux portable
executable directory with desktop entry. These are development bundles, not
signed Phase 11 installers. Linux requires a desktop session and graphics
driver. `cargo xtask dist --smoke` launches the actual packaged application,
waits for a GUI update with a live engine tick, then closes it and writes
`target/dist-smoke.txt`. Failure to open a window is a failing smoke test.

Phase 0 uses eframe's wgpu device. Sharing the compositor's device begins in
Phase 1. FFmpeg, NDI, SRT and CEF are not needed or linked by Foundation.

## Headless harness

This interface is for testing, not a GUI feature:

```sh
cargo run -p rezie-engine --bin rezie-headless -- --ws 127.0.0.1:9800
```

The WebSocket accepts JSON text requests on loopback only, with up to 16
connections and 64 KiB per message. Example:

```json
{"id":42,"command":{"type":"SetProjectName","name":"Evening programme"}}
```

An `Applied` event returns ID 42 and the resulting authoritative `state`.
Names must be nonempty, no more than 128 UTF-8 bytes, and contain no control
characters. Invalid names receive `Rejected` and leave the revision and
project name unchanged. `GetState` returns a `State` event; `Shutdown` returns
an `Applied` event and stops the engine. Malformed JSON receives `Rejected`
with a null ID. A response timeout is an indeterminate outcome: query state
before retrying. `TransportError` reports transport failures and includes
`may_have_applied`; it does not falsely claim the engine rejected the command.
The in-process client uses the same commands and handler. Harnesses can bind
port zero and pass `--ready-file <path>` to discover the bound address without
a port-selection race.

The GUI consumes read-only engine snapshots. It does not optimistically
mutate project data. Snapshot counters may lag by one control publication;
they are refreshed about every 10 ms. Queue depth/counters are approximate
while the clock is active and exact once it stops.

## Verification and measurements

`cargo xtask ci` runs dependency verification, asset generation, formatting,
clippy, workspace tests and the golden inventory. There are zero compositor
paths and zero image comparisons in Phase 0; `golden` explicitly verifies
that scope. It does not claim to test GPU pixels. The real tick tests are in
`tests/integration/foundation.rs`.

`cargo xtask clock-check` runs correctness only: contiguous indices, exact PTS,
complete delivery and independent sink drop accounting. Hosted CI uses this
mode and never asserts latency on shared runners. Its report explicitly has
`latency_passed: null`.

`cargo xtask bench` builds the release headless executable first, waits 15
seconds for builds to settle, then measures ten minutes on an otherwise idle
machine. Keep other applications/workloads idle for this measurement. It
writes `docs/benchmarks/phase-0-idle-<os>-<architecture>.json`, including all
30,001 lateness samples in tick order and nearest-rank p50/p99/p99.9/max.
Acceptance requires no skipped indices, final drift and maximum lateness
strictly below one frame interval (20 ms at 50 fps), and p99.9 below 5 ms.
The report records the achieved native scheduling policy and error codes;
it cannot pass latency with a denied priority request disguised as success.
A short pilot does not substitute for the ten-minute acceptance run.

`rezie-rt` is the reusable realtime boundary. macOS uses Mach time-constraint
scheduling; Windows uses MMCSS Pro Audio and a matched 1 ms timer-resolution
request; Linux requests SCHED_FIFO priority 10, falling back to monotonic
timerfd waits and attempted nice -10. If Linux denies RT or nice elevation,
the startup log and benchmark identify the exact error. Configure appropriate
CAP_SYS_NICE / RLIMIT_RTPRIO / RLIMIT_NICE permissions for a timing reference
runner; the program does not silently grant itself privileges. Guards restore
prior thread state on drop, including unwind. Audio will reuse this in Phase 6.

`cargo xtask soak --minutes 30` checks long-running clock/dispatch correctness
and writes `target/soak.json`. Latency acceptance is a separate idle benchmark.
The ignored ten-minute integration test is for idle local/reference use only.
No hosted test asserts a maximum or percentile lateness bound.

Logs use tracing with a nonblocking rolling file writer. The headless binary
writes `.logs/`; the GUI writes `rezie-logs` under the OS temporary directory
so packaged launch does not require a writable installation directory. There
is no logging on the clock's steady-state path.

## Dependency policy

`xtask/dependencies.json` pins versions, SHA-256 values and consuming phases.
Downloads go to ignored `.deps/`; no dependency source is committed. Existing
cache files are rehashed. A mismatch fails without silently trusting or
replacing the damaged file; remove that specific cache file and rerun the
fetch command after investigating it. Downloads use a temporary file and are
renamed only after verification. There is no command-line phase override.

The FFmpeg Windows 7.1.1 LGPL shared archive is pinned for Phase 1 but is not
downloaded now. CEF cannot be fetched before Phase 10. The NDI SDK is never
fetched and requires user installation/licence acceptance in its consuming
phase; absence must leave all non-NDI features functional.

## Git and CI

The repository is initialized on `main`; the attached remote is
`Github-HasaStudio` at `https://github.com/mhaslar/hasastudio.git`. The prepared
workflow checks Windows/macOS/Linux correctness and launches each packaged
GUI. Timing runs are isolated to a local idle machine or a provisioned
`self-hosted, rezie-reference` runner. No repository secrets are required for
hosted correctness. Phase 0 remains active until all required evidence is in.
