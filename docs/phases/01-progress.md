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
no latency or performance measurement is claimed.

The owner committed the Windows colour run in `92829e8`. The
[RX 6800 XT / D3D12 report](../testing/phase-1-colour-windows-x86_64/report.json)
passes all 83,525 pixels with the same maxima as M4. An
[independent audit](../testing/phase-1-colour-windows-audit.json) verifies every
exported RGBA channel against a separate double-precision calculation; all five
Windows PNGs have identical decoded bytes to M4. Source hashes match `8659673`
with CRLF on Windows and LF on macOS. The raw linear maxima remain
producer-reported because raw linear readbacks were not serialized.

[ADR 0030](../decisions/0030-review-first-colour-alpha-references.md) proposes the
five Windows outputs as the initial reference content. The
[candidate review](../testing/phase-1-golden-candidates.md) contains images,
band expectations and exact file hashes. The owner accepted the scene design but required 16-bit output and raw linear
serialization before approving hashes. The old 8-bit proposal is superseded;
no references have been installed. The asset generator/perceptual golden harness
is the next step after that approval, before decode/fallback and NDI.
No Phase 1 gate has closed.

## Remaining gate work

All Phase 1 acceptance clauses in SPEC §13 remain open: H.264 preview on the
reference machine, receivable NDI output, verified native decoders on equipped
platform workers and explicit software fallback, correct PNG alpha over colour,
and five-minute steady-state reference allocation evidence. Build the reduced
M4 two-input/one-output 1080p50 diagnostic when its media paths exist. Normative
goldens stay on the reference; M4/Metal functional success is necessary but
insufficient. The former Phase 0 clock obligation is already paid.

`gen-assets` and `golden` now implement the Phase 1 colour/alpha inventory.
Default golden comparison requires the reference machine; M4 uses explicitly
non-normative development mode. The manual Phase 0 clock sweep/bench remains
available independently of this work.

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

## Precision revision before reference approval

ADR 0030 now requires direct GPU RGBA16 PNG export and exact `.rgba16f.le`
readbacks. No scene values changed. The new
[M4 report](../testing/phase-1-colour16-macos-aarch64/report.json) and
[independent audit](../testing/phase-1-colour16-macos-aarch64/audit.json) pass
83,525 pixels: maximum linear error 0.000959691 (now reconstructed from raw
samples), maximum egress error one 16-bit code value. The separately recorded
full-pipeline PNG error reaches 193/65535 with intermediate half-float rounding.
All old reports remain unchanged. Windows has now rerun this revision; the
new hash-bound proposal is awaiting final approval. Conditional scene approval
is not reference approval. Backend byte equality is coincidental; sampling in Phase 3 can differ.
[Phase 4 notes](04-notes.md) reserve the ten-overlay accumulation scene for
when overlays exist. Only the single-blend scene is implemented now.

Validation for the precision revision: formatting and strict workspace Clippy
passed; 22 nextest tests and both doctests passed. Nextest marked the existing
pure clock-distribution unit test leaky once; its isolated rerun passed without
that flag. No cause is established and no engine code changed. Four portable
Python audit tests pass, including forged metrics, altered raw samples with an
updated hash, and PNG depth/CRC rejection. ci-fast runs these recorded-data
checks without GPU execution or a latency assertion.


## Windows 16-bit evidence and revised hash proposal

The owner's `7f01657` report passes the independent offline audit on all
83,525 pixels. Exact raw binary16 files now reconstruct the 0.000959691 linear
maximum; maximum egress error is one 16-bit code value. Source hashes match
`79113fd` with CRLF line endings. Five PNGs plus five raw files are proposed in
[the manifest](../testing/phase-1-colour16-candidate-manifest.json) and
[review sheet](../testing/phase-1-golden-candidates.md). No files are installed
as references pending final approval. The five raw readbacks match M4, but 184
exported channel values differ by one 16-bit step; this bounded variation
reinforces ADR 0030's warning against requiring backend byte equality.
No code changed for this evidence audit and no additional full matrix ran.


## Approved installation and golden harness

The owner approved all ten reference files bound to `c9397f0`. They are installed
byte-for-byte in `tests/golden/phase-1/colour-alpha/` with an approved manifest;
every copied length and hash matches. The earlier pending-approval entries
above are historical. No further permission is needed for this installation.

ADR 0031 implements the regenerated input, explicit input-file rendering and
CIEDE2000 comparison over black and white, with separate alpha and raw-linear
checks. Strict ΔE limits remain mean <1 / max <3. Failures preserve actual
PNG16/raw data and colour/alpha heatmaps. Tampering, alpha-only regressions,
strict threshold edges and image bit depth are covered by portable tests.
The reference workflow uploads these artifacts without changing its triggers.

