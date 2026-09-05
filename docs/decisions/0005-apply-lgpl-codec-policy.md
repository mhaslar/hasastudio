# 0005 — Apply the LGPL codec fallback policy

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 0
- **Decided by:** both
- **Affects:** SPEC §2.1, future rezie-media

## Context

§2.1 software x264/x265 fallback contradicted §3.1.

## Options

### A — Adopt the scoped change

Make §3.1 authoritative as ruled by the user.

### B — Alternative

Provide GPL software fallback; conflicts with closed-source-capable linking.

## Decision

Software decode uses FFmpeg LGPL decoders. Software encode fallback is OpenH264 for H.264 only. HEVC encode requires hardware; absence is an explicit GUI error, never a silent failure or downgrade. Amend §2.1.

## Consequences

This records the human ruling, not legal advice or a codec implementation. §16 item 2 remains for commercial review in Phase 5.

## Verification

Review amended text now; codec tests belong to later phases.

## Revisit when

§16 item 2 becomes relevant in Phase 5.
