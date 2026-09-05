# 0015 — Package the empty application and make checks explicit

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** agent
- **Affects:** xtask, CI, docs

## Context

Foundation must yield a runnable bundle on three operating systems without claiming pixels or installers.

## Options

### A — Use the scoped Foundation design

dist builds release app/headless binaries, creates macOS .app with Info.plist, a Windows portable directory and Linux portable directory with desktop entry. Bundle smoke mode starts the engine and actual eframe event loop and succeeds only after a GUI update. macOS/Linux/Windows CI run normal tests, formatting, clippy, explicit Phase 0 golden inventory, ten-minute timing verification and packaged launch. gen-assets writes only a Phase 0 manifest explaining there are no media assets. golden verifies that manifest and rejects unexpected reference images rather than reporting fictitious frame comparisons; golden --update is not available in Phase 0. bench and soak exercise actual clock/dispatch. Nightly jobs target a self-hosted reference-machine label and remain pending until provisioned.

### B — Alternative

Claim no-op golden tests exercise a compositor or produce signed installers ahead of Phase 11; misleading or out of scope.

## Decision

Adopt option A. This is an implementer choice within the human-approved Phase 0 scope.

## Consequences

dist builds release app/headless binaries, creates macOS .app with Info.plist, a Windows portable directory and Linux portable directory with desktop entry. Bundle smoke mode starts the engine and actual eframe event loop and succeeds only after a GUI update. macOS/Linux/Windows CI run normal tests, formatting, clippy, explicit Phase 0 golden inventory, ten-minute timing verification and packaged launch. gen-assets writes only a Phase 0 manifest explaining there are no media assets. golden verifies that manifest and rejects unexpected reference images rather than reporting fictitious frame comparisons; golden --update is not available in Phase 0. bench and soak exercise actual clock/dispatch. Nightly jobs target a self-hosted reference-machine label and remain pending until provisioned.

## Verification

Execute every documented Phase 0 command locally where possible and record platform limitations without claiming hosted CI passed.

## Revisit when

Phase 1 adds pixels/golden paths, Phase 11 adds installers.
