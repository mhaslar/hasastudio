# 0016 — Enable the empty shell's native wgpu backends

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** agent
- **Affects:** workspace dependencies, rezie-app

## Context

The actual packaged macOS launch failed: eframe 0.32.3's `wgpu` feature with
default features disabled does not enable any native wgpu backend. Compilation
and headless tests passed, but wgpu panicked before a GUI update. This is an
implementation configuration error, not an architectural change.

## Options

### A — Explicitly enable the prescribed native backends

Add the already-transitive wgpu 25.0.2 as a workspace dependency and consume
it only in rezie-app with `wgsl`, `metal`, `dx12` and `vulkan`. This
enables the locked stack's backends without implementing a compositor.

### B — Restore every eframe default

This also enables backends, but adds the unrelated OpenGL GUI path and other
defaults. The spec specifies wgpu, so explicitly selecting its features is
more precise.

## Decision

Use A within the user-approved Phase 0 eframe-device exception (ADR 0002).
wgpu is MIT/Apache-2.0; its native backends use the platform's graphics APIs.
No additional media/native SDK is fetched, linked or licensed by this change.

## Consequences

The shell can create its own temporary device. rezie-core, rezie-api and
rezie-engine still do not depend on wgpu or eframe. Shared compositor-device
ownership remains Phase 1 work. No platform-specific GUI source code is added.

## Verification

Rebuild the bundle and require the actual GUI/engine smoke marker. Re-run
workspace checks after updating Cargo.lock. The failed launch is retained in
the phase progress account as evidence of why compilation is insufficient.

## Revisit when

Phase 1 transfers device creation to rezie-gpu.
