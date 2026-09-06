# 0027 — Retain observed ticks and distinguish accounting resolution

- **Status:** Accepted
- **Date:** 2026-09-06
- **Phase:** 0
- **Decided by:** both
- **Affects:** benchmark reports, rezie-rt documentation, calibration guide

## Context

The v2 Windows ten-minute run meets numerical bounds but its idle evidence
reports about 11% total CPU with many unreadable process CPU counters. ADR
0021 records the INADEQUATE ruling and tail analysis. A passing boolean must
not substitute for admissible measurements. The owner also requests individual
PTS serialization and an explicit limit on cross-platform CPU comparisons.

## Options

### A — Reconstruct expected PTS when serializing and treat 100 ns units as accuracy

Produces a self-confirming report and disguises coarse OS accounting. Rejected.

### B — Retain received FrameTime values and disclose the counter's resolution

The non-realtime benchmark consumer retains each actual index/PTS in receive
order. Existing lateness samples and correctness counters remain separate.

## Decision

Adopt B. Add `observed_ticks` to ClockReport, populated by the draining sink
consumer, not reconstructed from expected indices. Preallocate in the harness;
no allocation, CPU pixel type or additional operation enters the clock loop.
Serialize after shutdown. Legacy reports deserialize with an empty vector,
explicitly meaning that replayable PTS evidence is absent. Never backfill or
rewrite historical measurement files. Add bench --output for fresh report paths
and reject existing report/host files so the next run preserves v2. Reference
automation supplies a run-ID/attempt output name; its triggers are unchanged. Document an independent integer-arithmetic
verifier using a different language and no engine types.

Retain GetThreadTimes accounting but label its limits in the API docs and
sweep metadata. Both Windows sweeps' nonzero spin and whole-thread totals are
multiples of **15,625,000 ns**. The 0.5 ms cases show zero measured spin CPU
despite nonzero spin segments. This is observed quantization of the selected
OS counter on this host, not a universal promised Windows resolution or an
arithmetic conversion bug. Summing short deltas loses attribution; zero does
not mean no CPU work. Even long-run totals remain accounting estimates.

Microsoft specifies [GetThreadTimes units of 100 ns](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getthreadtimes),
not matching measurement resolution. [QueryThreadCycleTime](https://learn.microsoft.com/en-us/windows/win32/api/realtimeapiset/nf-realtimeapiset-querythreadcycletime)
returns cycles and explicitly warns against converting them to elapsed time.
A separately labeled cycle counter could be added later; substituting cycles
or spin wall time into nanosecond CPU columns would be wrong. The existing
Windows CPU columns cannot be compared to CLOCK_THREAD_CPUTIME_ID on M4 as if
they had equal accuracy. This documents the limitation of this API rather than
claiming Windows offers no finer instrumentation of any kind.

The owner accepts any recorded active power plan with supporting idle evidence.
High Performance is preferred, not mandatory. An unavailable setting is not
an acceptance failure. Improve rerun telemetry with raw total-CPU counters and
process performance counters, and request an elevated recording shell to make
attribution more complete. Do not infer idle operation from operator notes.

## Consequences

The current Windows default stays unset and the single Phase 0 obligation
stays open pending adequate idle evidence. Phase 1 implementation does not
resume in this slice. The next finite report allows independent index/PTS
checking; old reports still rely on the original harness's live PTS checks.
No clock wait or scheduling behavior, bound, dependency or golden reference
changes. Raw performance data may still have inaccessible/protected processes;
record and explain residual gaps instead of labeling missing CPU time zero.

## Verification

The integration harness serializes actual received ticks; independently check
integer PTS/index order in JSON. Include a non-nominal synthetic FrameTime
serialization test so erroneous observed values cannot be silently normalized.
Run fmt, Clippy, nextest/doctests and the slice's single PR matrix. A brief
functional report is not a latency measurement. Preserve v2 originals byte for
byte and record tail indices and telemetry findings in ADR 0021.

Local validation passed: fmt, strict workspace Clippy, 20 nextest tests and
two compile-fail doctests. A one-second Correctness-mode JSON retained 51
actual ticks, independently checked with Python integer arithmetic. Existing
report and host-sidecar overwrite attempts both failed before reference capture.
All v2 raw evidence is byte-identical to `5282c5f`; restored original Windows
sweep files match `20a1759`. Reference workflow triggers are unchanged. The
single PR run supplies platform integration verification; no local latency
measurement was attempted.

## Revisit when

Adequate idle reference evidence pays Phase 0, or cycle-based/ETW CPU profiling
is implemented with separately named units and an explicit measurement model.

## Subsequent owner ruling — ADR 0028

The Windows v2 result is accepted under SPEC §13's recorded-load / tenfold-margin
rule. Windows is pinned to 1,000 µs; the Phase 0 obligation is PAID and its
ledger removed. Phase 0 is closed and Phase 1 resumes. Earlier pending or
INADEQUATE statements in this ADR are historical. Observed-PTS serialization,
accounting limitations, preferred power plan and evidence protection remain.
