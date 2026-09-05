# 0017 — Gate clock acceptance on the full lateness distribution

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** human review
- **Affects:** SPEC §13, clock benchmark, hosted and reference CI

## Context

The prior clock derives PTS from one monotonic origin and an absolute index.
Its 1.151 ms final lateness did not characterize stalls earlier in the run:
maximum lateness was 139.018 ms with 22 late deadlines. Concurrent builds
worsened the measurement conditions; they are not an established cause of
the scheduling defect. The prior result is not valid acceptance evidence.

## Options

### A — Enforce the human-corrected idle-machine criterion

Measure every tick, verify contiguous indices and exact PTS, and require final
drift and maximum lateness strictly below one programme frame interval and
p99.9 lateness strictly below 5 ms over ten minutes on an otherwise idle machine.

### B — Keep final drift as the only latency gate

This lets an absolute-deadline clock hide earlier visible stalls. Rejected.

## Decision

Adopt A, superseding ADR 0009's acceptance rule. Preserve all observed lateness
samples in tick-index order and record nearest-rank p50/p99/p99.9/max in the JSON
benchmark report. Preallocate atomic sample slots before the clock starts;
the clock never allocates or sends samples to a potentially lossy queue.
Summarize and serialize only after the clock stops. Zero sample loss is checked.

Use a release-profile headless measurement executable, built before idle
measurement begins. Separate short/hosted correctness checks (ordering,
counts, exact PTS, queue isolation) from the idle latency gate. Hosted runners
must not assert latency. Local/reference timing runs must record the actual
scheduling policy and fail if the required setup fails; Linux explicitly
reports its permitted fallback and any missing scheduling privileges.

## Consequences

The old report remains historical evidence, not an acceptance pass. There is
no relaxed latency mode hidden in hosted CI. A correctly prioritized target
platform that cannot meet the bounds requires a proposed ADR and human review,
not a changed threshold. Measurement preparation and tests precede the idle run;
push/CI follow it, as the user directed.

## Verification

Tests of percentile boundaries and pass/fail evaluation, actual native policy
status, a ten-minute idle report containing all samples, and three-platform
correctness CI after the measurement. No completion summary until all gates pass.

## Revisit when

A target platform cannot meet the bound with correct native thread priority.
