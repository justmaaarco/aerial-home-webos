$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
$base = Join-Path $env:LOCALAPPDATA 'AerialDesktop'
$helper = Join-Path $base 'helper'
$nodeExe = (Get-Command node -ErrorAction Stop).Source
$ffmpegExe = (Get-Command ffmpeg -ErrorAction Stop).Source
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
'@
$startupScript = $startupScript.Replace('__FFMPEG__', $ffmpegExe.Replace("'", "''")).Replace('__NODE__', $nodeExe.Replace("'", "''"))
$launch = Join-Path $helper 'start-helper.ps1'
Set-Content -LiteralPath $launch -Value $startupScript -Encoding UTF8
$startup = [Environment]::GetFolderPath('Startup')
$shortcut = (New-Object -ComObject WScript.Shell).CreateShortcut((Join-Path $startup 'Aerial Desktop Catalogue.lnk'))
$shortcut.TargetPath = Join-Path $env:SystemRoot 'System32/WindowsPowerShell/v1.0/powershell.exe'
$shortcut.Arguments = '-NoProfile -WindowStyle Hidden -File "' + $launch + '"'
$shortcut.WorkingDirectory = $helper
$shortcut.Description = 'Local Apple aerial catalogue playback service'
$shortcut.Save()
& $launch
Write-Output 'Installed the loopback catalogue helper and per-user startup shortcut.'
