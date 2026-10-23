$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Nro = Join-Path $Root "target/aarch64-skyline-switch/release/libssbu_tournament_mode.nro"
$Zip = Join-Path $Root "ssbu-tournament-mode-sd.zip"
if (-not (Test-Path $Nro)) {
    Write-Error "missing $Nro; run: cargo skyline build --release"
}
$Stage = Join-Path $Root "dist/sd"
$PluginDest = Join-Path $Stage "atmosphere/contents/01006A800016E000/romfs/skyline/plugins"
$CfgDest = Join-Path $Stage "ultimate"
if (Test-Path $Stage) { Remove-Item -Recurse -Force $Stage }
New-Item -ItemType Directory -Force -Path $PluginDest | Out-Null
New-Item -ItemType Directory -Force -Path $CfgDest | Out-Null
Copy-Item $Nro (Join-Path $PluginDest "libssbu_tournament_mode.nro")
Copy-Item (Join-Path $Root "config/tournament_mode.toml.example") (Join-Path $CfgDest "tournament_mode.toml")
if (Test-Path $Zip) { Remove-Item $Zip }
Compress-Archive -Path (Join-Path $Stage "*") -DestinationPath $Zip
Write-Host "wrote $Zip"
