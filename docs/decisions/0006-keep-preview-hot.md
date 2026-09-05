# 0006 — Keep preview inputs Hot

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** both
- **Affects:** SPEC §5.4, future resource policy

## Context

Preview was simultaneously listed as Hot and Warm; the operator is viewing it.

## Options

### A — Adopt the scoped change

Use the human-corrected promotion precedence Hot then Warm then Cold.

### B — Alternative

Treat invisible preview as Warm; ambiguous and rejected.

## Decision

Hot: any programme or preview bus, enabled overlay, output source, or explicitly playing. Warm: disabled overlay, rundown lookahead or explicit operator warming. Cold: otherwise. Preview is Hot. Amend §5.4 without implementing promotion behavior before its consuming phases.

## Consequences

Types may express states now; actual promotion logic is deferred.

## Verification

Review spec now; resource behavior tests in consuming phases.

## Revisit when

A future human-reviewed resource-policy change.
