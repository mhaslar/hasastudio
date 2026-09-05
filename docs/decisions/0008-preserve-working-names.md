# 0008 — Preserve the specified working names

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** both
- **Affects:** SPEC, workspace naming

## Context

User requested exact evidence for HasaStudio rather than an inferred naming decision.

## Options

### A — Adopt the scoped change

Report existing occurrences and retain rezie-* crate paths.

### B — Alternative

Rename the product or claim the string is absent; unsupported.

## Decision

Before amendments docs/SPEC.md contains HasaStudio at lines 1, 21, 23, 27 (twice), 485, 618 and 770. These are the title, §1 heading/opening/graphics paragraph, rundown editing requirement, SPX forward compatibility and §16 item 1. Keep product codename HasaStudio and prescribed rezie-* crates; final branding remains a Phase 11 human decision.

## Consequences

Line numbers after spec edits differ; use rg -n HasaStudio docs/SPEC.md for current locations.

## Verification

Record the exact search output and check that no renaming was introduced.

## Revisit when

Phase 11 branding review.
