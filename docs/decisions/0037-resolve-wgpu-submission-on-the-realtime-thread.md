# 0037 — Resolve wgpu submission on the realtime thread

- **Status:** Proposed — implementation waits for human review
- **Date:** 2026-09-06
- **Phase:** 1
- **Decided by:** pending human review
- **Affects:** SPEC §5.2/§5.3/§7.1, AGENTS rules 1/2, rezie-engine/rezie-gpu/rezie-app

## Context

The first streaming preview connects the programme clock to actual GPU work.
SPEC §5.2 puts gathering, compositor execution and sink dispatch on one
clock/composite thread. AGENTS rule 1 forbids locks and allocation on that
thread. The pinned wgpu implementation cannot provide that guarantee, even
when all application textures, views, buffers and bind groups are preallocated.

Inspected installed sources, matching Cargo.lock:

- `wgpu 25.0.2`, `src/api/queue.rs:233`: `Queue::submit` consumes command
  buffers; they are not reusable per-tick submissions.
- `wgpu-core 25.0.2`, `src/device/queue.rs:1071–1079`: submission takes the
  device snatch lock, fence write lock and command-index write lock, then
  creates `active_executions = Vec::new()`.
- The same file, line 1153, locks device trackers; line 1204 pushes a
  nonempty submission into that fresh vector; line 1221 locks pending writes.
  These are ordinary submission paths, not just optional tracing code.

SHA-256 of the inspected files, in the same order (wgpu API, wgpu-core queue):
`b51dcc2e41d2fe42c70e3376deb39780d514e905e188f05d23415742f3800076`,
`8185fababa0f641c27b9757c7d9515ec1fb1a3ed16a7b4e836a110ba0633e218`.

This is source evidence of possible blocking and CPU allocation, not a measured
stall or proof that a reference timing criterion cannot pass. All prior GPU
tests run off the realtime thread, so they did not exercise this conflict.
Preallocating FramePool textures does not preallocate wgpu's submission internals.

There is a related boundary ambiguity: SPEC §3 requires stock eframe rendering
on the shared device, but egui-wgpu creates its own GUI buffers, font textures
and samplers. A literal prohibition on *all* GPU allocations outside FramePool
also excludes that required renderer. GUI presentation resources need an
explicit scope; they must never be mistaken for pooled programme resources.
For example, `egui-wgpu 0.32.3/src/renderer.rs:626` creates its own texture;
lines 752/808 create samplers, and lines 1029/1039 create GUI buffers.
That file's SHA-256 is
`c1f4da7e9342457e35ce36147d8531e44d462b4e2010c06e2e845e5609d00790`.

## Options

### A — Permit wgpu internals on the clock/composite thread

Keep the specified thread layout but narrow the no-lock/no-allocation rule to
application code. Measure submission time, contention with GUI/upload and tail
latency on the reference machine. This changes a hard isolation rule into a
measured dependency on wgpu/driver behavior. It cannot promise no blocking.

### B — Keep the clock isolated and add a bounded GPU submission worker

The realtime clock owns timestamps, selects preallocated frame leases and
enqueues fixed-capacity render jobs without calling wgpu. A separate worker
records/submits GPU work and publishes completed frames to independently
bounded sinks. Queue exhaustion drops stale jobs with explicit accounting;
neither queues nor in-flight GPU work may grow without bound. Original
FrameTime index/PTS travels unchanged through completion and output.

This changes the thread table and dispatch location. It adds queueing latency
and does not make an overloaded renderer meet deadlines. Clock lateness alone
becomes insufficient evidence: publish-to-render, GPU completion, output age,
missed output frames and all drop causes must also be recorded. A smooth clock
must never conceal a stalled picture. Frame leases remain held through GPU
completion; reclamation/resource destruction stays off the clock.

### C — Pre-record all command buffers and replay them

Rejected: wgpu submission consumes command buffers and still takes submission
locks. Render bundles do not eliminate the outer command encoding/submission.
Replacing wgpu or using native backend bypasses would contradict the locked
stack and be a much larger decision, not a Phase 1 workaround.

## Decision

Recommend B, pending approval. Do not implement the thread split or narrow
the normative rule silently. If accepted, amend §5.2/§5.3 to separate realtime
selection from GPU execution/completion dispatch, preserving programme PTS and
per-sink isolation. Specify bounded render/in-flight capacity and observable
overload; no timing or zero-texture-allocation acceptance bound is relaxed.

Also explicitly scope FramePool ownership to application video resources:
working/ingest/egress textures, views, staging buffers and video pipelines.
Stock eframe GUI/presentation allocations remain on the GUI thread and are
reported separately. They cannot be passed off as zero programme allocations.
This clarification needs approval alongside the threading decision.

## Consequences

Preview, NDI conversion and sustained-load integration wait on this decision.
CI preflight and historical-evidence protection are independent and proceed.
No new dependency, shader, golden reference or later-phase feature is proposed.

The five-minute gate must count actual creation calls at the video resource
boundary, including ingest/egress allocations, rather than infer from pool
occupancy. Report GUI resource activity separately. Frame completion and sink
delivery latency must be visible alongside the inherited tick distribution,
including the known reference tail clusters at 131–137 s and 508–516 s.

## Verification

Before implementation, source inspection establishes the incompatibility above.
After an approved implementation: portable bounded-queue/lease-lifetime tests,
deliberately delayed GPU-worker tests proving clock isolation and visible
drops, shared-device preview on M4 and RX 6800 XT, and the reference five-minute
allocation/real-work timing run. None of these streaming tests has run yet.

## Revisit when

The pinned wgpu submission API changes, measured worker queueing is excessive,
or a reference phase criterion cannot be met without a further architecture
change. Any such change requires a new ruling, not relaxed bounds.
