# Starts the Candlescope live recorder hidden in the background.
#
# The release binary is copied to data\recorder-bin first, so `cargo build --release` can still
# replace target\release\candlescope.exe while the recorder runs. The process is created through
# WMI (Win32_Process.Create), so it does not belong to this PowerShell window: closing the window
# does not stop it. It stops on `scripts\stop-recorder.ps1`, sign-out, shutdown or reboot, and it
# pauses while the laptop sleeps (the gap is visible in the data and in data\live\recorder.log).
#
# Usage (from the imp folder):  powershell -ExecutionPolicy Bypass -File scripts\start-recorder.ps1

param([string]$DataDir = "", [string]$Source = "")

$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if ($DataDir -eq "") { $DataDir = Join-Path $root "data" }
$data = [System.IO.Path]::GetFullPath($DataDir)
$binDir = Join-Path $data "recorder-bin"
$exe = Join-Path $binDir "candlescope-recorder.exe"
$src = $Source
if ($src -eq "") { $src = Join-Path $root "target\release\candlescope.exe" }

$running = Get-Process -Name "candlescope-recorder" -ErrorAction SilentlyContinue
if ($null -ne $running) {
    Write-Output ("Recorder already running (pid " + (($running | ForEach-Object { $_.Id }) -join ",") + ").")
    exit 0
}
if (-not (Test-Path $src)) {
    Write-Output "Build first:  cargo build --release -p cs-cli"
    exit 1
}
New-Item -ItemType Directory -Force $binDir | Out-Null
Copy-Item -Force $src $exe

$cmd = '"' + $exe + '" record --data-dir "' + $data + '"'
$startup = New-CimInstance -ClassName Win32_ProcessStartup -ClientOnly -Property @{ ShowWindow = [uint16]0 }
$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
    CommandLine = $cmd
    CurrentDirectory = $root
    ProcessStartupInformation = $startup
}
if ($r.ReturnValue -ne 0) {
    Write-Output ("Could not start the recorder (Win32_Process.Create returned " + $r.ReturnValue + ").")
    exit 1
}
Write-Output ("Recorder started, pid " + $r.ProcessId + ". Data: " + (Join-Path $data "live"))
Write-Output ("Status:  " + $exe + ' record-status --data-dir "' + $data + '"')
