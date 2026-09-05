# 0003 — Gate dependency downloads by consuming phase

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** both
- **Affects:** xtask, dependency manifest, SPEC §3.1

## Context

Fetching roughly a gigabyte of CEF on every clean checkout is unacceptable; NDI requires user licence acceptance. Phase 0 must test fetching against a real dependency.

## Options

### A — Adopt the scoped change

Use per-entry from_phase and verify immutable downloads.

### B — Alternative

Download all native dependencies immediately, or test only a mock downloader; both rejected by human review.

## Decision

No entry is fetched before from_phase. CEF is prohibited before Phase 10 regardless of manifest mistakes; NDI SDK is never fetched and remains user-installed, runtime-loaded, optional. Phase 0 fetches crossbeam-channel 0.5.15, an actual workspace dependency, and verifies the crates.io SHA-256 82b8f8f868b36967f9606790d1903570de9ceaf870a7bf9fbbd3016d636a2cb2. Source archives remain in ignored cache, never vendored.

## Consequences

Cargo still resolves Rust packages. This additional verified cache exercises the same downloader that will handle native bundles. No FFmpeg/SRT/CEF consumer is added now. Future native entries require review.

## Verification

Real HTTPS fetch, cached-file revalidation, corruption rejection, manifest phase filtering and explicit NDI/CEF guard tests.

## Revisit when

A native consuming phase starts.
