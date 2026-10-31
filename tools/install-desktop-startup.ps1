param([string]$WallpaperRoot=(Join-Path ${env:ProgramFiles(x86)} 'Steam/steamapps/common/wallpaper_engine'))
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $WallpaperRoot).Path
if ($root.Contains('"') -or -not (Test-Path -LiteralPath (Join-Path $root 'wallpaper64.exe') -PathType Leaf)) { throw 'Choose a Wallpaper Engine installation containing wallpaper64.exe.' }
$base = Join-Path ([Environment]::GetFolderPath('MyDocuments')) 'AerialDesktop'
$helper = Join-Path $base 'helper'
New-Item -ItemType Directory -Path $helper -Force | Out-Null
$launch = Join-Path $helper 'start-wallpaper.ps1'
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$taskName = 'Aerial Desktop Wallpaper-' + $identity.User.Value
$existing = Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
if ($existing -and -not ($existing.Actions | Where-Object { $_.Arguments -and $_.Arguments.Contains($launch) })) { throw 'The wallpaper task name is already used by another action.' }
$runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$oldRun = (Get-ItemProperty -LiteralPath $runKey -Name WallpaperEngine -ErrorAction SilentlyContinue).WallpaperEngine
$backup = Join-Path $base 'before-wallpaper-startup.json'
if (-not (Test-Path -LiteralPath $backup)) {
    [pscustomobject]@{WallpaperEngineRun=$oldRun;TaskExisted=[bool]$existing} | ConvertTo-Json | Set-Content -LiteralPath $backup -Encoding UTF8
}
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'start-desktop-wallpaper.ps1') -Destination $launch
$nativePs = Join-Path $env:SystemRoot 'System32/WindowsPowerShell/v1.0/powershell.exe'
$action = New-ScheduledTaskAction -Execute $nativePs -Argument ('-NoProfile -WindowStyle Hidden -File "' + $launch + '" -WallpaperRoot "' + $root + '"')
$trigger = New-ScheduledTaskTrigger -AtLogOn -User $identity.User.Value
$trigger.Delay = 'PT10S'
$principal = New-ScheduledTaskPrincipal -UserId $identity.User.Value -LogonType Interactive -RunLevel Limited
$settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Minutes 2) -MultipleInstances IgnoreNew -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1)
Register-ScheduledTask -TaskName $taskName -Action $action -Trigger $trigger -Principal $principal -Settings $settings -Description 'Start Wallpaper Engine quietly after user sign-in and aerial helper startup' -Force | Out-Null
$taskName | Set-Content -LiteralPath (Join-Path $helper 'wallpaper-task-name.txt')
# Retire only the normal startup command belonging to this installation, after
# the replacement task is registered. Leave all other startup entries alone.
$knownRuns = @(
    ('"' + (Join-Path $root 'wallpaper32.exe') + '" -silent')
    ('"' + (Join-Path $root 'wallpaper64.exe') + '" -silent')
)
if ($oldRun -and $knownRuns -contains $oldRun) { Remove-ItemProperty -LiteralPath $runKey -Name WallpaperEngine }
Start-ScheduledTask -TaskName $taskName
Write-Output 'Installed the delayed per-user Wallpaper Engine startup task and started it now.'
