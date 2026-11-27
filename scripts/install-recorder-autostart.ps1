# OPTIONAL. Registers a Windows scheduled task that starts the recorder when you sign in, so it
# keeps recording after a reboot. Run it yourself if you want that; remove it again with:
#   Unregister-ScheduledTask -TaskName "Candlescope recorder" -Confirm:$false
#
# Usage (from the imp folder):  powershell -ExecutionPolicy Bypass -File scripts\install-recorder-autostart.ps1

$ErrorActionPreference = "Stop"
$start = (Resolve-Path (Join-Path $PSScriptRoot "start-recorder.ps1")).Path
$action = New-ScheduledTaskAction -Execute "powershell.exe" `
    -Argument ('-NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File "' + $start + '"')
$trigger = New-ScheduledTaskTrigger -AtLogOn -User $env:USERNAME
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries `
    -ExecutionTimeLimit (New-TimeSpan -Seconds 0)
Register-ScheduledTask -TaskName "Candlescope recorder" -Action $action -Trigger $trigger `
    -Settings $settings -Description "Starts the Candlescope market-data recorder at sign-in" | Out-Null
Write-Output "Scheduled task 'Candlescope recorder' registered (runs at sign-in)."
