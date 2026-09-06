# CI and reference execution

The repository is public. Standard hosted runners run correctness only; the
production reference remains Windows 11 / RX 6800 XT. See ADRs 0024 and 0026.

- `ci-fast`: Ubuntu formatting, Clippy, nextest and compile-fail doctests on
  every code push, including main. Branch HEAD and PR merge revisions are distinct checks.
- `ci-full`: PRs targeting main and manual dispatch only. No push trigger. Its three
  platform jobs require the fast job. Ubuntu reuses the completed Rust checks;
  Windows/macOS check platform-specific Rust paths. All three launch the package.
- `reference`: trusted main push, nightly schedule and manual main dispatch.
  All normative golden comparisons, benchmarks and soaks belong here. No PR
  trigger exists, and manual dispatch to a non-main ref is rejected by the job.

ci-fast and reference push triggers ignore `docs/**`, `**/*.md` and
`docs/decisions/**`. ci-full applies these exclusions inside its fast job so
required checks complete for docs-only PRs: classification and full-gate run,
all Rust builds and matrix jobs are skipped. A workflow-level path filter
would leave the protected branch's required check pending indefinitely.
Schedules/manual dispatches are intentional requests and are not path-filtered.
Ref-keyed concurrency cancels superseded runs. Fast and full Ubuntu jobs have separate caches so a debug-only cache cannot
prevent saving release artifacts. All hosted caches are isolated from reference caches. No retries are
configured for failed tests. nextest does not execute doctests, so the safe
thread-affinity compile-fail test still uses `cargo test --workspace --doc`.

## One PR per slice

Work on a slice branch and finish local/ci-fast checks before opening the PR.
Normally run ci-full once per slice, then merge; main's push runs only ci-fast
and reference, subject to their path filters. Manual dispatch remains available
to the owner. If another full run is needed, stop and explain it. In particular,
main moving after validation requires updating and revalidating the PR so an
untested integration cannot merge. Do not bypass checks to save a run.

Main requires PRs and the Actions `full-gate` check against up-to-date main,
with administrator enforcement and force-push/deletion disabled. No separate
human review count is imposed. Existing external-contributor Actions approval
is unchanged. Matrix caches may save in PR scope (isolated by GitHub from
trusted main); reference cache keys remain separate. Result-only gate jobs
have nothing to compile/cache.

## Repository settings and runner provisioning

Actions requires approval for **all external contributors**. Never approve a
fork workflow change that introduces self-hosted execution. The action policy
allows only the four exact pinned revisions used by these workflows, with
blanket GitHub-owned and verified-Marketplace allowances disabled. Update the
allowlist together with a reviewed action pin change.

The reference runner is not yet provisioned. Register it at
`https://github.com/mhaslar/hasastudio` only, using the repository's Settings →
Actions → Runners instructions. Do not register at organization/account scope.
It needs labels `self-hosted`, `Windows`, `rezie-reference`, Windows 11,
RX 6800 XT/Adrenalin, MSVC x64 tools/Windows SDK and an interactive desktop.
No secrets are required by the current workflows. Until hardware is available,
reference jobs can remain queued; this is not a successful measurement.
Windows now uses the accepted 1,000 µs default (ADR 0028). The reference
workflow writes each clock attempt to a fresh path; recorded load and the
margin rule govern evidence admission. The Phase 1 media/golden tooling is
still in progress; runner registration alone is not a completed phase gate.

## Historical wall-clock comparison (before ADR 0026)

The prior uncached matrix's measured job durations were 6m07s on macOS,
10m51s on Linux and 17m20s on Windows (run 33986748941). Main, branch and
PR triggers all used that same matrix, even for documentation-only commits.
A branch push with a matching open PR could start two independent matrices.

With no code changes, docs-only pushes/PRs now start no build jobs. For code,
initial warm-cache estimates are 1–3 minutes for a branch push and 3–7 minutes
for a main push or PR merge check, excluding external approval and runner queue
waits. A cold full run can be slower because the fast job intentionally
precedes the matrix. These estimates are not a measured speedup; observed cold
and warm runs are recorded separately in `docs/ci/timing-evidence.json`.
An open PR update also checks its branch revision through ci-fast; do not add
those parallel wall times together or pretend the revisions are identical.
Reference runtime is separate from hosted CI: the Phase 0 clock benchmark and
30-minute soak alone require 40 minutes of sampling when actually runnable.

The first successful restructured cold run (33990508830, revision a0e44b4)
completed in **12m41s** from first job start to last job completion, including
2m41s for the fast gate. This is 4m40s shorter than the old 17m21s end-to-end path (17m20s slowest job);
resource changes associated with public runners may also contribute, so it
is not a cache-only causal comparison. The same-revision warm run 33991160147 completed in **5m09s**, with a **58s**
fast gate. A subsequent correction separates fast/full Ubuntu keys after the
warm log exposed redundant release compilation. The measured 5m09s predates
that correction; no additional speedup is claimed without measurement.

| Event | Before (same uncached matrix) | After |
| --- | --- | --- |
| Documentation-only push/PR | About 17m21s | Zero build jobs; observed on closure commit 36ba587 |
| Branch code push without a PR | About 17m21s | Fast only: 58s observed warm; budget 1–3 minutes |
| Main code push | About 17m21s | 12m41s observed cold; 5m09s observed warm |
| PR targeting main | About 17m21s per matrix | Estimate 3–7 minutes warm; same full pipeline, not separately PR-measured |

Times run from first job start to last job completion; approval and queue waits
are excluded. Changed dependencies can invalidate caches. The fast/full cache
key correction came after these measurements and is not an invented extra saving.

## Current event cost after ADR 0026

A code slice now has fast checks on branch pushes, one full run on its ready
PR, and only the fast hosted gate after merge. Main no longer runs a second
full matrix. Existing 58s fast / 5m09s full warm measurements are historical
estimates for these paths, not newly measured timings of this change. Docs-only
PRs run only classification and result jobs; no build jobs run. Do not trigger
extra matrices to obtain a new timing comparison. Reference remains independent.
