# 0012 — Use correlated JSON commands for Foundation control

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** agent
- **Affects:** rezie-api, rezie-engine

## Context

The same engine behavior must work through in-process calls and a local WebSocket without blocking the control thread.

## Options

### A — Use the scoped Foundation design

Foundation supports GetState, SetProjectName and Shutdown, with request IDs and State/Applied/Rejected events. SetProjectName is absolute intent; invalid names are rejected without changing authoritative state. No later mixer or media commands are advertised. Both transports submit to the same bounded crossbeam control channel with a bounded per-request reply. GUI consumes arc-swap snapshots. WebSocket accepts loopback addresses only, bounds messages/connections, limits request duration, and rejects malformed input. Shutdown cancels listener and connection tasks.

### B — Alternative

Separate WebSocket state or optimistic GUI updates violate engine ownership. Implementing later commands as no-ops would be stubs.

## Decision

Adopt option A. This is an implementer choice within the human-approved Phase 0 scope.

## Consequences

Foundation supports GetState, SetProjectName and Shutdown, with request IDs and State/Applied/Rejected events. SetProjectName is absolute intent; invalid names are rejected without changing authoritative state. No later mixer or media commands are advertised. Both transports submit to the same bounded crossbeam control channel with a bounded per-request reply. GUI consumes arc-swap snapshots. WebSocket accepts loopback addresses only, bounds messages/connections, limits request duration, and rejects malformed input. Shutdown cancels listener and connection tasks.

## Verification

WebSocket integration tests for mutation, rejection, malformed input and shutdown; in-process tests for matching state/revisions.

## Revisit when

API evolution in subsequent phases.
