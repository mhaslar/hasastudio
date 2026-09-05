# HasaStudio / Rezie

Cross-platform live production switcher. The normative design is
[docs/SPEC.md](docs/SPEC.md); contribution rules are in [AGENTS.md](AGENTS.md).

**Phase 1 is in progress.** Phase 0 closed conditionally, not fully verified:
its [Windows reference clock obligation](docs/phases/OUTSTANDING.md) is due at
the Phase 1 gate. A failing reference measurement reopens Phase 0 and stops
Phase 1 until rezie-rt is fixed.

The engine/control foundation and empty GUI work. Phase 1 has begun with a
native GPU context and reusable Rgba16Float frame leases, tested functionally
on M4/Metal. The pool is not yet integrated into the engine/GUI; shared-device
preview, file/image/colour sources and NDI output remain in progress. See
[Phase 1 progress](docs/phases/01-progress.md).

Install the pinned Rust toolchain and cargo-nextest 0.9.143 (see
[CI/tooling notes](docs/user/ci.md)), then:

```sh
cargo xtask fetch-deps
cargo build --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo nextest run --workspace --locked
cargo test --workspace --doc --locked
cargo run -p rezie-app
```

macOS uses the calibrated 500 µs slack. Normal Windows/Linux startup explicitly
rejects their missing calibrated defaults; correctness harnesses provide an
explicit diagnostic value. The reference Windows sweep is still due. Native
dependency fetching is phase-gated and hash-verified; Phase 1 enables the pinned
Windows FFmpeg archive, which is not linked by this first GPU slice. NDI SDK is
never fetched, and CEF is never fetched before Phase 10.

```sh
cargo xtask clock-check           # hosted correctness only, explicit zero slack
cargo xtask clock-sweep           # manual M4/Windows calibration with CPU cost
cargo xtask bench                 # reference clock obligation; idle Windows RX 6800 XT
cargo xtask dist --smoke          # package and launch with explicit diagnostic slack
cargo run -p rezie-gpu --bin rezie-pool-check -- --output target/pool-check.json
```

Phase 0's `xtask gen-assets`, `golden` and aggregate `ci` commands deliberately
reject the unfinished Phase 1 media/golden scope; use the portable checks above.
No Phase 1 golden references or production performance result is claimed.

See [Foundation summary](docs/phases/00-summary.md),
[CI evidence](docs/user/ci.md), and [GPU pool check](docs/user/frame-pool-check.md).
