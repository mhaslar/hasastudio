# 0011 — Pin the Foundation toolchain and Rust dependencies

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** agent
- **Affects:** workspace

## Context

There was no Rust toolchain or code. Reproducible Foundation builds need one supported stable toolchain and only phase-consuming packages.

## Options

### A — Use the scoped Foundation design

Rust 1.88.0, edition 2021, resolver 2 with incompatible-rust-versions fallback, Cargo.lock committed. Use serde 1, serde_json 1 for JSON wire messages, thiserror 2, anyhow 1, tracing 0.1, tracing-subscriber 0.3, tracing-appender 0.2; arc-swap 1.7, crossbeam-channel exactly 0.5.15 and crossbeam-queue 0.3; tokio 1 with only I/O/runtime/signal/time/sync features, tokio-tungstenite 0.27 and futures-util 0.3 for WebSocket; eframe 0.32.3 with wgpu/default_fonts/x11/wayland only. xtask uses sha2 0.10 and platform curl for HTTPS, serde_json manifests, anyhow, tracing, and rezie-engine for the real benchmark. All are pure Rust direct dependencies except the user-approved eframe platform integration; no media SDK is linked. Cargo locks exact transitive versions.

### B — Alternative

Floating stable and all future stack dependencies would add unreviewed churn and later-phase code.

## Decision

Adopt option A. This is an implementer choice within the human-approved Phase 0 scope.

The toolchain pin is superseded by [0019](0019-pin-current-stable-rust.md).

## Consequences

Rust 1.88.0, edition 2021, resolver 2 with incompatible-rust-versions fallback, Cargo.lock committed. Use serde 1, serde_json 1 for JSON wire messages, thiserror 2, anyhow 1, tracing 0.1, tracing-subscriber 0.3, tracing-appender 0.2; arc-swap 1.7, crossbeam-channel exactly 0.5.15 and crossbeam-queue 0.3; tokio 1 with only I/O/runtime/signal/time/sync features, tokio-tungstenite 0.27 and futures-util 0.3 for WebSocket; eframe 0.32.3 with wgpu/default_fonts/x11/wayland only. xtask uses sha2 0.10 and platform curl for HTTPS, serde_json manifests, anyhow, tracing, and rezie-engine for the real benchmark. All are pure Rust direct dependencies except the user-approved eframe platform integration; no media SDK is linked. Cargo locks exact transitive versions.

## Verification

Build/test/clippy on the pinned compiler; cargo metadata confirms no engine dependency on GUI.

## Revisit when

Dependency additions, security update or MSRV bump.
