# 0020 — Provision the existing X11 backend's runtime library

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** agent
- **Affects:** Linux CI environment, Linux setup documentation

## Context

Acceptance run 33983189864 built the Linux bundle, then its real GUI launch
panicked in the already-locked xkbcommon-dl 0.4.2 loader because
`libxkbcommon-x11.so.0` was absent. Installing `libxkbcommon-dev` did not
provide this separate X11 runtime component. Headless tests and clock
correctness had passed; neither establishes GUI launchability.

## Options

### A — Install the selected backend's existing runtime prerequisite

Add Ubuntu's `libxkbcommon-x11-0` package to the runner setup and document
it for Linux users. Keep the actual packaged GUI smoke test.

### B — Skip the smoke test or change the selected windowing backend

This hides a required launch failure or changes application configuration
to accommodate an incomplete test environment. It does not fix provisioning.

## Decision

Use A as a correction to the existing approved eframe/X11 environment.
This introduces no Rust dependency, new application/native interface, or
change in linkage/licensing: xkbcommon-dl is already in the initial lockfile
and this library is already loaded by the selected backend. The user has
authorized fixing and executing the prepared three-platform workflow.

The existing libxkbcommon family uses permissive MIT-style licences;
see its [upstream licence](https://github.com/xkbcommon/libxkbcommon/blob/master/LICENSE).
Ubuntu supplies the runtime through its standard package repository
([Noble package](https://packages.ubuntu.com/noble/amd64/libxkbcommon-x11-0/download)).
No library is vendored or added to the portable bundle.

## Consequences

Linux installations must supply this runtime when using X11. CI installs
it explicitly so a compile-only pass cannot conceal the missing library.
The clock implementation and the completed idle measurement are unchanged.

## Verification

Require a new three-platform acceptance run, including Linux
`xvfb-run -a cargo xtask dist --smoke`, to pass. Preserve the failed run as
evidence; do not convert it into a pass by weakening the smoke condition.

## Revisit when

The windowing backend changes or Phase 11 packages system prerequisites.
