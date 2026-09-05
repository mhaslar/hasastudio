# 0004 — Pin the Windows FFmpeg 7.1 LGPL shared bundle

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** both
- **Affects:** xtask manifest, future rezie-media

## Context

User requires a specific 7.1.x LGPL build and hash without fetching it in Phase 0. Windows requires a prebuilt shared bundle.

## Options

### A — Adopt the scoped change

Pin BtbN n7.1.1-57-g1b48158a23, LGPL shared, autobuild-2025-08-31-13-00.

### B — Alternative

Use latest, a GPL bundle or statically linked libav; violates reproducibility or §3.1.

## Decision

Pin https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2025-08-31-13-00/ffmpeg-n7.1.1-57-g1b48158a23-win64-lgpl-shared-7.1.zip with SHA-256 e1b36a261be92f632e8e4d35077a6154520c686dd0bb451508bfefdcd4804f55, from_phase 1, Windows x86_64. Digest and artifact identity were read from the GitHub release API before coding. This is an LGPL shared build (LGPL 2.1-or-later/3-or-later depending on enabled components); exact bundled notices must be retained and audited before linking/distribution. No --enable-gpl or --enable-nonfree, no in-tree x264/x265. macOS/Linux use pkg-config in Phase 1.

## Consequences

No FFmpeg archive is downloaded or linked in Phase 0. At Phase 1, verify hash, ffmpeg -version/-buildconf, configure flags, libav shared linking and included licence notices before use. The dated upstream artifact may eventually disappear; a missing artifact must fail rather than fall back to latest.

## Verification

Manifest asserts exact URL/hash and Phase 0 exclusion. Source: https://api.github.com/repos/BtbN/FFmpeg-Builds/releases/tags/autobuild-2025-08-31-13-00 ; variant definitions: https://github.com/BtbN/FFmpeg-Builds .

## Revisit when

Phase 1 native consumption, unavailable upstream artifact or security update.
