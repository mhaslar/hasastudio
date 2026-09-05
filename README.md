# HasaStudio / Rezie

Cross-platform live production switcher. The normative design is
[docs/SPEC.md](docs/SPEC.md); contribution rules are in [AGENTS.md](AGENTS.md).

Phase 0 implements the engine/control foundation and an empty application
window. It emits timed `FrameTime` ticks with no pixel payload. GPU frames,
media inputs, mixing and output protocols begin in later phases.

Install the toolchain specified in `rust-toolchain.toml`, then:

```sh
cargo xtask fetch-deps
cargo xtask gen-assets
cargo build --workspace --locked
cargo run -p rezie-app
```

No FFmpeg, NDI, SRT or CEF installation is needed for Phase 0. `fetch-deps`
downloads and verifies the real `crossbeam-channel` archive consumed by this
workspace. Later entries are gated by the phase marker; NDI is never fetched.
Cargo resolves Rust dependencies separately using the committed lockfile.

```sh
cargo xtask ci
cargo xtask clock-check           # correctness only; suitable for hosted CI
cargo xtask bench                 # ten-minute latency gate; otherwise idle machine
cargo xtask dist --smoke          # package, launch, verify GUI + engine update
```

See [Foundation usage](docs/user/foundation.md) and
[Phase 0 progress](docs/phases/00-progress.md) for evidence and pending gates.
