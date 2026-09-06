# Existing build prerequisites only: no downloads or machine-wide changes.
# Dot-source locally, or invoke in Actions to export values to later steps.
$ErrorActionPreference = 'Stop'
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (-not (Test-Path $vswhere)) { throw 'Install MSVC Build Tools and Windows SDK; vswhere.exe was not found.' }
$rezieVs = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $rezieVs) { throw 'No Visual Studio installation with x64 C++ tools was found.' }
Import-Module "$rezieVs\Common7\Tools\Microsoft.VisualStudio.DevShell.dll"
Enter-VsDevShell -VsInstallPath $rezieVs -SkipAutomaticLocation -DevCmdArguments '-arch=x64 -host_arch=x64'

$rezieCandidates = if ($env:LIBCLANG_PATH) {
    # An explicit selection is authoritative; do not silently substitute it.
    @($env:LIBCLANG_PATH)
} else {
    @('C:\Tools\LLVM-21.1.8\bin', "$env:ProgramFiles\LLVM\bin", "$rezieVs\VC\Tools\Llvm\x64\bin")
}
$rezieSelected = $null
foreach ($rezieCandidate in $rezieCandidates) {
    $rezieDll = Join-Path $rezieCandidate 'libclang.dll'
    $rezieClang = Join-Path $rezieCandidate 'clang.exe'
    if (-not ((Test-Path $rezieDll) -and (Test-Path $rezieClang))) { continue }
    # Inspect the actual library loaded by bindgen, not rustc's LLVM backend.
    $rezieVersion = & python -c 'import ctypes,sys; lib=ctypes.CDLL(sys.argv[1]); S=type("S",(ctypes.Structure,),{"_fields_":[("data",ctypes.c_void_p),("flags",ctypes.c_uint)]}); lib.clang_getClangVersion.restype=S; lib.clang_getCString.argtypes=[S]; lib.clang_getCString.restype=ctypes.c_char_p; lib.clang_disposeString.argtypes=[S]; s=lib.clang_getClangVersion(); print(lib.clang_getCString(s).decode()); lib.clang_disposeString(s)' $rezieDll
    if ($LASTEXITCODE -ne 0) { throw "Cannot inspect $rezieDll" }
    Write-Host "libclang: $rezieDll : $rezieVersion"
    if ($rezieVersion -match 'clang version (\d+)\.') {
        $rezieMajor = [int]$Matches[1]
        if ($rezieMajor -ge 9 -and $rezieMajor -lt 22) {
            $rezieSelected = $rezieCandidate
            break
        }
    }
}
if (-not $rezieSelected) { throw 'bindgen 0.70.1 requires pre-22 libclang. Install LLVM 21.1.8 and set LIBCLANG_PATH to its bin directory; see ADR 0035.' }
$env:LIBCLANG_PATH = $rezieSelected
$env:CLANG_PATH = Join-Path $rezieSelected 'clang.exe'
$env:PATH = "$rezieSelected;$env:PATH"
& $env:CLANG_PATH --version
if ($LASTEXITCODE -ne 0) { throw 'Selected clang executable failed.' }
'#include <stdint.h>' | & $env:CLANG_PATH --target=x86_64-pc-windows-msvc -x c -fsyntax-only -
if ($LASTEXITCODE -ne 0) { throw 'Clang cannot parse standard C headers after MSVC setup; check MSVC and Windows SDK installation.' }
if ($env:GITHUB_ENV) {
    # Export only compiler setup, never credentials or the complete environment.
    foreach ($rezieKey in @('INCLUDE', 'LIB', 'LIBPATH', 'VCINSTALLDIR', 'VCToolsInstallDir', 'WindowsSdkDir', 'WindowsSDKVersion', 'UniversalCRTSdkDir', 'UCRTVersion', 'LIBCLANG_PATH', 'CLANG_PATH')) {
        $rezieValue = [Environment]::GetEnvironmentVariable($rezieKey)
        if ($rezieValue) { "$rezieKey=$rezieValue" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append }
    }
    $env:PATH -split ';' | Where-Object { $_ } | Out-File -FilePath $env:GITHUB_PATH -Encoding utf8 -Append
}
