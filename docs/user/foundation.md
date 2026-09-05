# Foundation application

Phase 0 opens an empty HasaStudio window and starts a headless engine in the
same process. Closing the window joins the engine threads. There are no media
controls yet. Every engine operation is available through the shared API.

The programme clock dispatches timestamps, not pixels. The configured domain
types describe later production features but do not activate them.

## Build and launch

Install Rust through rustup. `rust-toolchain.toml` selects stable 1.88.0 with
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

`cargo xtask bench` runs for ten minutes and writes a machine-labelled JSON
report under `docs/benchmarks/`. It checks all 30,001 ticks from index 0 at
zero seconds through index 30,000 at 600 seconds. PTS must match the absolute
50 fps timeline. The draining sink must receive every tick in order without
drops; a deliberately stalled two-entry sink must evict exactly 29,999 old
ticks. Final observed clock drift must be **strictly less than 20 ms**. The
report also exposes maximum lateness and deadline misses; average fps is
diagnostic and cannot override a failing drift result. This is a clock and
dispatch baseline, not a compositor/GPU benchmark.

`cargo xtask soak --minutes 30` exercises the same actual clock and dispatch
for a longer duration and writes `target/soak.json`. The ten-minute integration
test is explicitly ignored by the short test suite; CI runs `bench` as a
separate mandatory step. Neither a deterministic arithmetic test nor a short
run substitutes for the ten-minute gate.

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

The local repository is initialized and the first commit records human
rulings before implementation. The owner has deferred adding a GitHub remote.
Workflows are prepared for Windows, macOS and Linux, including ten-minute
clock measurements and actual packaged GUI launches. They are not evidence
of passing CI until executed. The nightly reference-machine workflow needs
a provisioned `self-hosted, rezie-reference` runner. Phase 0 remains active
until those gates are evidenced; Phase 1 must not begin meanwhile.
