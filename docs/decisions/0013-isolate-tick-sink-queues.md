# 0013 — Isolate tick sinks with bounded crossbeam queues

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** agent
- **Affects:** rezie-engine

## Context

Every slow sink must drop its own oldest tick and never block clock dispatch. crossbeam-channel does not expose atomic overwrite-oldest.

## Options

### A — Use the scoped Foundation design

Use crossbeam_queue::ArrayQueue<FrameTime> with force_push for sink queues, and bounded crossbeam-channel for control work. Preallocate queues, handles, stats and thread resources before clock starts. Only one clock producer; each sink has one consumer. force_push reports the evicted oldest tick and increments that sink counter. Clock updates only atomics, stack values and bounded queue entries; no logging, I/O, heap allocation, GPU objects or user Mutex. Absolute clock pacing uses sleep then a short spin; the prescribed timed wait is distinct from blocking on work or I/O. On lateness retain every index and catch up against the absolute origin; measure/report misses.

### B — Alternative

Emulate overwrite using try_send/try_recv races, or block until a sink consumes; loses exact accounting or stalls production.

## Decision

Adopt option A. This is an implementer choice within the human-approved Phase 0 scope.

## Consequences

Use crossbeam_queue::ArrayQueue<FrameTime> with force_push for sink queues, and bounded crossbeam-channel for control work. Preallocate queues, handles, stats and thread resources before clock starts. Only one clock producer; each sink has one consumer. force_push reports the evicted oldest tick and increments that sink counter. Clock updates only atomics, stack values and bounded queue entries; no logging, I/O, heap allocation, GPU objects or user Mutex. Absolute clock pacing uses sleep then a short spin; the prescribed timed wait is distinct from blocking on work or I/O. On lateness retain every index and catch up against the absolute origin; measure/report misses.

## Verification

Tests stall one sink while another drains, check newest retained ticks and independent counts; real-time benchmark reports timing and all drops. Review the small dispatch loop for prohibited operations.

## Revisit when

Phase 1 attaches FramePool-backed payloads; allocation instrumentation must obey unsafe restrictions.
