@AGENTS.md

## Claude Code specifics

Everything above applies. These are additions for this tool only.

### Plan mode

Use plan mode before changes to:

- `crates/rezie-gpu/` — the compositor and its shaders, where regressions are least visible
- `crates/rezie-core/` — the domain model; a change here ripples through every crate
- `crates/rezie-api/` — the command/event boundary; breaking it breaks the test harness
- Anything touching the threading model or the clock

### Reading order for a new session

1. This file and `AGENTS.md`
2. The "Current state" phase marker at the top of `AGENTS.md`
3. `docs/phases/` — the most recent summary
4. `docs/decisions/` — skim titles, read anything relevant to the area you are touching
5. The relevant section of `docs/SPEC.md`

Do not read `docs/SPEC.md` end to end unless you are starting a new phase. Read the sections the work touches.

### Verification

Run `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` before reporting anything as done. Do not report a change as working on the basis that it compiles.

Golden-frame tests need `cargo xtask gen-assets` to have been run at least once in the working tree.

### Long-running commands

`cargo xtask soak` and `cargo xtask bench` run for tens of minutes. Do not start them speculatively; run them at phase gates or when explicitly asked.
