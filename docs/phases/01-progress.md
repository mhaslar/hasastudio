# Phase 1 — Media foundation and first picture (in progress)

Phase 0 is closed under ADR 0028. Its Windows reference-clock obligation is
PAID and the ledger removed; no Foundation debt remains. The owner approved
resuming the sequence below. No Phase 1 gate has passed yet.

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
The M4 source hashes match implementation commit `573dce5`. Hosted formatting,
Clippy, nextest/doctests and packaged GUI launches passed on Windows, macOS
and Linux in [run 33991900133](https://github.com/mhaslar/hasastudio/actions/runs/33991900133).
These are portable correctness results; no hosted GPU or performance evidence
is inferred from them.

The owner has now supplied the [RX 6800 XT / D3D12 report](../testing/phase-1-pool-windows-x86_64.json):
20,000 cycles, zero additional texture/view creation during reuse, exhaustion,
shared lease retention, growth and budget checks all passed. Its
[provenance note](../testing/phase-1-pool-windows-provenance.md) records that the
report was supplied manually and did not embed its checkout revision. This
clears the first functional step in the approved reference-first sequence;
it does not satisfy the five-minute allocation gate.

## Second slice: deterministic colour/alpha diagnostic

[ADR 0029](../decisions/0029-check-linear-alpha-on-native-gpus.md) adds a
control-side FramePool diagnostic: sRGB PNG ingest into premultiplied linear
Rgba16Float, alpha-over a colour source, and sRGB PNG egress. It allocates and
waits on the control side; this is not yet the streaming compositor or preview.
Raw working channels and exported pixels are checked against a numerical oracle.

The [M4/Metal report](../testing/phase-1-colour-macos-aarch64/report.json)
passes all five cases, 16,705 pixels each. Maximum linear absolute error is
0.000959691 (limit 0.002), and maximum exported channel error is one code
value (limit two). Numerical expectations include the linear-light midpoint
(half-white over black exports near 188, not 128), hidden RGB under zero alpha,
translucent backgrounds, transfer breakpoints and odd dimensions. Shader,
probe and checker source hashes identify the measured code. The output PNGs
are diagnostic evidence, not approved references.

Local formatting, strict workspace Clippy, 22 nextest tests and both doctests
passed for this slice. The existing ignored manual timing test was not run;
no latency or performance measurement is claimed. Windows execution of this
colour diagnostic remains pending. Follow
[colour-alpha-check.md](../user/colour-alpha-check.md); only then proceed to
production golden candidates for human review. No Phase 1 gate has closed.

## Remaining gate work

All Phase 1 acceptance clauses in SPEC §13 remain open: H.264 preview on the
reference machine, receivable NDI output, verified native decoders on equipped
platform workers and explicit software fallback, correct PNG alpha over colour,
and five-minute steady-state reference allocation evidence. Build the reduced
M4 two-input/one-output 1080p50 diagnostic when its media paths exist. Normative
goldens stay on the reference; M4/Metal functional success is necessary but
insufficient. The former Phase 0 clock obligation is already paid.

Phase 0's gen-assets/golden commands still reject Phase 1 explicitly; their
Phase 1 media/golden implementation is not presented as complete. The manual
Phase 0 clock sweep/bench remains available independently of that work.

## Approved sequence with the reference host available

The clock obligation is paid. Begin with the existing FramePool check on
RX 6800 XT, then deterministic colour/alpha rendering there, golden references
for human review, decode with fallback and NDI. Availability does not imply
that a self-hosted runner or remote execution connection is configured.

The golden-reference policy is unchanged: normative goldens originate only on
Windows 11 / RX 6800 XT, with human review before updating references. M4 is a
required Metal correctness check, not a source of normative references; hosted
lavapipe remains optional/non-blocking and never updates them.

First validate the existing GPU context/FramePool on
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

Re-check the accepted clock report's 131–137 s and 508–516 s tail clusters
under real per-tick media/compositor work. Record tail counts, temporal grouping,
p50/p99/p99.9/max and headroom under the same unchanged timing bounds. These
are known reference observations, not existing defects or a new Phase 0 debt.
