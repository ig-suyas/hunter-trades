# Asks the Candlescope live recorder to stop cleanly (it closes and gzips its current files).
#
# Usage (from the imp folder):  powershell -ExecutionPolicy Bypass -File scripts\stop-recorder.ps1

param([string]$DataDir = "")

$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if ($DataDir -eq "") { $DataDir = Join-Path $root "data" }
$data = [System.IO.Path]::GetFullPath($DataDir)
$live = Join-Path $data "live"

$p = Get-Process -Name "candlescope-recorder" -ErrorAction SilentlyContinue
if ($null -eq $p) {
    Write-Output "Recorder is not running."
    exit 0
}
New-Item -ItemType Directory -Force $live | Out-Null
Set-Content -Path (Join-Path $live "STOP") -Value "stop" -Encoding ascii
for ($i = 0; $i -lt 40; $i++) {
    Start-Sleep -Milliseconds 500
    if ($null -eq (Get-Process -Name "candlescope-recorder" -ErrorAction SilentlyContinue)) {
        Write-Output "Recorder stopped."
        exit 0
    }
}
Write-Output "Recorder did not stop within 20 s; stopping the process (the next start gzips any unfinished file)."
Get-Process -Name "candlescope-recorder" -ErrorAction SilentlyContinue | Stop-Process -Force
