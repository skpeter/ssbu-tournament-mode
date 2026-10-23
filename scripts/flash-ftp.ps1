$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
if (-not $env:SWITCH_FTP) {
    Write-Error "Set SWITCH_FTP (example: ftp://192.168.0.10:5000) before flashing."
}
$HostFtp = $env:SWITCH_FTP
$Nro = Join-Path $Root "target/aarch64-skyline-switch/release/libssbu_tournament_mode.nro"
$RemoteNro = "$HostFtp/sdmc:/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/libssbu_tournament_mode.nro"
$RemoteLog = "$HostFtp/sdmc:/tournament_mode.log"

if (-not (Test-Path $Nro)) {
    Write-Error "missing $Nro - run: cargo skyline build --release"
}

Write-Host "Uploading NRO to $RemoteNro"
curl.exe -sS --connect-timeout 15 -T $Nro $RemoteNro
if ($LASTEXITCODE -ne 0) { Write-Error "NRO upload failed ($LASTEXITCODE)" }

Write-Host "Wiping log at $RemoteLog"
$Empty = Join-Path $env:TEMP "tournament_mode.empty.log"
Set-Content -Path $Empty -Value "" -NoNewline
curl.exe -sS --connect-timeout 15 -T $Empty $RemoteLog | Out-Null

Write-Host "Done. Reboot Smash (or the Switch) to load the new NRO."
