# Phase 0 clock and dispatch baseline

`cargo xtask bench` measures a real ten-minute run, at 50 fps, of the headless
engine's payload-free tick dispatch. It writes
`phase-0-<os>-<architecture>.json`. This is the Phase 0 CPU clock/dispatch
baseline; compositor frame times, encoder throughput and VRAM measurements
are not applicable until those systems exist. There is no previous-phase
frame-time baseline to compare against.

The local macOS run uses Apple M4 / Mac16,12, 10 CPU cores, 24 GiB RAM, macOS
27.0 build 26A5416b, Rust 1.88.0, Cargo's development profile. It is explicitly
not the AMD RX 6800 XT reference machine. Builds and checks ran concurrently
with part of the measurement; timing excursions are retained in the report.

Expected ticks are indices 0 through 30,000 inclusive (30,001 ticks over
exactly 600 seconds of programme time). The report records observed clock
lateness against monotonic deadlines, contiguous consumer delivery, and the
independent stalled-sink eviction count. Final drift must be strictly below
20,000,000 ns; diagnostic average fps does not override that bound. Maximum
lateness and missed deadlines are also reported rather than hidden.

The draining sink must receive all ticks without loss. The deliberately
stalled sink has capacity two and must report exactly 29,999 evictions. Those
intentional evictions are not failures of the draining sink or programme clock.

Commit the report with its phase evidence. Hosted CI and reference-machine
reports remain pending until the owner provides the remote and runner.

The 2026-09-05 local ten-minute run passed final drift at **1.151 ms** and
received all **30,001** ticks. The active sink dropped zero ticks; the stalled
sink evicted **29,999** as expected. Maximum lateness was **139.018 ms** with
**22** deadlines at least one frame late. Thus the final accumulated-drift
criterion passed, but this result does not demonstrate jitter-free scheduling
under concurrent developer-machine load. Reference-machine testing remains
required. The raw report preserves all those counters.
