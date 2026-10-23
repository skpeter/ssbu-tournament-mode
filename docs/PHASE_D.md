# Phase D — In-match scoreboard

## Done

| Piece | Status |
|---|---|
| Win attribution → `set_state.wins` | Done |
| `TournamentMode.get_score_p1/p2` / `get_score_line` | Done |
| Melee enter/leave bookkeeping | Done |
| `info_melee` pane `set_text` | **Abandoned** — Data Abort 2168-0002 on Melee enter |
| imgui-smash overlay (ssbu-online-deluxe host) | **0.3.9** |
| End-of-game DialogOk / toast | Rejected |

## Overlay

Same drawing path as [ssbu-online-deluxe](https://github.com/saad-script/ssbu-online-deluxe): companion `libimgui_smash.nro` + draw callback. Not vanilla HUD panes.

1. Place `libimgui_smash.nro` in `atmosphere/contents/01006A800016E000/romfs/skyline/plugins/`.
2. `scoreboard = true` in TOML.
3. Reboot Smash, play a local FT2/FT3 game.
4. Top-center: `SET  0-1  FT3`. Log: `score_overlay: imgui-smash draw registered`.

`scoreboard = false` is the kill switch.
