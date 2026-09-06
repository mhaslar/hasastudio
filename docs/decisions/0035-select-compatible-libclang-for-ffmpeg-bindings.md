# 0035 — Select a libclang compatible with the pinned FFmpeg bindings

- **Status:** Accepted
- **Date:** 2026-09-06
- **Phase:** 1
- **Decided by:** agent, build-tool compatibility within ADR 0033
- **Affects:** native build prerequisites and Windows decode instructions

## Context

The Windows reference checkout has LLVM 22.1.8. After correcting header
discovery, ffmpeg-sys-next 7.1.3 generated Rust structures of size one for
AVFormatContext (expected 472), AVHWAccel (24), and other complete C types.
Compile-time layout assertions correctly stopped the build. No decoder ran.

The pinned sys crate requires bindgen 0.70, resolved to 0.70.1. LLVM 22
changed declaration handling in libclang; old bindgen can mistake a complete
definition following a forward declaration for an opaque type. Upstream
[issue 3264](https://github.com/rust-lang/rust-bindgen/issues/3264) and the
[0.72.1 release](https://github.com/rust-lang/rust-bindgen/releases/tag/v0.72.1)
document the defect and fix. A plain Cargo update cannot select bindgen
0.72.1 under the sys crate's 0.70 requirement.

The prior unversioned LLVM installation advice was incomplete and selected
this incompatible combination. This is a build-tool failure, not reference
hardware decode failure or an FFmpeg shared-library ABI mismatch.

## Options

### A — Select compatible libclang for the existing binding generator

Use LLVM 21.1.8 for manual Windows validation. Keep matching clang.exe and
libclang.dll paths explicit and use the MSVC developer environment. Existing
working pre-22 libclang installations on other platforms can remain in use.

### B — Fork the sys crate to upgrade bindgen, or upgrade the FFmpeg wrappers

This can support LLVM 22, but expands the maintained dependency change beyond
the present decoder validation. Revisit as a deliberate binding update;
do not edit the user's Cargo registry or mislabel a newer bindgen as 0.70.

### C — Disable layout assertions or hand-edit generated Rust structures

Rejected: this would hide invalid FFI layouts rather than correct them.

## Decision

Use A. Document LLVM 21.1.8 and a dedicated installation directory for the
Windows reference build. Select both CLANG_PATH and LIBCLANG_PATH from that
installation; retain all generated layout assertions. Clean only the sys
crate's build products after changing libclang, to force regeneration.

No Rust toolchain, FFmpeg/dav1d pin, runtime licence or linking mode changes.
LLVM is an existing build prerequisite, not a new shipped native library.

## Consequences

Unqualified installation of the latest LLVM is not a supported setup recipe
while bindgen remains 0.70.1. A future CI image upgrade to LLVM 22 also needs
explicit compatible selection or the deliberate binding upgrade in option B.
Do not assume the LLVM embedded in rustc is the libclang loaded by bindgen.

## Verification

The isolated C reproducer `typedef struct Probe Probe; struct Probe { int
field; };` generates a one-byte Rust placeholder with local libclang 22.1.8,
and its layout assertion fails. With Apple clang/libclang 21.0.0 the field
is present and the generated Rust compiles. See
[retained comparison](../testing/phase-1-bindgen-llvm/audit.json).
This proves the generator incompatibility independently of FFmpeg.

The exact Windows LLVM 21.1.8 installation and full reference decode retry
were not measured by the original Mac reproducer. The owner subsequently
committed successful Windows hardware and software reports in `ad11a67`,
audited under `docs/testing/phase-1-decode-windows-x86_64/`. Those reports do
not capture the libclang version, so they prove a working Windows build and
decoder, not independently that LLVM 21.1.8 was selected. The LLVM version
in rustc's metadata is its separate backend.

Before the slice's first full matrix, Windows workflows now initialize the
x64 MSVC developer environment and inspect the actual selected libclang DLL
with `tools/windows-build-env.ps1`. Explicit LIBCLANG_PATH is authoritative;
otherwise known installed locations are inspected and the first compatible
pre-22 library selected. The script records its version, checks standard C
headers, and exports compiler variables for later steps. It downloads nothing
and changes no machine-wide settings. Hosted pre-22 LLVM can remain in use.

## Revisit when

The FFmpeg 7 binding line accepts bindgen with the upstream fix, or a planned
wrapper update makes LLVM 22 support possible without a maintained fork.
