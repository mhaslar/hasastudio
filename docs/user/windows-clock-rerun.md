# Windows clock evidence required after the 2026-09-06 audit

**V2 is accepted; no rerun is required to close Phase 0.** ADR 0028 records
the owner's revised load-admission rule and pays the obligation. Windows is
pinned to 1,000 µs. Preserve v2 and the first sweep unchanged.

Record background load; absence is not required for a pass with at least 10x
margin on every positive timing bound. For a failure or a pass with less
margin, collect idle evidence before ruling in either direction. Exact index
and PTS correctness are never relaxed. The existing 155.642x maximum and
196.078x p99.9 margins admit v2 despite the recorded activity.

The following procedure remains available for **future** calibration/regression
measurements. A short preflight can investigate unexplained CPU load when
needed; it is not a prerequisite for the already accepted v2 run. Raw counter
collection and independent observed-PTS verification remain useful. These
PowerShell instructions are for manual execution, not a claim that the agent
executed them on Windows.

## Prepare before allowing the machine to settle

Use an elevated (Administrator) native x64 Developer PowerShell on Windows 11 / RX 6800 XT. Check out the
merged audit/CI slice through a fast-forward pull and finish builds first:

```powershell
git switch main
git pull --ff-only
rustc -vV
cargo build --release -p rezie-engine --bin rezie-headless
if ($LASTEXITCODE -ne 0) { throw 'Headless build failed' }
cargo build -p xtask
if ($LASTEXITCODE -ne 0) { throw 'xtask build failed' }
```

Record the active plan. **High Performance is preferred, not mandatory**.
If available, select it in Power Options; if selecting it is unsupported, keep
and record the actual scheme. A recorded plan plus adequate idle telemetry is
the requirement. Balanced is not an automatic failure, and no absent setting
invalidates a run. Do not infer idleness or CPU frequency from the plan name.
If you change it, record the original GUID and restore it afterward.

```powershell
powercfg /getactivescheme
# Optional, only if available; selection failure is not a benchmark failure:
# powercfg /setactive SCHEME_MIN
```

Stop builds, media, games and other active work, allow updates/indexing to
settle where practical, and record actual activity throughout measurement. Do not
change affinity, scheduling code or timer settings to obtain a passing result.
Record whether remote-control software is running. The lightweight recorder
below is the only additional diagnostic workload; its own CPU appears in the
process telemetry. CPU telemetry helps audit idleness but cannot prove absence
of every interrupt, DPC or scheduler disturbance.

## Capture power configuration and utilization around each command

Paste this function once. It records a 30-second preflight, samples every five
seconds throughout the command, captures per-process CPU counters and retains
all evidence even when the benchmark fails. The phase tools build/check metadata
before their internal settling delay; no build may overlap actual tick sampling.

```powershell
function Invoke-RecordedClockRun {
    param([string]$EvidenceDirectory, [scriptblock]$Measurement)
    $ErrorActionPreference = 'Stop'
    if (Test-Path $EvidenceDirectory) { throw 'Use a fresh evidence directory' }
    $evidence = (New-Item -ItemType Directory $EvidenceDirectory).FullName
    $stopFile = Join-Path $evidence 'stop-monitor'
    $job = $null
    try {
        (Get-Date).ToUniversalTime().ToString('o') | Set-Content "$evidence/start-utc.txt"
        powercfg /getactivescheme | Out-File "$evidence/power-plan-before.txt"
        if ($LASTEXITCODE -ne 0) { throw 'Power-plan capture failed' }
        powercfg /qh SCHEME_CURRENT SUB_PROCESSOR | Out-File "$evidence/processor-power-before.txt"
        # Some hardware does not expose these settings; record the outcome, do not fail.
        $LASTEXITCODE | Set-Content "$evidence/processor-power-query-exit.txt"
        $job = Start-Job -ArgumentList $evidence,$stopFile -ScriptBlock {
            param($directory,$stop)
            $ErrorActionPreference = 'Stop'
            while (-not (Test-Path $stop)) {
                $record = @{
                    utc = (Get-Date).ToUniversalTime().ToString('o')
                    total = @(Get-CimInstance Win32_PerfFormattedData_PerfOS_Processor |
                        Where-Object Name -eq '_Total' |
                        Select-Object Name,PercentProcessorTime,PercentDPCTime,PercentInterruptTime)
                    raw_total = @(Get-CimInstance Win32_PerfRawData_PerfOS_Processor |
                        Where-Object Name -eq '_Total' |
                        Select-Object Name,PercentProcessorTime,Timestamp_Sys100NS)
                    raw_processes = @(Get-CimInstance Win32_PerfRawData_PerfProc_Process |
                        Select-Object Name,IDProcess,PercentProcessorTime,Timestamp_Sys100NS)
                    processes = @(Get-Process | Select-Object Id,ProcessName,CPU)
                }
                $record | ConvertTo-Json -Depth 6 -Compress |
                    Add-Content -Encoding UTF8 "$directory/idle-samples.jsonl"
                Start-Sleep -Seconds 5
            }
        }
        Start-Sleep -Seconds 30
        if ($job.State -ne 'Running' -or -not (Test-Path "$evidence/idle-samples.jsonl")) {
            throw 'Idle recorder failed; do not start measurement'
        }
        (Get-Date).ToUniversalTime().ToString('o') | Set-Content "$evidence/command-start-utc.txt"
        & $Measurement
    } finally {
        (Get-Date).ToUniversalTime().ToString('o') | Set-Content "$evidence/command-end-utc.txt"
        powercfg /getactivescheme | Out-File "$evidence/power-plan-after.txt"
        powercfg /qh SCHEME_CURRENT SUB_PROCESSOR | Out-File "$evidence/processor-power-after.txt"
        New-Item -ItemType File -Path $stopFile | Out-Null
        if ($null -ne $job) {
            Wait-Job $job | Out-Null
            $job.State | Out-File "$evidence/monitor-state.txt"
            Receive-Job $job -ErrorAction Continue 2>&1 | Out-File "$evidence/monitor-output.txt"
            Remove-Job $job
        }
        Remove-Item $stopFile
    }
}
```

