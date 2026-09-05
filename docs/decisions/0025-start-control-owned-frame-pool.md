# 0025 — Start Phase 1 with a control-owned GPU frame pool

- **Status:** Accepted
- **Date:** 2026-09-05
- **Phase:** 1
- **Decided by:** agent, within the owner's instruction to start Phase 1
- **Affects:** rezie-gpu, workspace, Phase 1 ownership tests

## Context

Phase 0 closed conditionally in 36ba587 with one Windows clock obligation due
at the Phase 1 gate. The first Phase 1 slice must establish GPU ownership before
pixels enter engine dispatch. SPEC §7.1 requires Rgba16Float, linear BT.709,
premultiplied alpha, a pool keyed by dimensions/format and no composite-thread
allocation. No compositor or decoder integration is claimed by this slice.

## Options

### A — Allocate a texture and reference-count wrapper for every frame

Creates steady-state allocations and makes lifetime/reuse invisible. Rejected.

### B — Preallocate texture/view slots and reference-count leases in place

Control-side reservations create resources; worker handles acquire/share/release
existing slots through bounded queues and atomics without allocating resources.

## Decision

Add rezie-gpu using already-pinned workspace wgpu, crossbeam-queue, rezie-core
and thiserror. Diagnostic binary dependencies reuse existing anyhow, tokio,
serde_json, tracing and tracing-subscriber. No new external/native dependency is introduced.

GpuContext owns the native instance/adapter/device/queue, with Metal on macOS,
D3D12 on Windows and Vulkan on Linux. Check required Rgba16Float usages before
requesting the device. These handles will later be supplied to eframe's existing
wgpu setup; this slice does not claim that GUI integration is implemented.

FramePool is control-thread-affine and has an explicit byte budget. Each
reservation accepts 1–1024 slots; invalid sizes are rejected before allocation. Reservations
are keyed by dimensions and the fixed working format. Growth creates a new
preallocated bucket on the control side; publish its new reader through engine
state when integration arrives. Existing readers/frames retain the old bucket.
Retired buckets are collected on the control side after all readers/leases end.
The pool owner must outlive worker handles; engine shutdown must join workers
before dropping it so GPU destruction does not migrate onto the composite path.

A worker reader only tries to acquire an existing slot; exhaustion returns None,
never waits or grows. FrameTime accompanies every leased frame. Sharing uses a
checked per-slot reference count; final release returns the index to the bounded
free queue. Callers retain a frame lease until all GPU uses have completed;
recording a command buffer is not completion. Future sink/compositor integration
must preserve that lifetime through submission and GPU completion.

## Consequences

No CPU pixel/frame representation is introduced. The pool only exposes working
GPU textures/views; ingest/egress colour conversion and compositing are later
work within Phase 1. Allocation counters count texture/view creation calls,
not driver-private memory behavior. Empty/exhausted pools and budget/format
errors are explicit. The real-device checker rejects CPU software adapters; no guessed production
result is permitted.

## Verification

Run a real-device functional check on M4: reserve, exhaust, share, release,
reuse repeatedly without new texture/view calls, and grow while old leases
remain live. Check actual adapter identity and working texture format. Compile-
fail doctest prevents moving the allocating owner to another thread. Hosted CI
runs portable validation only; the reference gate must run the real-device
check when hardware exists. This does not satisfy the five-minute reference
allocation criterion, compositor goldens, decoding, NDI output or the Phase 1
gate. All remain explicitly open in 01-progress.md.

## Revisit when

Integrating GPU completion into dispatch/sinks, adding ingest/egress staging
resources, or a device/pool requirement cannot be met on the reference GPU.

## Recorded first-slice evidence

The real Metal check on Apple M4 passed 20,000 two-worker acquire/share/release
cycles, with two initial textures/views, no additional creation calls during
reuse, and six cumulative textures/views after explicit control-side growth.
Old/shared leases and byte-budget rejection were checked. Exact tested source
hashes match commit 573dce5. The report is in docs/testing and is expressly
functional-only. Hosted run 33991900133 passed all three platforms; no reference
GPU, compositor, golden or five-minute performance criterion has been evaluated.
