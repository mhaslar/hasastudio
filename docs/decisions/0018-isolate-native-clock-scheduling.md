# 0018 — Share native realtime scheduling in rezie-rt

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** human review
- **Affects:** AGENTS.md rule 7, SPEC §15, rezie-rt, safe crate boundaries

## Context

The user mandates thread_policy_set(THREAD_TIME_CONSTRAINT_POLICY) on macOS,
MMCSS Pro Audio plus timeBeginPeriod(1) on Windows, and SCHED_FIFO or a
timerfd/elevated-nice fallback on Linux. Rust calls to these OS APIs require
unsafe FFI. AGENTS.md rule 7 and SPEC §15 allow unsafe only in rezie-ndi,
rezie-media, rezie-capture and rezie-html. None owns programme clock scheduling.
Putting these calls there merely to avoid the rule would violate crate ownership.

## Options

### A — Permit unsafe only in rezie-engine::clock_platform

Create a private platform module exposing a safe thread-affine scheduler guard,
an absolute-deadline wait, and a plain scheduling report. All native handles
are owned by the creating thread, cannot be sent/shared, and are released on
that thread. Every unsafe block documents pointer/lifetime/ABI invariants.
Use libc and windows-sys bindings to OS-provided libraries only; no external
native archive or SDK is fetched. libc and windows-sys are MIT/Apache-2.0.

### B — Put clock FFI in an existing allowed media/capture crate

This creates the wrong ownership and introduces a later-phase crate for an
unrelated concern. Rejected as a proposed design.

### C — Create a separate rezie-clock-platform crate

This gives a separate unsafe boundary but adds a crate and still needs an
exception to the same repository rule. A single private module is sufficient
for the small engine-owned OS boundary.

## Decision

Human review chose a shared `rezie-rt` crate, a refinement of option C, and
rejected option A's engine-module exception. Its subject is realtime thread
configuration generally, not clock semantics. It contains priority elevation,
a sleep-then-spin deadline waiter, a safe thread-affine API, RAII restoration
including unwind, and reports what was actually achieved. It contains no
domain types or knowledge of frames or audio blocks; callers own timing.

Add rezie-rt to the unsafe-permitted foreign-interface crate list in AGENTS.md
and SPEC §15. Place `#![forbid(unsafe_code)]` on rezie-engine, rezie-core,
rezie-api and rezie-app now; require it on rezie-audio and rezie-rundown when
those later-phase crates are introduced. Do not create empty crates early.
No deny-plus-local-allow precedent is permitted in those safe crates.

Use the existing libc 0.2 binding family on macOS/Linux and OS-defined
Avrt/Winmm bindings on Windows. The two-field Mach timebase declaration is
kept in rezie-rt because libc deprecates its duplicate declaration; ABI/layout
match mach/mach_time.h. No external native SDK/archive is added.

## Consequences

Phase 6 audio will reuse rezie-rt to elevate its block-rate mixer thread and
wait for deadlines. Keeping this in a separate crate avoids duplicating the
FFI or making rezie-audio depend on the engine.

macOS requests the programme period in Mach units, a 2 ms computation budget
and a 3 ms constraint for the 1.5 ms finishing spin at 50 fps; query the applied
policy and record it. Windows pairs every successful timer/MMCSS acquisition
with its matching release. Linux requests FIFO priority 10; if denied, it
creates a monotonic timerfd and attempts nice -10. Record every failure code
and report the fallback from startup/control, never from the steady-state loop.
Do not claim an elevated nice value if the OS denies it.

Sleep/wait to deadline minus an initially 1.5 ms finishing slack, then spin.
Calibrate that constant using an idle pilot before the ten-minute run, record
the pilot, and explain the measurement in the code comment. Native timed
waiting is the prescribed pacing operation, not unrelated media-path I/O.

ADR 0021 supersedes the shared slack choice: defaults are per-platform and
provisional until the owner supplies both platform sweeps. Calibration adds
optional spin CPU accounting through existing OS APIs; normal waits retain no
profiling queries. The generic crate remains free of domain and clock types.

## Verification

The idle 60-second Apple M4/macOS pilot on 2026-09-05 retained 1.5 ms slack:
3,001 ordered ticks, exact PTS, zero skipped indices, Mach time-constraint
policy confirmed by readback. Lateness p50/p99/p99.9/max was
1.292/16.125/18.375/19 microseconds; final drift was 1.167 microseconds.
The raw samples are in `docs/benchmarks/phase-0-idle-pilot-macos-aarch64.json`.
This measures the combined priority and wait strategy, not slack alone, and
does not establish the ten-minute gate or other platforms' latency.

Review the safety comments and enforce the crate boundary, compile all target
implementations, test policy/error reporting and resource cleanup, then measure
the idle ten-minute distribution. Report privilege/runner requirements instead
of silently claiming a successful policy or relaxing latency thresholds.

## Revisit when

The platform boundary grows beyond clock scheduling, or a policy cannot meet
the target-platform latency bound.
