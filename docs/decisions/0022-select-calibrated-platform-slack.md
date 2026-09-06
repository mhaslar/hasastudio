# 0022 — Select measured macOS slack and reject uncalibrated defaults

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** human review
- **Affects:** rezie-rt, engine startup, correctness harness

## Context

The M4 sweep in ADR 0021 has essentially flat tail latency from 0.5 to 5 ms.
At 0.5 ms, spin used 2.338% of one core, versus 7.330% at 1.5 ms. The owner
explicitly selected 0.5 ms now and deferred finer calibration. Windows has no
reference measurement. Its value must not inherit the Mac configuration.

## Options

### A — Retain a guessed cross-platform default

Hides missing Windows evidence and wastes Mac CPU. Rejected.

### B — Store an optional independently calibrated value per platform

Mac uses measured 500 µs. Uncalibrated platforms fail normal startup explicitly.

## Decision

Adopt B. macOS is Some(500 µs); Windows and Linux are None. Linux remains a
correctness target and no calibration claim is invented for it. The safe API
returns an actionable missing-calibration error before starting engine threads.
Exact caller-provided overrides remain available for calibration experiments.
Hosted correctness and GUI smoke paths explicitly use zero slack, record/log
that diagnostic choice, and never evaluate latency. Tests exercise native
priority/waiting with explicit budgets; they do not silently change defaults.
The WebSocket harness can provide an explicit diagnostic slack argument.

## Consequences

A regular Windows/Linux application or headless launch fails until its platform
has a reviewed calibrated value; explicit diagnostics remain executable. The
reference sweep can explore Windows values without a compiled default. The
reference ten-minute run must use a reviewed override or the eventual pin.
No assumed Windows timer-granularity prediction becomes a constant. Finer Mac
sampling is optional follow-up, not phase debt or a second outstanding item.

## Verification

Assert that missing calibration fails before startup, measured Mac default is
500 µs, and explicit zero-slack correctness runs retain ticks/PTS/queue isolation.
Keep the native thread-affinity compile-fail doctest and unwind cleanup tests.

## Revisit when

The Windows reference sweep selects its value, or new platform measurements
justify a change. ADR 0019's verified Rust 1.98.1 pin stands unchanged.

## Subsequent owner ruling — ADR 0028

The Windows v2 result is accepted under SPEC §13's recorded-load / tenfold-margin
rule. Windows is pinned to 1,000 µs; the Phase 0 obligation is PAID and its
ledger removed. Phase 0 is closed and Phase 1 resumes. Earlier pending or
INADEQUATE statements in this ADR are historical. Observed-PTS serialization,
accounting limitations, preferred power plan and evidence protection remain.
