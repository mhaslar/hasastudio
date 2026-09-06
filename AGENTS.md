# AGENTS.md

Working agreement for coding agents in this repository. Read this fully before your first edit in a session.

**Rezie** is a cross-platform live video production switcher in Rust. The full specification is `docs/SPEC.md` — it is normative. This file is only about *how we work here*. When the two disagree, the spec wins for **what** to build, this file wins for **how**.

---

## Current state

> **Phase: 1 — Media foundation and first picture**
> Maintained by hand. Update it when a phase gate passes. Never work ahead of it.

Read `docs/phases/` for what previous phases concluded before starting anything. If `docs/phases/NN-summary.md` exists and records a closed current phase, this line is stale — say so rather than starting the next phase. Exception: a failed outstanding measurement can explicitly reopen a conditionally closed phase; update its summary status and marker, preserve the historical evidence, and stop the next phase as specified below.

---

## Non-negotiables

These break silently and expensively. They are not style preferences.

1. **Never block the composite thread.** No `Mutex`, no allocation, no I/O, no logging that can block, no GPU buffer creation. Cross-thread state uses `arc-swap` snapshots and bounded `crossbeam` channels. If you find yourself wanting a lock there, the design is wrong — stop and write an ADR.

2. **Never allocate GPU resources outside `FramePool`,** and never grow the pool from the composite thread. Steady-state allocation count must stay at zero; there is a test asserting this.

3. **The engine owns all state.** The GUI sends commands and renders events. It never mutates its own copy optimistically, and it never contains logic the engine lacks. Any feature reachable only through the GUI is a bug — it must be expressible as a command in `rezie-api`.

4. **All compositing happens in linear light, `Rgba16Float`, premultiplied alpha.** Conversion in and out happens exactly twice: ingest and egress. If you add a stage that converts anywhere else, you have introduced fringing that no one will notice for six months.

5. **M/E programme output is always clean.** Overlays composite per-output, never into the mix. See SPEC §7.5.

6. **No `unwrap()` or `expect()` on any path reachable from user action.** In tests, freely.

7. **No `unsafe` outside `rezie-ndi`, `rezie-media`, `rezie-capture`, `rezie-html`, `rezie-rt`.** Every `unsafe` block carries a `// SAFETY:` comment stating the invariant that makes it sound.

   `rezie-engine`, `rezie-core`, `rezie-api`, `rezie-audio`, `rezie-rundown`, and `rezie-app` must use crate-level `#![forbid(unsafe_code)]`. The allowed crates own foreign-interface wrappers.

8. **No stubs in merged work.** `todo!()` and `unimplemented!()` are permitted only behind a feature flag for an explicitly deferred later phase, and only with a tracking note in the phase summary.

---

## Commands

```bash
cargo xtask fetch-deps          # native deps (FFmpeg, SRT, CEF); hash-verified. Run first.
cargo xtask gen-assets          # generate tests/assets/ — required before golden tests

cargo build --workspace
cargo nextest run --workspace
cargo test --workspace --doc
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all

cargo run -p rezie-app                          # the GUI application
cargo run -p rezie-engine --bin rezie-headless -- --ws 127.0.0.1:9800   # headless, for the harness

cargo xtask golden                # golden-frame tests
cargo xtask golden --update       # regenerate references — NEVER without human review
cargo xtask bench                 # writes docs/benchmarks/
cargo xtask soak --minutes 30
cargo xtask dist                  # platform bundle
```

Before proposing any change as finished: `fmt`, `clippy`, and portable
nextest and doctest checks must pass on all three supported platforms. Phase 0's
`golden` command checks the pixel-free inventory. From Phase 1, normative
`golden` comparisons run only on the Windows 11 / RX 6800 XT reference machine.
Optional hosted lavapipe smoke is non-blocking and never updates references.
Every compositor/shader change must run on that reference machine and on
M4/Metal before a phase gate; M4 success is necessary, never sufficient.

`cargo xtask golden --update` overwrites the reference images that are the only defence against silent compositor regressions. Running it because a test failed is how the defence gets deleted. If a golden test fails, either fix the code or explain in your summary exactly why the new output is correct — and then ask before updating.

---

## Crate boundaries

```
rezie-rt        realtime thread configuration, safe deadline waiter, platform FFI; no domain types
rezie-core      domain model, project state, command bus, clock, scheduler.  Depends on nothing else here.
rezie-gpu       wgpu device, FramePool, shaders, compositor graph
rezie-media     FFmpeg: demux, decode, encode, mux; source/sink traits
rezie-audio     mixer, buses, DSP, resampling, metering
rezie-ndi       NDI SDK runtime loader, receiver, sender, discovery
rezie-net       SRT/RTMP/RTSP ingest; MPEG-TS/UDP/SRT egress
rezie-capture   screen and window capture, per platform
rezie-html      HTML source client + helper-process IPC          (phase 10)
rezie-rundown   rundown model, YAML, scheduler
rezie-engine    assembles all of the above into a runnable headless engine
rezie-api       command/event types; in-process + WebSocket transports
rezie-app       egui GUI; the shipped binary
```

**Nothing may depend on `rezie-app`.** The engine must link and run with no GUI code present — that property is what makes every feature testable, and it is easy to destroy by accident.

`rezie-core` must not depend on `wgpu`, FFmpeg, or any platform API. If a domain type needs one of those, the type is in the wrong crate.

Adding a workspace dependency requires an ADR. Adding a *native* dependency requires an ADR and a note on its licence — see SPEC §3.1, the licensing constraints there are load-bearing.

---

## Conventions

