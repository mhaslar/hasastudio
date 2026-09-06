# 0036 — Preflight platform CI and reject evidence deletion

- **Status:** Accepted
- **Date:** 2026-09-06
- **Phase:** 1
- **Decided by:** agent implementation of the owner's single-platform proposal
- **Affects:** hosted CI, evidence upload convention, phase checklist

## Context

PR #6 required three full runs. Ubuntu-only branch checks could not exercise
Windows linker setup or the Mac cache restoration failure. Repeating the
matrix to test a platform-specific workflow fix makes the one-full-run rule
difficult to follow.

Evidence disappeared in incoming commits `5282c5f` and `ad11a67`, before any
merge resolution: the former removed/moved old sweep files, the latter
deleted two golden audit JSON files. The recorded Windows worktree already
contained those audit deletions. Git history cannot establish which local
copy/sync command caused them. No merge-engine defect has been demonstrated.
Evidence-only pushes were excluded from workflows, and no retention check
protected old evidence paths. Existing generator overwrite protection cannot
prevent a manual directory replacement or broad git add from staging deletions.

## Options

### A — A manually dispatched platform workflow using the full job's steps

Expose Windows/macOS/Linux selection on a hosted reusable workflow. ci-full
calls the same workflow as its matrix, so a preflight validates the actual
job implementation rather than a reduced duplicate. Keep Ubuntu branch checks.

### B — Run the whole matrix on every branch push

Rejected: this restores the cost the owner explicitly removed. A platform
preflight is deliberate and scoped to the affected platform.

### C — Rely on review to catch evidence loss

Rejected as the sole defence: both losses passed ordinary compilation gates.

## Decision

Use A plus a cheap independent evidence-retention check. `ci-platform` accepts
one platform and the branch selected by workflow_dispatch; it has no automatic
push or PR trigger. It shares its implementation with ci-full through
workflow_call. A manual run performs the full platform correctness/packaging
job, including native guards, with no performance thresholds. It cannot emit
or substitute for the required `full-gate`. ci-full remains PR-to-main plus
manual full dispatch; the reference workflow remains trusted-main only.

Before opening the slice PR, preflight any changed platform-specific build,
packaging or CI configuration on the affected hosted platform. Iterate there;
then open one ready PR for the full matrix. Record the selected source SHA in
preflight output. Hosted caches stay separate from the self-hosted reference.

Run evidence retention on PRs and relevant pushes even when they only change
documentation. Compare Git trees with rename detection disabled and reject
removed paths under docs/testing and docs/benchmarks. New runs use new paths;
retain failed/superseded measurements and explain their status in notes. The
check does not claim to detect every in-place content edit or independently
approve changed golden references; those still need explicit review.

## Consequences

GitHub requires a dispatchable workflow on the default branch before its
manual entry point is available. This bootstrap slice therefore needs its
one ordinary PR matrix first. After merge, exercise a manual single-platform
run on a branch to verify the dispatch path. No self-hosted runner or secret
is needed. See [GitHub's dispatch rules](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow).

The evidence guard prevents silent deletion and broken historical paths. It
does not infer a user's file-manager actions or rewrite evidence. Genuine
removal requires a separately reviewed change to the retention policy; moving
files without updating their historical links is not routine cleanup.

## Verification

Check workflow syntax and shared-step equivalence before the bootstrap PR.
Exercise the retention script against a temporary Git repository with an
addition, deletion, rename, spaces in filenames and a non-evidence deletion.
The full matrix must pass once; the post-merge dispatch must run exactly the
chosen hosted platform on the selected branch, with no full-gate or reference job.

## Revisit when

New supported platforms appear, CI steps diverge, evidence storage moves, or
routine corrections need a stronger append-only evidence manifest.
