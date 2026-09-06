# Native file decode checks (Phase 1)

The decoder backend handles local H.264, HEVC, VP9 and AV1 video in MP4/MOV,
MKV and TS. It retains original PTS and colour metadata. This slice does not
yet connect decoded pictures to the GUI preview or NDI.

FFmpeg is isolated in `.deps/native`. Do not point the application at a
system FFmpeg: builds and engine startup reject GPL/nonfree configuration or
any libavcodec major other than 61. Windows retains the approved LGPLv3 bundle;
macOS/Linux build LGPLv2.1 FFmpeg 7.1.1 with dav1d 1.5.4. The Mac build includes
the small optional session-property accessor in ADR 0034; it does not change
decoded pixels. Unmodified compatible libraries remain usable via explicit
software fallback. See ADRs 0032–0034.

## Prepare the native libraries

Use the Rust version in `rust-toolchain.toml`. Native builds also need Python
3.12 or later and a C toolchain. macOS needs Command Line Tools and pkg-config.
Ubuntu needs pkg-config, clang, nasm, python3-venv, libva-dev and patchelf.
Windows needs MSVC build tools/Windows SDK and LLVM's libclang available to
bindgen (typically `C:\Program Files\LLVM\bin`; set `LIBCLANG_PATH` to that
directory if it is not discovered). Guard fixture tests also use `clang`.

Run each command separately. **“then” is prose, not a shell separator.**

```text
cargo xtask native-deps
```

Unix activation, in the same shell as subsequent Cargo commands:

```bash
source .deps/native-env.sh
```

PowerShell activation can use the generated `. .\.deps\native-env.ps1`.
Equivalent inline commands, which need no script-execution-policy change:

```powershell
$env:FFMPEG_DIR = (Resolve-Path .deps/native).Path
$env:PATH = "$env:FFMPEG_DIR\bin;$env:PATH"
```

The bootstrap fetches only eligible hash-pinned artifacts, then builds or
extracts them. It does not install over system libraries. Keep `.deps` out
of Git. The source archives and build recipe remain available for licence
review; the development package is not commercial distribution clearance.

## Windows reference: hardware and forced software

Update the checkout to `phase-1/media-decode`, prepare/activate native
libraries as above, then run these individual PowerShell commands:

```powershell
Remove-Item Env:REZIE_DISABLE_HW_DECODE -ErrorAction SilentlyContinue
cargo xtask decode-check --mode hardware --output docs/testing/phase-1-decode-windows-x86_64/hardware
$env:REZIE_DISABLE_HW_DECODE = '1'
cargo xtask decode-check --mode auto --output docs/testing/phase-1-decode-windows-x86_64/software
Remove-Item Env:REZIE_DISABLE_HW_DECODE
```

Commit the whole `docs/testing/phase-1-decode-windows-x86_64` directory,
including `report.json`, `native.log` and `stdout.log` under both runs.
No idle preparation is needed: these are correctness checks, not timing
measurements. A failed check still writes its report/logs; preserve those
and report the error. Existing output directories are never overwritten.

The default inventory has seven small fixtures: 8-bit H.264/MP4 and TS,
HEVC/MOV, VP9/MKV and AV1/MKV, plus 10-bit HEVC/MOV and AV1/MKV. Each has 24
pictures, including delayed codec output drained at EOF. The oracle records
independent ffprobe PTS and decoded sample hashes; it is not derived from
Rezie's decoder. Component hashes normalize byte packing and row padding,
not sample values; there is no pixel tolerance. The tool records actual
hardware contexts/formats and captures the native dav1d version log.

`--mode hardware` rejects software fallback. `--mode auto` tries native
hardware, records any fallback reason and reopens software without repeating
already-delivered frames. `REZIE_DISABLE_HW_DECODE=1` forces software and is
asserted in the report. `--mode software` directly requests software.

To inspect a particular file, add `--input path/to/file.mp4`. Unknown files
have no fixture-oracle comparison; the report explicitly records null rather
than claiming their pixels were verified. Normative preview/load tests remain
separate Phase 1 work.

## Portable guard checks

```text
python tools/check-native-guards.py
cargo nextest run --workspace --locked
cargo test --workspace --doc --locked
```

On Unix use `python3` if `python` is unavailable. The guard script compiles
isolated incompatible native libraries and verifies both the actual build
script and shared startup-guard implementation. It writes
`target/native-guard-tests.json`. CI runs it on all three platforms and also
checks the real approved library and forced software decode. Hardware tests
require equipped hosts; hosted CI does not claim hardware correctness.
