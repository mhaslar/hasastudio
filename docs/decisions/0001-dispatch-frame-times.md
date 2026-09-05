# 0001 — Dispatch frame times without pixel payloads

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** both
- **Affects:** rezie-core, rezie-engine, SPEC §13

## Context

Phase 0 needs clock, dispatch and bounded-queue evidence, while §7.1 allows only GPU Rgba16Float premultiplied frames and Phase 1 introduces FramePool.

## Options

### A — Adopt the scoped change

Emit FrameTime ticks without pixels.

### B — Alternative

Allocate CPU black buffers; rejected by human review because a second frame representation would outlive its intended temporary use.

## Decision

Emit FrameTime through the real sink dispatch path. Do not introduce CPU pixels or a CPU frame type. Amend Phase 0 wording accordingly.

## Consequences

GPU resources and pixel/golden-frame paths begin in Phase 1. Phase 0 verifies tick content, dispatch isolation and drop accounting, not image correctness.

## Verification

Clock integration tests, bounded dispatch tests and the ten-minute measured run. Golden command explicitly reports zero compositor paths in Phase 0.

## Revisit when

Phase 1 introduces FramePool.
