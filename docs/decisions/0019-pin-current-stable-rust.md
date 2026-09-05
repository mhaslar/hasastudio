# 0019 — Pin the verified current stable Rust release

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** both
- **Affects:** rust-toolchain.toml, workspace MSRV, CI

## Context

Rust 1.88.0 was chosen by the implementer, not supplied by the sandbox. The
earlier ADR gave no project-specific compatibility reason to remain on it.
The user requests current stable or an explicit justification for an older pin.

## Options

### A — Pin current stable 1.98.1

The official stable manifest fetched on 2026-09-05 reports date 2026-09-03,
version 1.98.1 (48a229cea 2026-09-01). Pin exactly that release.

### B — Retain 1.88.0

No validated compatibility constraint requires it. Rejected.

## Decision

Adopt A. Supersede the toolchain part of ADR 0011, retaining its dependency
decisions. Update workspace rust-version to 1.98.1 and all CI toolchain installs.
Preserve the dependency lock unless the compiler or necessary scheduling
bindings require a specific change. Edition stays 2021.

Source: https://static.rust-lang.org/dist/channel-rust-stable.toml . The
downloaded manifest is discovery evidence; the committed exact version makes
future builds reproducible without following a moving stable alias.

## Consequences

The new compiler and clippy must pass all existing checks. The prior benchmark
is labelled with its original toolchain; new measurements use 1.98.1 and record
that version. A future MSRV change still requires an ADR.

## Verification

rustc --version, formatting, strict clippy, workspace tests, golden inventory,
packaged launch, idle clock measurement, then the three-platform workflow.

## Revisit when

A compiler compatibility/security issue or next deliberate toolchain update.
