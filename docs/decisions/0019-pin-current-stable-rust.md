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

The pin was challenged and rechecked directly on 2026-09-05. Literal file:

```toml
[toolchain]
channel = "1.98.1"
profile = "minimal"
components = ["clippy", "rustfmt"]
```

Actual `/tmp/rezie-cargo/bin/rustc -vV`, with RUSTUP_HOME=/tmp/rezie-rustup
and CARGO_HOME=/tmp/rezie-cargo:

```text
rustc 1.98.1 (48a229cea 2026-09-01)
binary: rustc
commit-hash: 48a229ceaefd4985c50990b14116b6d856af0985
commit-date: 2026-09-01
host: aarch64-apple-darwin
release: 1.98.1
LLVM version: 22.1.8
```

Neither RUSTUP_DIST_SERVER nor RUSTUP_UPDATE_ROOT was set. Fresh HTTPS GETs
of the official stable and `channel-rust-1.98.1.toml` manifests both returned
HTTP 200, date 2026-09-03 and the same rustc commit. Their SHA-256 was
`a7c8774a5fd8441c997d94c029776cbc5eb111e9d72ab5d256fa69866644347e`.
The [official release announcement](https://blog.rust-lang.org/2026/09/03/Rust-1.98.1/)
independently records the September 3 point release. The exact pin is unchanged.

## Revisit when

A compiler compatibility/security issue or next deliberate toolchain update.
