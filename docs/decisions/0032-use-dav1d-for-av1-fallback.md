# 0032 — Use dav1d through FFmpeg for software AV1 decode

- **Status:** Accepted
- **Date:** 2026-09-06
- **Phase:** 1
- **Decided by:** Human approval, with amendments on 2026-09-06
- **Affects:** `rezie-media`, native dependency manifest, SPEC §2.1/§3.1

## Context

SPEC §13 includes AV1 file decode with software fallback in Phase 1, while
§2.1 says fallback uses “FFmpeg’s own LGPL decoders”. FFmpeg 7.1.1's native
`av1` decoder returns `AVERROR(ENOSYS)` when no hardware accelerator is
configured; it cannot supply that software fallback. H.264, HEVC and VP9
have native software decoders and do not require this exception.

Primary evidence: [FFmpeg 7.1.1 av1dec.c, lines 612–623](https://github.com/FFmpeg/FFmpeg/blob/n7.1.1/libavcodec/av1dec.c#L612-L623).
FFmpeg already provides a [libdav1d wrapper](https://github.com/FFmpeg/FFmpeg/blob/n7.1.1/libavcodec/libdav1d.c).

The exact Windows archive approved in ADR 0004 was downloaded for read-only
inspection and its SHA-256 verified. Its `avcodec-61.dll` contains an embedded
configuration with `--enable-libdav1d`, `--enable-shared`, `--disable-static`
and `--enable-version3`; `--enable-gpl` and `--enable-nonfree` are absent,
while x264/x265 are disabled. It contains the `libdav1d` decoder name and
ships LGPL v3 in `LICENSE.txt`. The [artifact inspection record](../testing/phase-1-ffmpeg-windows-artifact-audit.json) retains the exact configuration and hashes. This is artifact inspection, **not** execution
of the Windows library or proof that a file decodes. Only the aggregate
licence file is present; third-party notices still need to be assembled
before redistribution. ADR 0004's Windows pin remains unchanged.

The development Mac's installed FFmpeg is 9.0.1 with GPL, x264 and x265
enabled. It cannot be selected by pkg-config for this project. An isolated
FFmpeg 7.x LGPL shared build is needed on macOS/Linux.

## Options

### A — Select FFmpeg's libdav1d decoder for AV1 software fallback

Use the existing FFmpeg API, keeping all FFmpeg access in `rezie-media`.
dav1d is a native dependency under BSD-2-Clause. Preserve its copyright,
conditions and disclaimer in distributed notices. There is no direct Rust
binding to dav1d and no separate frame representation.

### B — Keep native av1 as the only fallback

Rejected: with hardware disabled, FFmpeg 7.1.1 returns ENOSYS. Labelling
this a fallback would leave a required codec unavailable.

### C — Use libaom instead

Also requires an external native decoder and the same clarification to
§2.1. Prefer dav1d because the already-approved Windows bundle includes it
and FFmpeg has a dedicated integration. No Rezie performance comparison of
these libraries has been measured or is claimed by this decision.

## Decision

Select `libdav1d` for software AV1 decode;
retain FFmpeg's native H.264, HEVC and VP9 software decoders. Keep FFmpeg
dynamically linked and built without GPL/nonfree components. Hardware
selection and the explicit override that disables hardware remain required.

Replace only the decode sentence in SPEC §2.1 with:

> Decode always has a software fallback through the LGPL FFmpeg build:
> FFmpeg's native decoders for H.264, HEVC and VP9, and its libdav1d
> integration (BSD-2-Clause) for AV1.

Add the corresponding dav1d licence/notice requirement to §3.1. Leave encoder policy unchanged; extend §16 item 4 with the LGPLv3 distribution review below.

For isolated macOS/Linux builds, pin dav1d **1.5.4**, the latest release in
VideoLAN's official release index when checked on 2026-09-06:

- URL: https://download.videolan.org/pub/videolan/dav1d/1.5.4/dav1d-1.5.4.tar.xz
- Size: 1,038,852 bytes.
- SHA-256: `686616b7c69eb88d44459391ab25cac13b6647a3b288835c5784e71c1514a5c5`.
- `COPYING` inspected inside that exact archive: BSD-2-Clause.
- Manifest consuming phase: `from_phase: 1`; source build isolated from
  system pkg-config paths, shared dav1d and shared FFmpeg.

The Windows bundle retains its existing artifact pin; do not pretend its
embedded dav1d is necessarily 1.5.4. Record the actual bundled version
when validating it on Windows. Changing that bundle remains a separate ADR.

### LGPL version and why version3 is enabled

The pinned Windows build is **LGPL version 3 or later**, not LGPLv2.1.
The exact enabled configuration intersects FFmpeg 7.1.1's
`EXTERNAL_LIBRARY_VERSION3_LIST` at **gmp, libaribb24,
libopencore_amrnb and libopencore_amrwb**. FFmpeg's configure script rejects
these when version3 is disabled and selects LGPLv3-or-later when version3
is enabled without GPL/nonfree. This flag is required by this configuration,
not an unexplained build default. dav1d itself does not require version3.
See [the list](https://github.com/FFmpeg/FFmpeg/blob/n7.1.1/configure#L1883-L1892)
and [enforcement and licence selection](https://github.com/FFmpeg/FFmpeg/blob/n7.1.1/configure#L4493-L4518).

A future reduced build that excludes those components can prefer the weaker
LGPLv2.1-or-later obligations; **do not change the Windows pin in this slice**.
The isolated macOS/Linux decode build has no need for those components and
keeps version3 disabled. Record each build's actual licence version.

LGPLv3 remains compatible with the project's closed-source-capable desktop
model: use replaceable shared libraries under §4(d)(1), retain required
notices and corresponding source, and permit debugging modified libraries.
Dynamic linking addresses the shared-library/recombination condition, not
all distribution obligations. Commercial review under SPEC §16 item 4 must
cover the exact LGPLv3 bundle and packaging, including §4(e)'s conditional
installation-information provision; ordinary replaceable desktop libraries
must not become locked into an appliance-style distribution. This records
the approved distribution approach, not completed legal clearance.

### Build-time and startup rejection

At build time, load the selected libavcodec and call `avcodec_configuration`,
`avcodec_version` and `avcodec_license`. At application/headless startup,
query the libavcodec actually linked into that process again, before opening
media. Both checks fail with an error if configuration contains
`--enable-gpl` or `--enable-nonfree`, or the reported major is not **61**
(FFmpeg 7.x's libavcodec major). Include offending configuration, actual
version and expected major in diagnostics. There is no warning-only mode or
licence-check bypass. Reject a non-LGPL licence string too.

CI on Windows, macOS and Linux must exercise these guards, including
negative build-time and startup cases using deliberately incompatible
probe libraries. The real selected native library must also pass; a test
of a manifest string alone does not establish loaded-library compliance.

## Consequences

AV1 software fallback becomes testable on every supported target through
the same demux/decode interface. The software override must select
`libdav1d` explicitly for AV1, not simply reopen native `av1` without a
hardware device. Hardware and software runs must record the selected
codec and actual hardware context/pixel format; a codec name alone does
not prove a hardware accelerator was used.

The native bootstrap must reject accidental selection of the Mac's system
FFmpeg and verify the exact libraries/configuration actually loaded.
Retain FFmpeg and dav1d notices and identify bundled versions; no encoder
or later-phase feature is introduced by this decoder decision.

Version skew between the bundled Windows dav1d and macOS/Linux 1.5.4 is
**not a reason for different decoded pixels or a tolerance**: AV1 decoding
is normatively bit-exact. Conformant decoders given the same valid bitstream
and decode settings produce identical decoded samples, unlike encoders.
Compare native decoded planes before colour conversion; do not confuse GPU
colour-conversion rounding, optional processing or error concealment of
invalid input with decoder conformance. Skew matters for security patching
and reproducibility. Record versions and investigate any decoded-sample
mismatch rather than relaxing the comparison.

Golden tolerances remain mean ΔE00 <1 / max ΔE00 <3 on both platforms.
M4's observed maximum 0.00284570 gives no reason for a separate threshold.
Revisit only when Phase 3 introduces bilinear/Lanczos sampling and measured
backend differences justify a new reviewed decision. No threshold changes
are introduced here.

## Verification

Completed: inspected FFmpeg's pinned source, verified both archive hashes,
read dav1d's licence and the Windows bundle's embedded configuration.

Implementation verification: build the isolated LGPL configuration; record library paths,
build configuration, versions and notices; decode a known AV1 fixture with
hardware disabled on all supported targets, assert `libdav1d`, and validate
frames, PTS and end-of-stream handling. Verify actual hardware decode
separately on equipped machines. The implementation now passes local M4
software fixtures and hardware H.264/HEVC checks; Windows/Linux execution
remains to be recorded. See ADR 0034 for the actual Apple session probe.

## Revisit when

FFmpeg gains native software AV1 decode, a security update requires a new
pin, the selected build lacks libdav1d, or packaging changes the licence
or linkage of the dependency.
