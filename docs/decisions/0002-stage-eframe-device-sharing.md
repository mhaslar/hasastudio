# 0002 — Stage eframe device sharing with the compositor

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** both
- **Affects:** rezie-app, SPEC §3

## Context

Phase 0 requires a runnable empty GUI bundle; the compositor device is Phase 1 work.

## Options

### A — Adopt the scoped change

Let eframe own its initial wgpu device.

### B — Alternative

Implement rezie-gpu ahead of Phase 1; violates build order.

## Decision

Use an empty eframe/wgpu application shell in Phase 0. Introduce shared-device rendering in Phase 1. Human approved this staging exception.

## Consequences

No preview or compositor is implemented now. The Phase 1 device migration is explicit follow-up work.

## Verification

Launch the packaged shell and require an actual GUI update before smoke-test success.

## Revisit when

Phase 1 device initialization.
