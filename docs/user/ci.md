# CI and reference execution

The repository is public. Standard hosted runners run correctness only; the
production reference remains Windows 11 / RX 6800 XT. See ADR 0024.

- `ci-fast`: Ubuntu formatting, Clippy, nextest and compile-fail doctests on
  every code push. On main, `ci-full` calls it and the standalone event starts
  no redundant runner. Branch HEAD and PR merge revisions are distinct checks.
- `ci-full`: PRs targeting main, main pushes and manual dispatch. Its three
  platform jobs require the fast job. Ubuntu reuses the completed Rust checks;
  Windows/macOS check platform-specific Rust paths. All three launch the package.
- `reference`: trusted main push, nightly schedule and manual main dispatch.
  All normative golden comparisons, benchmarks and soaks belong here. No PR
  trigger exists, and manual dispatch to a non-main ref is rejected by the job.

All push/PR triggers ignore `docs/**`, `**/*.md` and `docs/decisions/**`.
Schedules/manual dispatches are intentional requests and are not path-filtered.
Ref-keyed concurrency cancels superseded runs. Rust caches are shared between
compatible hosted jobs and isolated from reference caches. No retries are
configured for failed tests. nextest does not execute doctests, so the safe
thread-affinity compile-fail test still uses `cargo test --workspace --doc`.

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
The uncalibrated Windows default also causes a clear benchmark failure until
its manual sweep selects the value. Follow `clock-calibration.md` first.

## Wall-clock comparison

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
2m41s for the fast gate. This is 4m39s shorter than the old 17m20s slowest job;
resource changes associated with public runners may also contribute, so it
is not a cache-only causal comparison. A same-revision warm run is in progress.
