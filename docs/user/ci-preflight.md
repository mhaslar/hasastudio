# Checking one CI platform before opening a PR

Push the phase-slice branch, then open **Actions → ci-platform → Run workflow**.
Select that branch and one of `windows-2022`, `macos-14`, or `ubuntu-24.04`.
Run each affected platform when changing platform-specific build, packaging,
or CI configuration. The log records the checked-out commit; verify it is
the commit you intend to propose.

This runs the same platform job that ci-full calls: formatting, Clippy,
tests/doctests, build/startup native-library guards, forced-software decode,
clock correctness, and a packaged GUI launch. It has no performance gate.
Artifacts include native guard and decode reports and package smoke output.

Correct problems on the branch and repeat only the affected preflight. Once
it passes, open one ready PR and let ci-full run once. The preflight cannot
satisfy `full-gate`; main still requires the up-to-date PR matrix. A change
to main while the PR is open requires updating and revalidating the PR.

The initial introduction of this workflow requires one bootstrap PR matrix:
GitHub enables manual dispatch only after the workflow exists on main.
This does not require secrets, runner labels, or a reference runner.

## Uploading measurement evidence

Use a new directory for every run. Preserve earlier runs, including failed
or superseded measurements; add a note explaining their status. Before
committing, inspect `git diff --stat` and `git diff --cached --diff-filter=D`.
The evidence-retention check rejects deleted or renamed committed paths under
`docs/testing/` and `docs/benchmarks/`, including documentation-only changes.
Restore accidentally removed files from their earlier commit and add the new
run separately. The check preserves paths; it does not approve changes to
golden references or independently validate report contents.