The [full M4 harness run](../testing/phase-1-golden-macos-aarch64/report.json)
passes all five cases. Worst per-case/background mean ΔE00 is 0.00000881734,
worst maximum 0.00284570, with zero alpha and raw-linear difference against
approved Windows samples. Its `normative_reference_result` is false. The
independent renderer audit also passes. Native-reference restriction and
existing-output rejection were verified, with earlier evidence preserved.

Formatting, strict workspace Clippy, 28 nextest tests, both doctests and four
Python audit tests pass. No new full matrix has run yet. A fresh Windows run
of [the harness](../user/golden-tests.md) is now the remaining validation for
this slice before its one ready PR/full matrix. Reference acceptance is not
inferred from approving files or M4 success. Phase 1 remains open; decode and
fallback, shared-device preview, NDI, allocation and load measurements follow
in the approved sequence. Phase 4 notes carry the observed precision and
repeated-blend accumulation follow-up without implementing overlays now.


## Reference golden inventory passed

The owner committed the fresh Windows golden run in `e252a0e`. Its
[audit](../testing/phase-1-golden-windows-x86_64/audit.json) verifies native
Windows 11 build 26200 / RX 6800 XT / D3D12 execution and all five scenes:
mean/max ΔE00 zero on both backgrounds, alpha error zero, raw-linear difference
zero. All 83,525 pixels and ten freshly rendered files match the approved
reference bytes. Source and manifest hashes resolve with CRLF line endings;
the embedded and standalone renderer reports agree. An independent raw/PNG
audit reconstructs the numerical pipeline checks. The corresponding M4 run
remains a development pass with worst maximum ΔE00 0.00284570.

The colour/alpha golden inventory now passes on both required native targets.
Phase 1 is still open: this is the control-side diagnostic, not integrated
preview/decode/NDI or five-minute allocation and real-load timing evidence.
The slice is ready for its one full CI matrix and PR. Binary Git attributes
protect the approved image/readback bytes from automatic text conversion.


## Decode backend and mandatory native-library policy

PR #5 (colour/alpha goldens) merged as `09197e3` after one successful full
Windows/macOS/Linux matrix. The next slice is `phase-1/media-decode`.

The owner approved ADR 0032 and its amendments. SPEC now records the actual
Windows LGPLv3-or-later licence, the four enabled components requiring
version3, dav1d AV1 fallback, and mandatory build/startup ABI/licence checks.
The Windows artifact pin is unchanged. AV1 version skew carries no pixel
tolerance; it matters for security updates and reproducibility. M4 golden
thresholds remain unchanged; Phase 3 sampling is the review trigger.

ADRs 0033/0034 add the dedicated file decoder, isolated native bootstrap,
explicit software override, actual hardware context/pixel-format reporting,
and the optional macOS accessor for Apple's actual hardware-session property.
Generic VideoToolbox device hwctx is legitimately null. The modern FFmpeg
session is private, and HEVC permits internal Apple software fallback, so
backend names and hardware-shaped frames alone are not sufficient evidence.
No application code depends on FFmpeg private layouts.

Seven owned synthetic fixtures cover MP4/MOV/MKV/TS, all four codecs, and
8/10-bit decode. They retain independently generated component hashes and PTS.
H.264 includes actual B-frames; all 24 pictures per file must survive EOF drain.
Local M4 automatic decode matches all 168 pictures: H.264 and 8/10-bit HEVC
use observed hardware sessions, while this FFmpeg version has no VideoToolbox
configuration for VP9/AV1 and explicitly falls back to native VP9/libdav1d.
The actual dav1d log reports 1.5.4. Replacing the Mac library with an unmodified
compatible FFmpeg was tested: Auto remains functional via software, while
strict hardware reports the missing accessor and fails.

Both guard stages reject synthetic GPL, nonfree and wrong-major libraries.
The real development Mac GPL FFmpeg 9 was also rejected with its actual
63.1.101 version/configuration. The isolated Mac build is LGPLv2.1-or-later,
libavcodec 61.19.101; ordinary engine startup always checks the process-linked
library before starting threads. Native bootstrap no longer depends on the
engine, and build preparation does not mutate system FFmpeg.

The first portable test attempt exposed an outdated Phase 1 Unix dependency
count and one nextest leak label on an unrelated sink test; after correcting
the manifest expectation the complete 32-test run passed without a leak
label. The ignored reference latency test remains intentionally excluded.
Both compile-fail doctests and four offline colour-audit tests passed.
A development-package smoke passed with loader overrides removed; goldens
passed again at the existing shared thresholds. Final source-bound native
reports and Linux ci-fast are recorded before the slice is proposed for its
single full matrix. Windows reference decode evidence is still required;
[manual commands](../user/native-decode.md) do not require idle preparation.

No GPU upload/colour conversion from decoded planes, shared-device preview,
input commands or NDI output is presented as complete in this slice. No
performance or Phase 1 closure claim is made. These remain the next approved
integration steps after native decode validation.
