use anyhow::{Context, Result};
use std::process::Command;

pub(super) fn capture() -> Result<serde_json::Value> {
    anyhow::ensure!(cfg!(windows), "normative benchmark/soak requires Windows 11 / RX 6800 XT; M4 uses clock-sweep for diagnostic calibration");
    let script = r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$os = Get-CimInstance Win32_OperatingSystem
$gpu = @(Get-CimInstance Win32_VideoController)
if ($os.Caption -notmatch 'Windows 11') { throw 'Reference requires Windows 11' }
if (-not ($gpu | Where-Object Name -match 'RX 6800 XT')) { throw 'Reference requires RX 6800 XT' }
@{
    os = ($os | Select-Object Caption, Version, BuildNumber)
    gpu = @($gpu | Select-Object Name, DriverVersion)
    cpu = @(Get-CimInstance Win32_Processor | Select-Object Name, NumberOfCores, NumberOfLogicalProcessors)
    memory_bytes = (Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory
} | ConvertTo-Json -Depth 5 -Compress
"#;
    let result = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .context("query Windows reference OS/GPU/driver metadata")?;
    anyhow::ensure!(
        result.status.success(),
        "reference identity check failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).context("parse reference hardware metadata")
}
