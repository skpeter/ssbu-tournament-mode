## Learned User Preferences

- Prefer leaving in-game Versus **First to = 1 Win** and tracking set length in software; do not fight vanilla FT>1 rematch/results/SSS multi-pick behavior.
- When FT-override / spoof approaches keep regressing or crashing, pivot the design rather than continuing fragile engine overrides.
- Prefer incorporating CSS-first into this plugin instead of running standalone `libcss_first.nro` alongside it.
- Prefer a bottom-center, one-line in-match scoreboard (player tags and a win grid, no stage-history row): one cell per game the ruleset requires, filled for a win and hollow for a game still needed. Show it only on Versus ruleset presets whose names contain `FT2` or `FT3`, and only for 1v1 / at most 3 fighters — hide when `FighterManager::entry_count` is greater than 3 (no doubles/FFA scoreboard). Keep it up on character select, stage select, the match, and results while the set is in progress; hide it when a fighter is near or above it during the match; after first-to, hide it once results finishes and show an empty grid when the next game starts. Reject end-of-game toasts and skyline-web DialogOk (applet face buttons unreliable; web UI mid scene_transition can crash).
- Draw that scoreboard with imgui-smash. Do not write vanilla battle HUD panes (`info_melee` / `pane_set_text`); that Data Aborted on Melee load.
- v1 proximity hide is a smashline world-Y cutoff. Do not call `world_to_screen` from fighter Main (that crashed smush_info on the KO camera). Screen-space overlap is a later follow-up.
- Prefer SSS on-screen chrome from Midwest Lua / layout.arc over Rust hijacking random SSS text panes.
- Game-1 who-bans-first / RPS is IRL-only — do not reintroduce in-plugin RPS or L/R confirm UI; scoreboard HUD is higher priority.
- Stage-history strip is stage-select only, drawn with imgui-smash as text and rects: the winner's tag above each played stage, no character stock icon. The in-match scoreboard stays the one-line win grid.

## Learned Workspace Facts

- This repo is a Skyline/ARCropolis Smash Ultimate tournament set-mode plugin (`ssbu-tournament-mode`).
- Target game version is Smash Ultimate ≤ 13.0.4.
- Runtime config lives at `sd:/ultimate/tournament_mode.toml`; plugin log at `sd:/tournament_mode.log`.
- Set length comes only from `FT2`/`FT3` in the selected Versus **ruleset preset name**. There is no TOML `set_length` or `first_to_override`. A name without that marker leaves the set and scoreboard inactive. Engine First to stays 1 Win.
- Do not load `libcss_first.nro` with this plugin (same CSS↔SSS hooks); CSS-first is built in (AGPL; see NOTICE).
- Stage striking depends on the companion Midwest Stagelist SSS + Striking pack (separate from this plugin).
- SSS prompt/score chrome should be driven by Midwest `stage_select_actor3` via `TournamentMode.*` and/or a real layout.arc, not Rust pane-name guessing.
- FTP deploy (sys-ftpd) is used for flashing builds/logs; do not store host/credentials in repo docs.
- In-match drawing uses imgui-smash: `libimgui_smash.nro` on the SD plus linked `libimgui_smash.a`. Skyline `LookupSymbol` does not resolve `igBegin`/`igText`. That build cannot draw plugin-supplied images (PNG or the game's BNTX stage thumbnails): `igImage` does not upload pixels, the NVN backend only has the font texture, and passing a user texture can fault the game.
- The stage strip reads `app::sv_information::stage_id`, the same call smush_info makes on fighter entry. That number is the `StageID` constant: Final Destination is 3 and Battlefield is 0. A negative id is not a strip cell. Display names are a manual map of those constants, including the omega and battlefield forms of the legal list.
- Scoreboard fighter-height samples use smashline. Do not install both `libsmashline_plugin.nro` and `libsmashline_hook.nro` (that pair crashes after character select). Do not copy smush_info's match feed, sockets, or replay dump.
