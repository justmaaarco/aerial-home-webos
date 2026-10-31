param([Parameter(Mandatory=$true)][string]$WallpaperRoot)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $WallpaperRoot).Path
$executable = Join-Path $root 'wallpaper64.exe'
if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) { throw 'Wallpaper Engine 64-bit is missing.' }
function Test-WallpaperRunning {
    return [bool](Get-Process -Name wallpaper64,wallpaper32 -ErrorAction SilentlyContinue | Where-Object {
        $_.Path -and (Split-Path $_.Path -Parent) -eq $root
    })
}
if (Test-WallpaperRunning) { return }
# The helper has its own logon task. Give it a short head start; local footage
# still works if the helper is unavailable, so it must not block host startup.
$deadline = (Get-Date).AddSeconds(15)
do {
    try {
        if ((Invoke-RestMethod -Uri 'http://127.0.0.1:5194/api/status' -TimeoutSec 2).service -eq 'aerial-desktop-catalogue') { break }
    } catch {}
    Start-Sleep -Milliseconds 500
} while ((Get-Date) -lt $deadline)
if (Test-WallpaperRunning) { return }
Start-Process -FilePath $executable -ArgumentList '-silent' -WorkingDirectory $root -WindowStyle Hidden | Out-Null
$deadline = (Get-Date).AddSeconds(15)
do {
    if (Test-WallpaperRunning) { return }
    Start-Sleep -Milliseconds 500
} while ((Get-Date) -lt $deadline)
throw 'Wallpaper Engine did not stay running after startup.'
