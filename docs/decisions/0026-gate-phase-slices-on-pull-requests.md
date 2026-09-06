# 0026 — Gate phase slices on pull requests

- **Status:** Accepted
- **Date:** 2026-09-06
- **Phase:** 0
- **Decided by:** both
- **Affects:** hosted workflows, main protection, AGENTS.md

## Context

Pushing slices directly to main caused repeated three-platform matrices.
The owner requires branch development, one PR and normally one ci-full run
per slice, with no push-to-main full run. Main was unprotected at review.

## Options

### A — Retain the main-push matrix as insurance

Duplicates work without preventing an unvalidated merge. Rejected by the owner.

### B — Require the PR's full gate against current main

Validate the proposed integration before merging and enforce it in protection.
An intervening main change requires updating the PR and another validation;
this is an exceptional integration change, not a routine duplicate run.

## Decision

Adopt B. Develop on a branch per phase slice, finish fast/local checks, open
one ready PR, run ci-full once and merge after it passes. ci-full triggers only
on pull_request targeting main and workflow_dispatch. Keep synchronization
events so changes to an open PR invalidate the preceding result. Stop and
explain additional runs; never suppress validation of changed code to meet a
run-count target. ci-fast continues on every code push, including main.
The reference workflow retains its existing trusted-main triggers unchanged.

Protect main with required PRs, no administrator bypass, no force push/deletion,
an up-to-date requirement and the GitHub Actions `full-gate` check. Require no
second-person approval: the owner authorized the agent's PR/merge workflow and
this single-owner repository must remain operable. This is separate from the
existing mandatory approval of all external contributors' Actions runs.

An always-emitted gate is necessary: GitHub leaves workflow-path-filtered
required checks pending, blocking documentation-only PRs. Replace ci-full's
workflow-level paths-ignore with a lightweight changed-file classification
using the same docs/** and **/*.md exclusions. No builds run for docs-only
PRs. ci-fast and reference retain their workflow-level filters. Classify the
entire PR diff, including deletions and both sides of renames, without a
300-file API/path-filter truncation; an inability to classify fails closed.
Manual dispatch always requests the full matrix. The final gate requires
classification success and either a confirmed docs-only change or success of
both fast checks and every matrix entry. Bind protection to the Actions app.

Full-job caches can save in PR scope as well as manual-main scope; otherwise
removing main-push matrices would prevent refreshing those caches. GitHub's
PR cache scope is not restored onto trusted main. Reference cache separation,
read-only workflow permissions and the exact action allowlist are unchanged.

## Consequences

No full matrix runs on merge. Docs-only PRs use small gate jobs, not compilers
or three platform runners. A main update after validation requires an updated
PR and another run; explain that exception rather than merge an untested
combination. Workflow edits themselves require review because contributors can
edit their own workflow; external Actions approval remains mandatory. Protected
checks are not a substitute for reviewing malicious changes to the gate itself.

## Verification

Inspect actual triggers, branch protection readback and one PR run. Check
classification for root Markdown, docs files, mixed changes and renames out of
docs. Assert failure/cancellation cannot make full-gate succeed. Do not dispatch
an extra full run merely to measure CI speed. Record the actual result in the PR.

Local verification passed YAML/trigger checks, the actual classification
script against temporary Git history (docs, code, rename, dispatch, push and
invalid refs), and all 48 combinations of fast/matrix outcomes and build flags.
Formatting, strict workspace Clippy, 20 nextest checks and both compile-fail
doctests passed. No runtime or benchmark implementation changed.

Repository settings were applied and read back on 2026-09-06: strict
`full-gate` bound to GitHub Actions app 15368, PR required (zero additional
review approvals), administrators enforced, force pushes/deletion disabled.
`all_external_contributors` Actions approval remains active. The single PR
run and merge will provide integration verification; do not infer them from
this settings readback.

GitHub documents [skipped required checks](https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/troubleshooting-required-status-checks)
and [strict protected-branch checks](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches).

## Revisit when

Concurrent slices repeatedly invalidate each other's integration results, or a
merge queue is introduced (which would require a separately reviewed trigger).
