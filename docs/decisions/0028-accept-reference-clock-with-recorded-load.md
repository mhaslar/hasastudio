# 0028 — Accept the reference clock with recorded load and sufficient margin

- **Status:** Accepted
- **Date:** 2026-09-06
- **Phase:** 0
- **Decided by:** human review
- **Affects:** SPEC §13, Windows realtime default, Phase 0 closure

## Context

The owner overruled the v2 INADEQUATE ruling. The reference run meets every
numeric bound: maximum lateness 128,500 ns against 20,000,000 ns; p99.9
25,500 ns against 5,000,000 ns; final drift 300 ns against 20,000,000 ns.
The earlier idle rule was intended to prevent attributing a loaded failure to
rezie-rt, not to reject a passing result with large margins. Recorded load and
the Task Manager/operator-note difference are not reasons to reject this run.

## Options

### A — Require another idle run despite the numerical margins

Retains the preceding admission rule. Rejected by the owner.

### B — Record load and admit passes with at least tenfold timing margin

Apply the owner's revised evidence-admission rule, preserve the bounds and
accept the existing reference result. Require idle evidence for a failure or
any passing result with less margin before ruling in either direction.

## Decision

Adopt B. Amend SPEC §13 with the owner's exact text:

> Background load is recorded, not required to be absent. If a run
> passes every bound with at least 10x margin, recorded background load
> does not invalidate it — load biases lateness upward, so a pass under
> load is conservative. If a run fails, or passes with less than 10x
> margin on any bound, an idle machine is required before the result can
> be ruled on in either direction.

Tenfold margin applies to the positive timing bounds (final drift, maximum,
p99.9). Zero skipped indices and exact PTS remain exact correctness constraints;
there is no meaningful ratio against a zero-error bound. Keep all reported
load and raw samples. This records the owner's admission policy and reasoning;
no experiment separately isolating the effect of each background process was
performed or is claimed.

Rule **PASS** on the existing Windows v2 sweep and ten-minute report. Pin
Windows finishing slack to **1,000 µs**, matching the measured override and
both Windows curves. macOS stays at 500 µs; Linux remains uncalibrated. Mark
the single Phase 0 obligation PAID with its evidence link, preserve that status
in git history, then remove the outstanding file. Rewrite the Phase 0 summary
as closed, not conditionally closed. The active phase remains Phase 1.

The accepted v2 report predates observed-PTS serialization. The owner accepts
its recorded live zero-error checks; do not manufacture or backfill raw PTS.
PR #3's actual-observation serialization, CPU quantization documentation,
preferred-not-mandatory power plan, overwrite protection and restored first
sweep remain in force.

## Consequences

Record the 131–137 s and 508–516 s tail clusters as known characteristics of
the reference measurement, not defects. Re-check them with real per-tick
work during Phase 1. Keep that check in the Phase 1 plan, not as a new unpaid
Phase 0 obligation. Record collector/Task Manager/Edge/VS Code as plausible
background contributors without claiming complete attribution.

Windows wins decisively at ten-minute p50 and p99, but loses at ten-minute
p99.9 (25.5 vs 18.25 µs). The short sweep was misleading about the longer-run
tail; the owner's generalization from it was incorrect. Preserve the lesson:
longer runs reveal tail behavior that short trials hide, even when central
percentiles improve. Do not generalize the short sweep into a universal
platform ranking.

Phase 1 resumes in its own slice: existing FramePool on RX 6800 XT first,
deterministic colour/alpha rendering there, golden references for human review,
then decode with fallback and NDI. Normative goldens remain reference-only;
M4 correctness is necessary and hosted performance is never normative.

## Verification

Recompute all distributions/counts and native scheduling status from
[the accepted report](../benchmarks/phase-0-idle-windows-x86_64.json), alongside
[host metadata](../benchmarks/phase-0-idle-windows-x86_64.host.json),
[v2 sweep](../benchmarks/phase-0-slack-sweep-windows-x86_64-v2/summary.json)
and [recorded load](../benchmarks/windows-acceptance-idle-evidence-v2/idle-samples.jsonl).
The timing margins are **155.642× maximum**, **196.078× p99.9**, and
**66,666.667× final drift**. Index/PTS errors remain zero and the native thread
reports MMCSS Pro Audio and a successful 1 ms timer request. Verify normal
Windows startup selects the pin; run portable checks and one PR full matrix.
No additional reference clock run is necessary to pay this obligation.

## Revisit when

Reference hardware/configuration changes materially, or real Phase 1 per-tick
work changes the tail or consumes its headroom. Never relax numeric bounds.
