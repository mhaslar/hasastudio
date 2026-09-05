# 0009 — Make the one-frame clock drift bound normative

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** both
- **Affects:** SPEC §13, rezie-core, rezie-engine, benchmark harness

## Context

50.000 ±0.005 fps permits roughly three frames of error in 600 seconds; it is weaker than under one frame.

## Options

### A — Adopt the scoped change

Measure absolute monotonic-time drift at 50 fps over ten minutes.

### B — Alternative

Retain the fps tolerance as an alternative pass condition; rejected by human review.

## Decision

Remove fps tolerance. Require drift strictly less than 20 ms over a ten-minute 50 fps run. Schedule each tick from one Instant origin plus its absolute frame index, never cumulative sleeps. Measure observed emission time against scheduled time, verify indices/PTS and count, report maximum lateness, deadline misses and queue drops. The acceptance run must emit all 30,001 ticks from index 0 through 30,000 with no skipped indices; require final drift under 20 ms, and report timing excursions without hiding them behind an average.

## Consequences

Throughput fps may be diagnostic only. Pure clock tests cannot replace the real-time ten-minute run. No reference-machine result may be inferred from a developer machine.

## Verification

Ten-minute engine timing test and committed machine-labelled benchmark report, plus deterministic deadline/PTS tests.

## Revisit when

Reference-hardware measurements fail the criterion; stop and request review.
