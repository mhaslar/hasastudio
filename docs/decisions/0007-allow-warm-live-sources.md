# 0007 — Allow Warm live sources

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** both
- **Affects:** SPEC §5.4 and Phase 7 acceptance

## Context

Cold NDI was both disconnected and connected at lowest bandwidth.

## Options

### A — Adopt the scoped change

Separate Cold, Warm and Hot live-source states.

### B — Alternative

Keep Cold connected; conflicts with its definition and obscures bandwidth accounting.

## Decision

Cold is disconnected; Warm is connected for multiview only (NDIlib_recv_bandwidth_lowest for NDI); Hot is connected at full bandwidth. Replace the Phase 7 Cold NDI acceptance assertion with Warm. The NDI SDK flag applies only to NDI; other live transports retain transport-specific implementation for their phase.

## Consequences

No NDI receiver or live-source behavior is implemented in Phase 0.

## Verification

Spec consistency now; SDK bandwidth assertion in Phase 7.

## Revisit when

Phase 7 implementation needs further transport-specific human decisions.
