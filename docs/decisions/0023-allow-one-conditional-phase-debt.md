# 0023 — Close Phase 0 conditionally with one dated gate obligation

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** human review
- **Affects:** SPEC §§0/13, AGENTS.md, phase summaries and OUTSTANDING.md

## Context

The owner explicitly permits Phase 1 to start while reference hardware is
unavailable. M4 evidence is valid development evidence; it does not verify
Windows MMCSS latency or replace the production benchmark.

## Options

### A — Pretend M4 passes the Windows gate or defer indefinitely

Both lose the original acceptance obligation. Rejected.

### B — Conditional closure with one explicit debt due at the next gate

Allows work to proceed with a hard cap and a written reopening consequence.

## Decision

Adopt B. Close Phase 0 conditionally after its remaining correctness work
passes. Write 00-summary.md explicitly saying it is not fully verified and
advance the marker to Phase 1. Start Phase 1 in a separate implementation commit.
Record exactly one outstanding item:

> Phase 0 — clock benchmark on Windows 11 / RX 6800 XT reference machine,
> including the slack sweep for that platform. Blocked on hardware availability.
> Due at the Phase 1 gate.

## Consequences

At most ONE outstanding item may exist. While it is open no phase may close
conditionally. Phase N+1 cannot close until Phase N's outstanding item is paid.
If the Windows measurement fails, Phase 0 reopens and Phase 1 work stops until
rezie-rt is fixed. Do not weaken the clock thresholds. Record this consequence
in OUTSTANDING.md and the working agreement before work proceeds.
This supersedes ADR 0021's no-closure-before-reference-result policy only as
explicitly stated here; all production evidence assignments remain in force.

## Verification

The phase summary distinguishes measured M4/CI evidence from missing Windows
evidence. OUTSTANDING.md has exactly one open item and its next-gate deadline.
The current marker advances only after the conditional closure is recorded.

## Revisit when

The Windows reference result is supplied. Success pays the item; failure
reopens Phase 0 and halts Phase 1 pending correction.
