# Windows reference decode audit

**PASS for the native file decoder inventory.** The owner committed both
runs in `ad11a67`, measured against `e9c7605` on Windows 11 Pro build 26200,
i5-14600K, RX 6800 XT, driver 32.0.21045.5002. This is functional evidence,
not a preview, throughput, soak or Phase 1 completion result.

`python3 docs/testing/phase-1-decode-windows-x86_64/audit.py` independently
checks the retained reports against the fixture oracle and M4 report, without
calling Rezie or relying on its `passed` fields. Its output is `audit.json`.

| Check | Windows hardware | Windows forced software | M4 automatic |
| --- | --- | --- | --- |
| Pictures, seven files | 168 | 168 | 168 |
| Exact PTS/time bases and component hashes | All match | All match | All match |
| H.264 / HEVC 8-bit | D3D11VA, NV12 | Native software | VideoToolbox, NV12 |
| HEVC 10-bit | D3D11VA, P010LE | Native software | VideoToolbox, P010LE |
| VP9 | D3D11VA, NV12 | Native software | Explicit software fallback |
| AV1 8/10-bit | D3D11VA, NV12/P010LE | libdav1d | Explicit libdav1d fallback |

Every file has indices 0–23 in order and all delayed pictures are present.
Dimensions, component depth and the shared colour metadata also agree. The
oracle was authored independently of Rezie; canonical hashes normalize plane
packing, not decoded values. Raw YUV planes are not retained by this harness.

Hardware evidence includes actual AVHWFramesContext/device observations and
`d3d11` frames, with no fallback. Host inventory lists the RX 6800 XT; the
individual decoder's DXGI adapter identity is not serialized. Software evidence
has the override set, no hardware contexts and the expected software decoder.

Both build and loaded libavcodec report 61.19.101 and LGPLv3-or-later. Their
configurations exactly match the inspected pinned Windows artifact, with no
`--enable-gpl` or `--enable-nonfree`. Native dav1d reports **`7161642`**;
retain this build identifier without treating it as a semantic version.
M4 reports 1.5.4. All AV1 sample hashes agree without tolerance.

The native libclang version is not recorded. Rust's LLVM 22.1.8 is its own
backend and does not identify bindgen's library. This run proves the Windows
build worked, but does not independently establish the selected LLVM 21.1.8.

Both worktree records show deletion of two unrelated golden audit files, plus
new decode evidence. No source edits are recorded. `ad11a67` carried those
deletions; the audit change restores both JSON files byte-for-byte from
`e9c7605`, preserving prior evidence and its links. No reference pixels changed.
