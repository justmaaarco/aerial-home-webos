$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
$base = Join-Path ([Environment]::GetFolderPath('MyDocuments')) 'AerialDesktop'
$helper = Join-Path $base 'helper'
$legacyHelper = Join-Path $env:LOCALAPPDATA 'AerialDesktop/helper'
$nodeExe = (Get-Command node -ErrorAction Stop).Source
$ffmpegExe = (Get-Command ffmpeg -ErrorAction Stop).Source
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$taskName = 'Aerial Desktop Catalogue-' + $identity.User.Value
$existingTask = Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
if ($existingTask) {
    $knownActions = @((Join-Path $helper 'start-helper.ps1'),(Join-Path $helper 'desktop-stream-server.cjs'),(Join-Path $legacyHelper 'start-helper.ps1'),(Join-Path $legacyHelper 'desktop-stream-server.cjs'))
    if (-not ($existingTask.Actions | Where-Object { $arguments=$_.Arguments; @($knownActions | Where-Object { $arguments.Contains($_) }).Count -gt 0 })) { throw 'The catalogue task name is already in use by a different action.' }
    Stop-ScheduledTask -TaskName $taskName
}
New-Item -ItemType Directory -Force -Path $helper | Out-Null
$serviceFile = Join-Path $helper 'desktop-stream-server.cjs'
$pidFile = Join-Path $helper 'service.pid'
if (Test-Path -LiteralPath $pidFile) {
    $ownedProcess = Get-CimInstance Win32_Process -Filter ('ProcessId = ' + [int](Get-Content -LiteralPath $pidFile))
    if ($ownedProcess -and $ownedProcess.CommandLine -and $ownedProcess.CommandLine.Contains($serviceFile)) { Stop-Process -Id $ownedProcess.ProcessId }
}
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'desktop-stream-server.cjs') -Destination $helper
Copy-Item -LiteralPath (Join-Path $repo 'desktop/catalogue-core.cjs') -Destination $helper
Copy-Item -LiteralPath (Join-Path $repo 'app/catalogue.js') -Destination $helper
$startupScript = @'
param([switch]$WaitForExit)
$ErrorActionPreference = 'Stop'
try { if ((Invoke-RestMethod -Uri 'http://127.0.0.1:5194/api/status' -TimeoutSec 2).service -eq 'aerial-desktop-catalogue') { return } } catch {}
$ffmpegExe = '__FFMPEG__'
if (-not (Test-Path -LiteralPath $ffmpegExe)) { $ffmpegExe = (Get-Command ffmpeg -ErrorAction Stop).Source }
$env:AERIAL_DESKTOP_FFMPEG = $ffmpegExe
$nodeExe = '__NODE__'
if (-not (Test-Path -LiteralPath $nodeExe)) { $nodeExe = (Get-Command node -ErrorAction Stop).Source }
$helper = $PSScriptRoot
$service = Join-Path $helper 'desktop-stream-server.cjs'
$process = Start-Process -FilePath $nodeExe -ArgumentList ('"{0}"' -f $service) -WorkingDirectory $helper -WindowStyle Hidden -RedirectStandardOutput (Join-Path $helper 'service.log') -RedirectStandardError (Join-Path $helper 'service-error.log') -PassThru
$process.Id | Set-Content -LiteralPath (Join-Path $helper 'service.pid')
if ($WaitForExit) { $process.WaitForExit(); $process.Refresh(); exit $process.ExitCode }
'@
$startupScript = $startupScript.Replace('__FFMPEG__', $ffmpegExe.Replace("'", "''")).Replace('__NODE__', $nodeExe.Replace("'", "''"))
$launch = Join-Path $helper 'start-helper.ps1'
Set-Content -LiteralPath $launch -Value $startupScript -Encoding UTF8
$nativePs = Join-Path $env:SystemRoot 'System32/WindowsPowerShell/v1.0/powershell.exe'
$action = New-ScheduledTaskAction -Execute $nativePs -Argument ('-NoProfile -WindowStyle Hidden -File "' + $launch + '" -WaitForExit')
$trigger = New-ScheduledTaskTrigger -AtLogOn -User $identity.User.Value
$principal = New-ScheduledTaskPrincipal -UserId $identity.User.Value -LogonType Interactive -RunLevel Limited
$settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit ([TimeSpan]::Zero) -MultipleInstances IgnoreNew -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1)
Register-ScheduledTask -TaskName $taskName -Action $action -Trigger $trigger -Principal $principal -Settings $settings -Description 'Local aerial catalogue playback for the signed-in user' -Force | Out-Null
$taskName | Set-Content -LiteralPath (Join-Path $helper 'task-name.txt')
Start-ScheduledTask -TaskName $taskName
# Retire only this installer's previous shortcut after the scheduled task is registered.
$oldShortcut = Join-Path ([Environment]::GetFolderPath('Startup')) 'Aerial Desktop Catalogue.lnk'
if (Test-Path -LiteralPath $oldShortcut) {
    $link = (New-Object -ComObject WScript.Shell).CreateShortcut($oldShortcut)
    if ($link.Arguments.Contains($launch) -or $link.Arguments.Contains((Join-Path $legacyHelper 'start-helper.ps1'))) { Remove-Item -LiteralPath $oldShortcut }
}
Write-Output 'Installed and started the loopback catalogue helper through a per-user logon task.'
