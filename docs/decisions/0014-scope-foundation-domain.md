# 0014 — Define domain data without later-phase behavior

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** agent
- **Affects:** rezie-core

## Context

Phase 0 explicitly includes domain types while later phases own their execution and persistence.

## Options

### A — Use the scoped Foundation design

Implement §6 identifiers, configuration and runtime-state data in core, with format/clock invariants, serde data representation and documented units. Domain structs represent the specified future inputs, outputs, mixers and settings without opening, routing, processing or saving them. Initial engine exposes only Foundation name/state/lifecycle. Do not create empty later-phase crates; add them when they contain working code. No project file schema, YAML persistence, resource promotion, DSP, transitions or media processing in Phase 0.

### B — Alternative

Implement entire future subsystems now or use empty placeholder types; both break scope or misrepresent completeness.

## Decision

Adopt option A. This is an implementer choice within the human-approved Phase 0 scope.

## Consequences

Implement §6 identifiers, configuration and runtime-state data in core, with format/clock invariants, serde data representation and documented units. Domain structs represent the specified future inputs, outputs, mixers and settings without opening, routing, processing or saving them. Initial engine exposes only Foundation name/state/lifecycle. Do not create empty later-phase crates; add them when they contain working code. No project file schema, YAML persistence, resource promotion, DSP, transitions or media processing in Phase 0.

## Verification

Domain serialization and clock tests; dependency graph audit keeps core free of platform/GPU APIs.

## Revisit when

A consuming phase refines its configuration types before Phase 9 schema stability.
