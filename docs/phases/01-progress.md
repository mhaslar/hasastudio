# Phase 1 — Media foundation and first picture (in progress)

Phase 0 closed **conditionally**, not fully verified, in 36ba587. Its sole
Windows reference-clock obligation remains OPEN in OUTSTANDING.md and is due
at this phase's gate. If that measurement fails, Phase 0 reopens and Phase 1
work stops until rezie-rt is fixed. No conditional Phase 1 closure is allowed.

## First implementation slice: GPU ownership

ADR 0025 records rezie-gpu's device and frame-pool boundary. Native context
selection uses Metal/macOS, D3D12/Windows and Vulkan/Linux, checking working
format capabilities. A control-thread-affine FramePool reserves texture/view
slots by dimensions/format with an explicit byte budget. Rgba16Float is the
only frame format; frame leases carry FrameTime and no CPU pixel payload.
Worker readers acquire/share/release using bounded queues and atomics. An
exhausted reader returns None without allocating or waiting. Growth preserves
old leases; retired buckets are collected by their control-side owner.

The GPU pool is not yet integrated into the engine/GUI. The empty Foundation
GUI still uses its existing eframe setup. Shared-device preview, ingest colour
conversion, file/image/colour sources, hardware/software decoder selection,
NDI output and add/remove-input commands remain Phase 1 work. No compositor
shader or golden references have been introduced by this slice.

## Evidence so far

Local formatting, strict workspace Clippy, 20 nextest executable tests and
both compile-fail doctests passed. A real Metal run on Apple M4 passed the
pool ownership check: two worker threads completed 20,000 acquire/share/release
cycles without additional texture/view creation calls. Exhaustion did not grow
the pool; sharing held the slot until final release; growth retained an old
lease; the explicit byte budget rejected an oversized reservation.

The raw functional report is in
[phase-1-pool-macos-aarch64.json](../testing/phase-1-pool-macos-aarch64.json),
with exact source-file hashes. This is not a runtime benchmark, five-minute
reference allocation test, pixel readback, decoder test or golden comparison.
The reference hardware has not run this GPU code. The source hashes match implementation commit `573dce5`. Hosted formatting,
Clippy, nextest/doctests and packaged GUI launches passed on Windows, macOS
and Linux in [run 33991900133](https://github.com/mhaslar/hasastudio/actions/runs/33991900133).
These are portable correctness results; no hosted GPU or performance evidence
is inferred from them.

## Remaining gate work

All Phase 1 acceptance clauses in SPEC §13 remain open: H.264 preview on the
reference machine, receivable NDI output, verified native decoders on equipped
platform workers and explicit software fallback, correct PNG alpha over colour,
and five-minute steady-state reference allocation evidence. Build the reduced
M4 two-input/one-output 1080p50 diagnostic when its media paths exist. Normative
goldens stay on the reference; M4/Metal functional success is necessary but
insufficient. Pay OUTSTANDING.md before this phase can close.

Phase 0's gen-assets/golden commands still reject Phase 1 explicitly; their
Phase 1 media/golden implementation is not presented as complete. The manual
Phase 0 clock sweep/bench remains available independently of that work.

## Sequence once the reference obligation is paid

The reference host is available for manual tests, but v2's unexplained CPU load
means the clock obligation is not yet paid. No new Phase 1 implementation is
included in this audit slice. Availability does not waive idle, GPU or codec
verification, and it does not imply a self-hosted runner is configured.

The golden-reference policy is unchanged: normative goldens originate only on
Windows 11 / RX 6800 XT, with human review before updating references. M4 is a
required Metal correctness check, not a source of normative references; hosted
lavapipe remains optional/non-blocking and never updates them.

After adequate clock evidence, validate the existing GPU context/FramePool on
RX 6800 XT immediately, including working-format/limit support and the D3D12
path (and Vulkan where shipped). Then implement the deterministic colour +
alpha-PNG ingest/composite path and shared-device preview, checking numerical
colour/alpha expectations before proposing the first reference goldens for
human review. Do not bless whatever pixels the current renderer produces.
Bring file decode and explicit hardware/software selection into that validated
path, testing native Windows decode and fallback early. Add the single NDI
output with the user-installed SDK and verify it in NDI Studio Monitor. Finish
reference five-minute steady-state allocation evidence and the reduced M4
2-input/1-output 1080p50 diagnostic; retain actual decoder assertions on
appropriately equipped macOS/Linux workers. AMF *encoding* remains Phase 5.

The change in sequencing is earlier production-backend feedback for each small
slice instead of accumulating Metal-only code and waiting until the gate to
try Windows. Every shader/compositor slice still needs reference execution and
M4 checks before the gate; one ready PR/full run per slice remains the rule.
