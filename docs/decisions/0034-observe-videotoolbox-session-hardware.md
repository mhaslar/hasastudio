# 0034 — Expose the actual VideoToolbox session hardware property

- **Status:** Accepted
- **Date:** 2026-09-06
- **Phase:** 1
- **Decided by:** Agent implementation choice under required hardware verification
- **Affects:** isolated macOS FFmpeg source build, rezie-media

## Context

FFmpeg 7.1.1's HEVC VideoToolbox setup requests EnableHardware, not
RequireHardware. A hardware-format frame is therefore insufficient to prove
Apple did not use its internal software decoder. The modern device-context
path stores the live session in FFmpeg's private VTContext; the public
AVCodecContext.hwaccel_context is null. The first public-header-only probe
returned unavailable. Do not treat that as a measured software decoder.

## Options

### A — Assume VideoToolbox frames imply silicon decode

Rejected: it cannot distinguish Apple's permitted HEVC software fallback.

### B — Reproduce FFmpeg private layouts in Rezie

Rejected: a compatible replacement shared library could change those layouts,
creating undefined behaviour and undermining library replaceability.

### C — Add a small optional accessor inside the isolated FFmpeg build

The accessor resolves FFmpeg's own live session and queries Apple's actual
UsingHardwareAcceleratedVideoDecoder property, releasing the copied value.
Rezie sees only the stable AVCodecContext pointer and integer result.

## Decision

Use C. Apply `tools/patches/ffmpeg-7.1.1-vt-hardware.patch` only to the isolated
macOS build after verifying the original source archive. Retain the original
7.1.1 archive pin and **leave the Windows bundle byte-for-byte unchanged**.
There is no new SDK, licence version change or static FFmpeg link. The patch
is part of the native recipe/cache identity and corresponding-source record.
Mark the macOS build with `--extra-version=rezie-vt-probe1`.

Look up the accessor dynamically, so an unmodified, ABI-compatible LGPL
replacement library still runs: Auto reports unavailable hardware evidence
and selects software; RequireHardware fails explicitly. Never peek at a
private layout from application code. Hardware codec contexts use one FFmpeg
caller thread so the inspected session is the one that delivered the frame;
VideoToolbox's own asynchronous hardware processing remains available.

## Consequences

Hardware reports distinguish actual Apple session status from the generic
backend name, pixel format and frame/device contexts. A false/unavailable
property cannot pass a strict hardware check. Decode samples are unchanged.

This small source patch must accompany the original FFmpeg source and build
instructions before distribution. The bootstrap keeps source and patch
provenance; commercial distribution review remains open under SPEC §16.
The implementation stays on the specified FFmpeg/VideoToolbox architecture.

## Verification

Compare hardware H.264 and 8/10-bit HEVC frames/PTS against the independent
fixture oracle and assert session property true. Check Auto fallback and
forced software separately. No hardware result is inferred from compilation
or from the accessor's presence alone.

## Revisit when

Upstream FFmpeg exposes this property through a public API, the pinned source
changes, or hardware-thread throughput measurements justify changing scheduling.
