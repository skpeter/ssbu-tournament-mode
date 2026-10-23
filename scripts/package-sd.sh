#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NRO="$ROOT/target/aarch64-skyline-switch/release/libssbu_tournament_mode.nro"
ZIP="$ROOT/ssbu-tournament-mode-sd.zip"
STAGE="$ROOT/dist/sd"
PLUGIN="$STAGE/atmosphere/contents/01006A800016E000/romfs/skyline/plugins"
CFG="$STAGE/ultimate"
[[ -f "$NRO" ]] || { echo "missing $NRO — cargo skyline build --release"; exit 1; }
rm -rf "$STAGE"
mkdir -p "$PLUGIN" "$CFG"
cp "$NRO" "$PLUGIN/libssbu_tournament_mode.nro"
cp "$ROOT/config/tournament_mode.toml.example" "$CFG/tournament_mode.toml"
rm -f "$ZIP"
( cd "$STAGE" && zip -r "$ZIP" . )
echo "wrote $ZIP"
