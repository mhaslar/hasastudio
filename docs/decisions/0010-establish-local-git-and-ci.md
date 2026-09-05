# 0010 — Establish local Git and prepare three-platform CI

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** both
- **Affects:** repository, CI, phase gate

## Context

User included git init, first commit, remote and CI in Phase 0, then clarified there is no GitHub repository and it will be added later.

## Options

### A — Adopt the scoped change

Initialize and commit locally; prepare executable GitHub workflows and defer remote execution per the later instruction.

### B — Alternative

Invent a remote or claim CI passed from local tests; unacceptable.

## Decision

Run git init and create conventional Phase 0 commits. Prepare Windows/macOS/Linux checks, dist smoke tests and timing runs. Do not create or push a GitHub repository until the user supplies it. Remote setup and hosted CI evidence remain pending, so keep Phase 0 active.

## Consequences

Record available evidence and outstanding gates in a progress document; reserve docs/phases/00-summary.md for actual completion because AGENTS.md treats its existence as a completed phase.

## Verification

Local commit exists; workflow reviewed and local equivalent checks run. Hosted results explicitly pending.

On 2026-09-05 the owner attached `Github-HasaStudio`, pointing to
`https://github.com/mhaslar/hasastudio.git`, and explicitly authorized push
and workflow execution after the idle measurement. Commit `d1aac59` was
pushed successfully; push-triggered acceptance run `33983189864` started.
GitHub reported no registered repository self-hosted runners, so the nightly
`self-hosted` / `rezie-reference` job still needs a provisioned machine.

## Revisit when

User adds the GitHub repository.