The per-process `CPU` field is cumulative CPU seconds, not an instantaneous
percentage. Differences between successive samples show active processes.
Keep the full record, not just a favorable screenshot or an average. If the monitor failed or the plan changed, preserve that information. Do not
claim idle conditions that were not observed; recorded competing load by itself
does not invalidate a high-margin pass.

## Diagnose idleness before the long runs

With the recorder function above loaded, run only a short preflight:

```powershell
Invoke-RecordedClockRun 'docs/benchmarks/windows-idle-preflight-v3' {
    Start-Sleep -Seconds 60
}
```

Compare the formatted total CPU values with successive raw total counters:
`100 * (1 - delta(PercentProcessorTime) / delta(Timestamp_Sys100NS))`.
Process counters use `100 * delta(PercentProcessorTime) / delta(Timestamp_Sys100NS)`
and are percentages of one core; sum individual process instances, excluding
Idle and _Total, then divide by the 20 logical processors when comparing to
machine-wide CPU. Match PID and name between samples; process turnover and
protected processes must be reported, not treated as zero. If raw data disagrees
with the formatted value, retain both for review rather than picking the lower.

For a failing or low-margin run needing idle confirmation, if unexplained
load persists, send this short preflight first. Do not
silently disable services or declare the host idle from a threshold. Identify
and pause actual competing work. If attribution remains incomplete, a separate
WPR/ETW diagnostic trace can resolve it before the acceptance measurement;
such tracing adds overhead and is not itself idle acceptance.

## Repeat the same sweep, then the unprofiled acceptance run

Use a new output directory so the original sweep cannot be overwritten:

```powershell
Invoke-RecordedClockRun 'docs/benchmarks/windows-sweep-idle-evidence-v3' {
    cargo xtask clock-sweep --output docs/benchmarks/phase-0-slack-sweep-windows-x86_64-v3
    if ($LASTEXITCODE -ne 0) { throw 'Sweep failed; preserve its output' }
}
```

Inspect `summary.csv` and all native scheduling results. If the curve still
puts 1,000 µs at the low-tail region before degradation at 500 µs, proceed with
the explicitly reviewed candidate below. If it changes materially, send the
sweep first; do not guess a new pin. Leave other applications idle between runs.
The instrumentation overhead and CPU-accounting caveat remain disclosed.

Use the new explicit output path to keep v2 intact. Existing report/host
files are rejected, including failed evidence: select another fresh path for
any further attempt.

```powershell
Invoke-RecordedClockRun 'docs/benchmarks/windows-acceptance-idle-evidence-v3' {
    cargo xtask bench --slack-us 1000 --output docs/benchmarks/phase-0-idle-windows-x86_64-v3.json
    if ($LASTEXITCODE -ne 0) { throw 'Acceptance failed; preserve its output' }
}
```

Expected acceptance files:

- `docs/benchmarks/phase-0-idle-windows-x86_64-v3.json`: 600 seconds, 30,001
  samples and 30,001 actual `observed_ticks` index/PTS records, zero index/PTS errors, `wait_profile: null`, applied 1,000 µs slack,
  MMCSS Pro Audio confirmed, successful 1 ms timer request, full distribution.
- `docs/benchmarks/phase-0-idle-windows-x86_64-v3.host.json`: source/toolchain,
  Windows build, CPU, GPU and driver metadata.
- Both `*-idle-evidence-v3/` directories: power settings before/after,
  timestamps, complete utilization record, monitor outcome. Add a short
  `operator-notes.txt` in each describing foreground applications, remote
  access, interruptions and whether the machine remained idle. Record actual
  observations; do not fill in a boilerplate assertion after a loaded run.

The numerical bounds remain **zero skipped indices; exact PTS; final drift <20 ms;
maximum lateness <20 ms; p99.9 <5 ms** at 50 Hz. Do not interpret sweep
`passed: true` or `latency_passed: null` as acceptance. If a future run fails or passes with less than tenfold margin, obtain idle
evidence before ruling on it; preserve all attempts and do not relax a bound.

Commit all raw and host/idle evidence on an evidence branch and submit one PR;
do not push directly to main. Restore the original power plan using its saved
GUID when finished. No self-hosted runner is required for these manual runs.

## Independently verify individual PTS

The new `observed_ticks` array stores the actual received FrameTime values in
receive order. These are not expected values reconstructed during export.
Legacy files lack this field; do not retrofit them. For a 600-second 50 Hz
report, this standalone Python check uses no Rust/harness PTS implementation:

```python
import json
from pathlib import Path
r = json.loads(Path("docs/benchmarks/phase-0-idle-windows-x86_64-v3.json").read_text())
frames = r["observed_ticks"]
assert r["duration_seconds"] == 600
assert len(frames) == len(r["lateness"]["samples_ns"]) == 30001
for i, frame in enumerate(frames):
    assert frame["index"] == i
    assert 0 <= frame["pts"]["nanos"] < 1_000_000_000
    actual_ns = frame["pts"]["secs"] * 1_000_000_000 + frame["pts"]["nanos"]
    assert actual_ns == i * 20_000_000
```

This verifies index/PTS correctness, not native scheduling, idleness or latency;
those remain separate audit requirements. Windows spin CPU columns are
quantized OS accounting, not comparable in accuracy to the M4 thread clock.
See ADR 0027; zero recorded spin CPU must never be described as free spinning.
