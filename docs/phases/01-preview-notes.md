# Phase 1 preview/NDI integration preparation

The native decoder slice is merged and verified on Windows/Mac as recorded in
01-progress.md. CI preflight/evidence retention is a separate PR. Streaming
implementation is paused for [ADR 0037](../decisions/0037-resolve-wgpu-submission-on-the-realtime-thread.md):
the pinned wgpu submission path takes locks/allocates, conflicting with the
literal clock/composite-thread contract. No rule has been narrowed silently.

## Installed NDI SDK

The owner supplied `/Library/NDI SDK for Apple`. Its Version.txt identifies
6.3.2.0; `/usr/local/lib/libndi.dylib` reports 6.3.1.0. The required sender
exports are present. The [inventory](../testing/phase-1-ndi-macos-sdk-inventory.json)
records both versions and exact header hashes. This is not a sender/receiver,
ABI-conformance or NDI Studio Monitor test. No SDK was fetched, redistributed
or linked into the application. Runtime absence must remain nonfatal.

The installed `Processing.NDI.Send.h` establishes these implementation contracts:

- `clock_video` defaults true in the C++ convenience constructor. Set it
  explicitly false: the engine owns programme timing; NDI must not clock it.
- `NDIlib_send_send_video_async_v2` retains the submitted pixel memory until
  a synchronization event. Keep each mapped egress buffer alive until the
  next send returns, or flush/destroy before releasing the last buffer.
- A previous send may synchronize during the next call. All NDI calls belong
  to the sink thread, never the realtime clock, GUI or control thread.
- UYVA is a UYVY 4:2:2 plane followed by a full-resolution alpha plane.
  Encode/convert on GPU once at egress, then read back for the SDK. These
  SDK bytes are egress storage, not a CPU programme-frame representation.

Use the installed headers as the ABI source; do not infer struct layout from
the runtime version string. Preserve required header notices if declarations
are copied. SDK licensing is distinct from the MIT notices on header files.
The 6.3.2-header/6.3.1-runtime combination still needs actual sender execution;
symbol presence alone does not establish complete compatibility.

## Pending real-work measurements

Shared-device preview must sample GPU video textures without CPU readback.
The GUI sends input commands and renders authoritative engine state. Codec
workers keep borrowed decode staging local, upload once and publish working
Rgba16Float frames. No Phase 3 transforms/resampling or Phase 4 overlays are
introduced by this integration.

The five-minute reference run must count calls at the actual texture/view
creation boundary, covering ingest, working frames and egress. GUI-only
allocations need separate reporting under the proposed ADR clarification.
Retain live frame leases through actual GPU completion. Record both tick
lateness and rendered-output age/drops; inspect the known reference clusters
at 131–137 s and 508–516 s under real per-tick work. The M4 two-input/one-output
1080p50 diagnostic has no performance threshold. No sustained run has yet
been measured or claimed.
