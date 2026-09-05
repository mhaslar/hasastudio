# Manual Phase 0 clock calibration

The owner runs both sweeps manually. No self-hosted runner is needed. The
Windows 11 / RX 6800 XT result is Phase 0's sole conditional-closure obligation,
due at the Phase 1 gate (OUTSTANDING.md); runner provisioning and nightly soak
automation are separate. The M4 sweep informs its
development configuration and never provides production acceptance.

## Prerequisites and checkout

Use the native OS, not WSL or a VM standing in for the reference machine.
Windows needs the RX 6800 XT/Adrenalin driver, Git, rustup and MSVC x64 build
tools with the Windows SDK. If those tools are missing, follow the official
[Rust installer](https://rust-lang.org/tools/install/) and
[MSVC setup instructions](https://rust-lang.github.io/rustup/installation/windows-msvc.html)
(the Desktop Development with C++ workload includes the required components).
Use the Visual Studio edition appropriate to your installation.
An x64 Developer PowerShell is a convenient Windows shell.

For a fresh checkout, on either platform:

```sh
git clone https://github.com/mhaslar/hasastudio.git
cd hasastudio
rustup toolchain install 1.98.1 --profile minimal --component clippy --component rustfmt
rustc -vV
cargo xtask fetch-deps
```

For the existing Mac checkout, use `/Users/matyashaslar/HasaStudio` and the
latest pushed Phase 0 commit. If using the agent's temporary toolchain instead
of your own rustup installation, run these in that terminal first:

```sh
export RUSTUP_HOME=/tmp/rezie-rustup
export CARGO_HOME=/tmp/rezie-cargo
export PATH="/tmp/rezie-cargo/bin:$PATH"
```

The literal pin is `1.98.1`, with rustfmt and clippy. It was reverified against
both `rustc -vV` and the official September 3 release manifest; see ADR 0019
for the complete output, commit hash and manifest hash. On Windows, verify
`host: x86_64-pc-windows-msvc`; on M4, `host: aarch64-apple-darwin`.

## Sweep, on each machine

Pause other work and let the machine stay idle. Run:

```sh
cargo xtask clock-sweep
```

The tool builds the release executable first, captures compiler/revision/host
metadata, then waits 15 seconds before sampling. It measures six 60-second
trials in this order: **1.5, 0, 5, 0.5, 3, 1 ms**, with a two-second pause
after each. Allow about **6½ minutes after the build**. Zero is the
priority-enabled sleep-only baseline. No builds overlap sampling.

Expected directories:

- M4: `docs/benchmarks/phase-0-slack-sweep-macos-aarch64/`
- Windows: `docs/benchmarks/phase-0-slack-sweep-windows-x86_64/`

Each contains `metadata.json`, six `slack-<microseconds>-us.json` reports,
`summary.json`, `summary.csv`, and `curve.svg`. Open the SVG to see lateness
and CPU cost against slack. Every raw report contains all **3,001 samples**,
p50/p99/p99.9/max, exact tick/PTS checks, requested and applied slack, native
policy/errors, and CPU measurements. `latency_passed` is null: a calibration
trial is not ten-minute acceptance, and there is no Mac performance threshold.

CPU columns distinguish actual finishing-spin CPU nanoseconds, whole
clock-thread CPU nanoseconds, and spin wall nanoseconds. Percentages use the
measured wall interval as the denominator and represent one CPU core. The
Unix implementation uses `CLOCK_THREAD_CPUTIME_ID`; Windows sums kernel and
user time from `GetThreadTimes` (100 ns units, subject to OS accounting
granularity). This is not process CPU time or an estimate from wall time.

Calibration adds two CPU-time queries around each finishing-spin segment.
Those queries have overhead; normal operation and ten-minute acceptance
disable them. At 50 Hz, the native computation budget is max(2 ms, slack + 0.5 ms) and
the constraint is another 1 ms; every trial records both. Assess the curve
with those instrumentation and budget differences visible.

Send or commit **both complete directories**, not just the graph or pass flag.
Do not commit this measurement evidence alongside unrelated changes:

```sh
git add docs/benchmarks/phase-0-slack-sweep-macos-aarch64
git commit -m "test(phase-0): record manual M4 slack sweep"
```

On Windows use `docs/benchmarks/phase-0-slack-sweep-windows-x86_64` and the
commit message `test(phase-0): record manual reference slack sweep`.
Share the commit/output for review before selecting the constant. The tool
never silently chooses a value. Existing sweep directories are protected
against overwriting; repeat with a fresh `--output` directory if needed.

If the curve needs finer sampling, for example:

```sh
cargo xtask clock-sweep --slacks-us 0,100,250,500,750,1000 --output docs/benchmarks/phase-0-slack-refinement
```

This example is a refinement command, not a recommendation to choose any
particular value. Keep all reports, including poor candidates. Runs shorter
than 60 seconds via `--seconds` are functional tooling checks only, not
calibration evidence.

## Windows ten-minute acceptance after sweep review

Pick the smallest slack comfortably above where lateness starts degrading,
considering the CPU-cost curve. Record the choice and both sweeps in ADR 0021
and pin the platform constants. After checking out that pin, run on the idle
Windows reference:

```powershell
cargo xtask bench
```

For an explicitly reviewed candidate before it is pinned, the equivalent
command is `cargo xtask bench --slack-us N`, where `N` is the reviewed integer
in microseconds; the report records the override. An override is not a licence
to skip pinning the final default or verifying that it matches the measurement.

Expected files:

- `docs/benchmarks/phase-0-idle-windows-x86_64.json`
- `docs/benchmarks/phase-0-idle-windows-x86_64.host.json`

The command verifies Windows 11/RX 6800 XT identity and records OS/GPU/driver
metadata. It builds before its 15-second settling delay. Allow ten minutes
after settling, with the machine otherwise idle. The report must contain
30,001 contiguous ticks with exact PTS, confirmed MMCSS Pro Audio and 1 ms
timer resolution, final drift and maximum lateness strictly below 20 ms,
and p99.9 strictly below 5 ms. All lateness samples are retained. CPU profiling
is disabled (`wait_profile: null`) for this acceptance run.

```powershell
git add docs/benchmarks/phase-0-idle-windows-x86_64.json docs/benchmarks/phase-0-idle-windows-x86_64.host.json
git commit -m "test(phase-0): record manual Windows reference clock acceptance"
```

Preserve a failed report before repeating anything. If correctly configured
MMCSS cannot meet the maximum-lateness bound, stop for an ADR and human review;
do not lower the standard or present a short/hosted/M4 run as a substitute.

## Approved operating values (ADR 0022)

macOS now uses the owner-approved 500 µs value from the recorded M4 sweep.
Finer sampling is optional. Windows and Linux have no calibrated default and
normal startup returns a missing-calibration error. Sweep candidates remain
explicit overrides, so an unset default does not prevent calibration.
Hosted correctness and GUI smoke use an explicit zero-slack diagnostic value;
that is never a performance result. The WebSocket harness can run explicitly
with `rezie-headless --ws 127.0.0.1:9800 --slack-us 0` for correctness.
If the deferred Windows benchmark fails, Phase 0 reopens and Phase 1 work stops
until rezie-rt is fixed; the latency bounds are unchanged.
