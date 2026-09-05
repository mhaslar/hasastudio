# 0024 — Publish the repository and separate hosted checks from trusted measurement

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** both; public disclosure and security rules by human review
- **Affects:** repository visibility/settings, three workflows, developer tooling

## Context

The owner approves public disclosure of the repository, SPEC and every ADR.
Uncached three-platform runs took about 17 minutes in the last measured run.
Documentation-only commits also built all platforms. A public repository with
self-hosted execution requires an explicit boundary against fork PR code.

## Options

### A — Keep the unconditional uncached matrix and permit PR reference runs

Wastes wall time and exposes the production machine to untrusted code. Rejected.

### B — Cache hosted correctness and restrict production execution to trusted main

Use three workflows, an Ubuntu prerequisite, and an explicit runner trust policy.

## Decision

Make mhaslar/hasastudio public after reviewing tracked content/history for
specific accidental secret material. Retain the commercial linking/licensing
constraints; public source visibility does not authorize GPL/native changes.

Use ci-fast (push on every branch plus workflow_call), ci-full (PRs targeting
main, pushes to main, workflow_dispatch), and reference (push to main, schedule,
workflow_dispatch only). All push/PR triggers ignore docs/**, **/*.md and
explicit docs/decisions/**. Scheduled/manual reference measurements still run
when requested; path filters do not apply to those event types.

ci-full calls ci-fast with an explicit full_gate input and its matrix uses
needs: ci-fast. The standalone ci-fast main push allocates no runner; the full
workflow performs that fast check once. Other branch pushes check branch HEAD;
PRs check the proposed merge revision, a distinct correctness obligation.
The full Ubuntu job packages/launches and checks tick correctness; its Rust
checks already passed in ci-fast. Windows/macOS additionally run Clippy and
nextest. Use ref-keyed cancel-in-progress concurrency, distinguishing called
fast checks from their parent to avoid self-cancellation.

Swatinem/rust-cache runs on every runner job, with separate hosted/reference
keys and no reference cache writable by fork PRs. Use cargo-nextest for unit
and integration executables. Keep cargo test --workspace --doc because stable
nextest does not execute doctests, including our thread-affinity compile-fail
assertion. These are development-only tools, not native/application dependencies.
Pin third-party action revisions and cargo-nextest 0.9.143 (official release
resolved 2026-09-05); retain Rust 1.98.1.
Prefer existing long jobs over fragmented setup/check jobs.

No hosted latency or performance gate. All normative golden comparisons,
benchmarks and soaks run in reference only. Phase 0 has no pixel golden paths;
its inventory is a correctness check, not a frame comparison. Reference builds
finish before measurement. Reference hardware is currently unavailable; do not
fabricate provisioning or a successful measurement.

## Consequences — mandatory security boundary

1. reference has NO pull_request or pull_request_target trigger. Its job also
   requires this repository and refs/heads/main; manual dispatch cannot run an
   arbitrary feature revision on production hardware. Never add a PR trigger.
2. Repository Actions setting: Require approval for all external contributors
   (all_external_contributors). Review all workflow/code changes before approval;
   approval is a trust decision, not a routine retry button. Hosted PR jobs have
   read-only permissions and do not receive production secrets.
3. Register the runner at repository scope only, with the configuration URL
   https://github.com/mhaslar/hasastudio and labels self-hosted + rezie-reference
   (Windows). Never account/organization scope. No runner currently exists to
   register remotely; provisioning remains separate from phase debt.

These prevent current fork PR workflows from targeting the reference runner.
Approval alone does not make malicious workflow edits safe: never approve a
fork workflow that adds self-hosted execution, and review trusted-main changes
before merging. The reference workflow uses its own cache namespace so it
cannot restore attacker-produced hosted PR build products.

## Verification

Read back public visibility, external-contributor approval policy and repository
runner inventory. Validate event filters, needs and runner labels. Run hosted
checks on all three platforms and compare actual job/run elapsed times against
the previous baseline. Report warm-cache estimates separately from observed
cold-cache runs; do not manufacture a before/after speedup. Doc-only pushes
must schedule zero builds. No untrusted PR is used to test the real runner.

## Sources

- https://docs.github.com/en/actions/how-tos/manage-runners/self-hosted-runners/add-runners
- https://docs.github.com/en/rest/actions/permissions
- https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows
- https://github.com/Swatinem/rust-cache
- https://nexte.st/ (doctests require cargo test --doc)

## Revisit when

Runner scope, public visibility, workflow triggers or the trust model changes.

## Settings readback — 2026-09-05

The repository was already public when checked. Repository administrator access
is available. The fork policy was initially first_time_contributors and was
changed to all_external_contributors; API readback confirmed that value. The repository runner endpoint
reports zero runners, so no runner registration/scope is claimed as complete.
A scan of all 129 historical file blobs found no private-key or common live
credential-token patterns; no specific tracked file was identified for exclusion.
This targeted check is not a guarantee that arbitrary secrets cannot exist.

The first workflow attempt scheduled zero jobs because the repository's
allowed_actions policy was local_only, including rejection of actions/checkout.
Use selected actions with only the four exact commit pins in the workflows;
github_owned_allowed and verified_allowed stay false. This permits the explicitly
requested cache/nextest setup without blanket third-party access. Keep the
all_external_contributors approval rule unchanged. Record readback before retry.

The first executable fast run caught an omitted fetch-deps prerequisite: its
real-dependency verification test failed while the other 18 tests passed. The
needs gate skipped the entire matrix. Restore fetch-deps before tests, cache
its hash-reverified .deps files, and retain trusted-main caches on failure.
No test or acceptance assertion was removed. Local execution already included
the prerequisite, which explains why this was a clean-run CI setup failure.
