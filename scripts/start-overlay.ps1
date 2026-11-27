# Starts the Candlescope context overlay (context only: no signals, no orders).
# Builds the release binary first if it is missing or older than any source file, then starts it
# from the project folder so it finds data\, config\ and reports\. Extra arguments go to the
# overlay, e.g.:  scripts\start-overlay.ps1 --symbol ETHUSDT --tf 5m --expanded
# Settings (position, size, opacity, mode, account, risk, symbol, screen reading) are restored
# from data\overlay-settings.json; arguments win over the file.

param([Parameter(ValueFromRemainingArguments = $true)] [string[]] $OverlayArgs)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $root "target\release\candlescope-overlay.exe"

$sources = @(Get-ChildItem -Path (Join-Path $root "crates") -Recurse -File -Include *.rs, Cargo.toml) +
    @(Get-Item (Join-Path $root "Cargo.toml"), (Join-Path $root "Cargo.lock")) +
    @(Get-ChildItem -Path (Join-Path $root "config") -File -ErrorAction SilentlyContinue)
$newest = ($sources | Sort-Object LastWriteTime -Descending | Select-Object -First 1)

if (-not (Test-Path $exe) -or $newest.LastWriteTime -gt (Get-Item $exe).LastWriteTime) {
    Write-Host "Building the overlay (release); newest source: $($newest.Name)"
    Push-Location $root
    try {
        cargo build --release -p cs-ui
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit $LASTEXITCODE)" }
    } finally {
        Pop-Location
    }
} else {
    Write-Host "Release build is up to date."
}

$p = if ($OverlayArgs -and $OverlayArgs.Count -gt 0) {
    Start-Process -FilePath $exe -ArgumentList $OverlayArgs -WorkingDirectory $root -PassThru
} else {
    Start-Process -FilePath $exe -WorkingDirectory $root -PassThru
}
Write-Host "Overlay started (process $($p.Id)). Hotkeys: Ctrl+Alt+H hide, Ctrl+Alt+M mute, Ctrl+Alt+T click-through, Ctrl+Alt+S screen reading."