- Rust stable, edition 2021. MSRV pinned in `rust-toolchain.toml`; bumping it needs an ADR.
- `#![warn(missing_docs)]` on every library crate. Public items are documented.
- `thiserror` in libraries, `anyhow` at the binary boundary.
- Errors carry the specifics. `"failed to open input"` is useless. `"failed to open input 'vt_mtb_report' (/media/vt/mtb.mp4): no video stream found"` is not.
- `tracing`, not `println!`. Spans on anything that crosses a thread boundary.
- Commits: conventional format, with the phase — `feat(phase-4): overlay z-ordering`, `fix(phase-2): t-bar direction on reversed transition`.
- Tests live next to the code for unit work, in `tests/integration/` for anything driving the engine.
- Platform-specific code goes behind `#[cfg]` in the crate that owns the concern, never in `rezie-core` and never in the GUI.

---

## Definition of done, per phase

A phase is complete when **all** of these hold:

- [ ] Every acceptance criterion in `docs/SPEC.md` §13 passes on its tagged target
- [ ] CI green on Windows, macOS, and Linux
- [ ] At most one ci-full run per phase slice under normal circumstances. If another is needed, stop and explain what changed or what is wrong with the workflow.
- [ ] No `todo!()` / `unimplemented!()` outside deferred feature flags
- [ ] ADRs written for every implementer's-choice decision taken
- [ ] `cargo xtask bench` run on Windows 11 / RX 6800 XT, results committed to `docs/benchmarks/`; a manual run is valid and runner automation does not block Phase 0
- [ ] From Phase 1: reduced M4 diagnostic (2 inputs, 1 output, 1080p50) recorded at the gate, with no performance threshold
- [ ] Frame time has not regressed more than 10% versus the previous phase, or the regression is justified in an ADR
- [ ] User documentation written for the features added, in `docs/user/`
- [ ] `docs/phases/NN-summary.md` written: what was built, what was deferred and why, what surprised you, what the benchmarks said
- [ ] The "Current state" line at the top of this file updated

Do not begin the next phase in the same change as completing one.

### Conditional closure (ADR 0023)

Only explicit human approval permits conditional closure; never infer it.
Record the unpaid obligation in `docs/phases/OUTSTANDING.md` and say plainly
in the phase summary that the phase is not fully verified.

- At most **ONE** outstanding item exists at any time. While one is open,
  no phase may close conditionally. This is a hard cap, not guidance.
- Phase N+1 cannot close until Phase N's outstanding item is paid.
  Phase 0's reference-clock item is due at the Phase 1 gate.
- If the Windows reference measurement fails, Phase 0 reopens and Phase 1
  work stops until `rezie-rt` is fixed. Never relax the latency criterion.

### Public CI and reference-runner security (ADR 0024)

Work on a branch per phase slice. Finish branch/local checks before opening
one ready PR; let ci-full validate it once, then merge. Never push slices
directly to main. Main requires an up-to-date PR and the Actions full-gate
check, including for administrators. If main moves after validation, update
and revalidate the PR; explain this exceptional additional run.

`ci-fast` performs Ubuntu checks on every code push, including main; `ci-full`
gates its three-platform matrix behind that fast check only for PRs targeting
main or explicit workflow_dispatch. It has no push trigger. Documentation-only
pushes do not run workflows; documentation-only PRs emit a lightweight required
gate and skip every build (ADR 0026). Use nextest plus the separate
compile-fail doctest, caching and ref-keyed cancellation; no hosted latency gate.

The `reference` workflow may run only on trusted main via push, schedule or
manual dispatch. **Never add a pull_request or pull_request_target trigger.**
Require Actions approval for **all external contributors**. A fork PR must never
run on the production machine; do not approve workflow changes introducing
self-hosted PR execution. Register the reference runner to this repository
only, never an account or organization. Keep reference caches separate from
hosted PR caches. All normative goldens, benchmarks and soaks run there.


---

## When to stop and ask

Stop, write an ADR describing the problem and your proposed change, and request human review — do not proceed on your own judgement:

- A phase reveals that the architecture in SPEC §5–§11 is wrong or unworkable
- You want to add a native dependency, or change how an existing one is licensed or linked
- An acceptance criterion cannot be met on the reference hardware
- Golden-frame references need updating
- You want to change the rundown or project file schema after phase 9
- Anything in SPEC §16 (open items requiring human decision) becomes relevant

Silent deviation from the architecture is the single worst failure mode on this project. A stopped session costs an hour. A quietly reinvented compositing model costs a phase.

---

## Mistakes specific to this project

Written down because they are easy to make and hard to notice.

- **Optimistic GUI updates.** Feels responsive, produces a UI that lies about what is on air. Always render engine state.
- **Compositing in sRGB.** Everything looks fine until an alpha logo sits over a bright background and grows a dark halo.
- **Correcting audio drift by dropping samples.** Audible click every time. Always resample (SPEC §8.4).
- **Assuming NVENC.** The reference GPU is AMD. Encoder selection is AMF on Windows, VAAPI on Linux, VideoToolbox on macOS, with software fallback.
- **Decoding every configured input.** With 50 inputs this melts the machine. Respect the Cold/Warm/Hot policy in SPEC §5.4, and pre-warm from the rundown lookahead.
- **Blocking a sink and stalling everything.** Each output has a bounded queue and drops its own oldest frame on overflow. One slow receiver must never affect the other three.
- **Frame-rate blending by default.** `Nearest` is the default for a reason (SPEC §7.3). Blending ghosts on motion.
- **Confirmation dialogs on on-air controls.** Take, cut, and overlay buttons act immediately. Never put a modal in front of them.
