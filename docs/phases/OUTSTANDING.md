# Phase 0 obligation — PAID

**PAID — 2026-09-06, explicit owner ruling, ADR 0028.**

The Windows 11 / RX 6800 XT clock benchmark, including the platform slack
sweep, is accepted under the revised recorded-load / 10x-margin criterion.

Evidence: [ten-minute report](../benchmarks/phase-0-idle-windows-x86_64.json),
[host metadata](../benchmarks/phase-0-idle-windows-x86_64.host.json),
[v2 curve](../benchmarks/phase-0-slack-sweep-windows-x86_64-v2/summary.json),
[load record](../benchmarks/windows-acceptance-idle-evidence-v2/idle-samples.jsonl),
and [ruling](../decisions/0028-accept-reference-clock-with-recorded-load.md).

Zero index/PTS errors, 30,001 ticks, confirmed MMCSS Pro Audio and successful
1 ms timer request. Final/max/p99.9 margins are 66,666.667x / 155.642x /
196.078x. Windows is approved at 1,000 µs; no further run is owed.

No open obligations remain. Preserve this payment entry in git history and
remove this file in the closure commit; the summary and ADR retain its links.
