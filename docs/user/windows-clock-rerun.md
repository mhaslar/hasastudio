# Windows clock evidence required after the 2026-09-06 audit

The committed six-value sweep is internally consistent. It is not a ten-minute
run and contains no power-plan or idle evidence. Keep it unchanged. Phase 0's
obligation remains OPEN. No Windows slack is pinned. **1,000 µs is the reviewed
candidate override for the next measurement**, subject to the repeated curve.

If the ten-minute JSON already exists on the reference machine, first preserve
and send it together with its `.host.json` and any contemporaneous power/idle
record. Do not reconstruct missing metadata as if it had been captured then.
A complete original record could avoid repeating that measurement. Otherwise,
repeat the sweep and ten-minute acceptance as below, approximately 17 minutes
of measurement plus setup. These PowerShell commands are provided for manual
execution; they have not been executed on the Windows machine by the agent.

## Prepare before allowing the machine to settle

Use native x64 Developer PowerShell on Windows 11 / RX 6800 XT. Check out the
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

Record the existing power plan. Select **High Performance** for this run using
Windows Power Options, or `powercfg /setactive SCHEME_MIN`. Do not proceed if
that selection fails or the plan is unavailable; report it instead. Preserve
the original plan GUID so you can restore it afterward. Balanced results must
be labeled Balanced and are not accepted here as equivalent idle calibration.
The processor settings dump below includes core-parking configuration; a plan
name alone does not document custom settings.

```powershell
powercfg /getactivescheme
# After recording the original GUID:
powercfg /setactive SCHEME_MIN
if ($LASTEXITCODE -ne 0) { throw 'Could not select High Performance' }
powercfg /getactivescheme
```

Stop builds, media, games and other active work, allow updates/indexing to
settle, and keep the machine otherwise idle throughout measurement. Do not
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
        if ($LASTEXITCODE -ne 0) { throw 'Processor power capture failed' }
        $job = Start-Job -ArgumentList $evidence,$stopFile -ScriptBlock {
            param($directory,$stop)
            $ErrorActionPreference = 'Stop'
            while (-not (Test-Path $stop)) {
                $record = @{
                    utc = (Get-Date).ToUniversalTime().ToString('o')
                    total = @(Get-CimInstance Win32_PerfFormattedData_PerfOS_Processor |
                        Where-Object Name -eq '_Total' |
                        Select-Object Name,PercentProcessorTime,PercentDPCTime,PercentInterruptTime)
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
Keep the full record, not just a favorable screenshot or an average. If the
monitor failed, a substantial competing workload ran, or the plan changed,
retain that run but do not submit it as idle acceptance.

## Repeat the same sweep, then the unprofiled acceptance run

Use a new output directory so the original sweep cannot be overwritten:

```powershell
Invoke-RecordedClockRun 'docs/benchmarks/windows-sweep-idle-evidence-v2' {
    cargo xtask clock-sweep --output docs/benchmarks/phase-0-slack-sweep-windows-x86_64-v2
    if ($LASTEXITCODE -ne 0) { throw 'Sweep failed; preserve its output' }
}
```

Inspect `summary.csv` and all native scheduling results. If the curve still
puts 1,000 µs at the low-tail region before degradation at 500 µs, proceed with
the explicitly reviewed candidate below. If it changes materially, send the
sweep first; do not guess a new pin. Leave other applications idle between runs.
The instrumentation overhead and CPU-accounting caveat remain disclosed.

The bench output path is fixed. If an earlier Windows acceptance JSON or host
sidecar exists, preserve both under a separate clearly named directory before
running; do not overwrite or discard them.

```powershell
if ((Test-Path docs/benchmarks/phase-0-idle-windows-x86_64.json) -or
    (Test-Path docs/benchmarks/phase-0-idle-windows-x86_64.host.json)) {
    throw 'Preserve existing acceptance files before running again'
}
Invoke-RecordedClockRun 'docs/benchmarks/windows-acceptance-idle-evidence-v2' {
    cargo xtask bench --slack-us 1000
    if ($LASTEXITCODE -ne 0) { throw 'Acceptance failed; preserve its output' }
}
```

Expected acceptance files:

- `docs/benchmarks/phase-0-idle-windows-x86_64.json`: 600 seconds, 30,001
  samples, zero index/PTS errors, `wait_profile: null`, applied 1,000 µs slack,
  MMCSS Pro Audio confirmed, successful 1 ms timer request, full distribution.
- `docs/benchmarks/phase-0-idle-windows-x86_64.host.json`: source/toolchain,
  Windows build, CPU, GPU and driver metadata.
- Both `*-idle-evidence-v2/` directories: power settings before/after,
  timestamps, complete utilization record, monitor outcome. Add a short
  `operator-notes.txt` in each describing foreground applications, remote
  access, interruptions and whether the machine remained idle. Record actual
  observations; do not fill in a boilerplate assertion after a loaded run.

The bounds remain **zero skipped indices; exact PTS; final drift <20 ms;
maximum lateness <20 ms; p99.9 <5 ms** at 50 Hz. Do not interpret sweep
`passed: true` or `latency_passed: null` as acceptance. If the genuine
acceptance run fails, stop: Phase 0 reopens and Phase 1 work must stop.

Commit all raw and host/idle evidence on an evidence branch and submit one PR;
do not push directly to main. Restore the original power plan using its saved
GUID when finished. No self-hosted runner is required for these manual runs.
