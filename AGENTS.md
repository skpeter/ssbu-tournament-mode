## Learned User Preferences

- Prefer leaving in-game Versus **First to = 1 Win** and tracking set length in software; do not fight vanilla FT>1 rematch/results/SSS multi-pick behavior.
- When FT-override / spoof approaches keep regressing or crashing, pivot the design rather than continuing fragile engine overrides.
- Prefer incorporating CSS-first into this plugin instead of running standalone `libcss_first.nro` alongside it.
- Prefer a top-center scoreboard with names beside the win grid (not above it): one cell per game the ruleset requires, filled for a win and hollow for a game still needed. Left side names are right-aligned against their grid (cells fill from the right); right side names are left-aligned (cells fill from the left). Doubles stacks both teammate tags. Show it on Versus ruleset presets whose names contain `FT2` or `FT3`; for 4+ fighters enable only when `TeamModule::team_no` yields exactly two teams (skip FFA / 3+ teams). Keep it up on character select, stage select, the match, and results while the set is in progress; hide it when a fighter is near the top during the match (threshold tuned from the temporary Y debug overlay); after first-to, hide it once results finishes and show an empty grid when the next game starts. Reject end-of-game toasts and skyline-web DialogOk (applet face buttons unreliable; web UI mid scene_transition can crash).
- Docked scoreboard chrome is 2× with a more opaque background; handheld, tabletop, and a 720p image stay the original size. 3× was too big. Prefer a real larger font size over stretching the glyphs.
- Draw that scoreboard with imgui-smash. Do not write vanilla battle HUD panes (`info_melee` / `pane_set_text`); that Data Aborted on Melee load.
- Proximity hide is a smashline world-Y cutoff (top band once tuned; temporarily off with a Y debug box). Do not call `world_to_screen` from fighter Main (that crashed smush_info on the KO camera). Screen-space overlap is a later follow-up. Team side comes from `TeamModule::team_no` (same as smush_info), not port index alone.
- Prefer SSS on-screen chrome from Midwest Lua / layout.arc over Rust hijacking random SSS text panes.
- Game-1 who-bans-first / RPS is IRL-only — do not reintroduce in-plugin RPS or L/R confirm UI; scoreboard HUD is higher priority.
- Stage-history is stage-select only, drawn with imgui-smash as separate boxes below the top scoreboard: one box per result, winner tag(s) and stage name centered, no character stock icon. The scoreboard itself stays the win grid with names beside it.

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
- The stage strip reads `app::sv_information::stage_id` for a normal pick. Random leaves that id invalid (`-1`); the played stage is `app::stage::get_stage_id` or the id cached while the match was running. That number is the `StageID` constant: Final Destination is 3 and Battlefield is 0. A negative id or a system id (training/results) is not a strip cell. Display names are a manual map of those constants, including the omega and battlefield forms of the legal list.
- HUD 2× (`hud_font_scale`) applies only when `nn::oe::GetOperationMode` is docked and the imgui display width is at least 1600. Eden reports docked mode on a 720p framebuffer; baking a 2× font from operation mode alone crashed character select. The draw uses `SetWindowFontScale` under that width gate.
- Scoreboard fighter-height samples use smashline. Do not install both `libsmashline_plugin.nro` and `libsmashline_hook.nro` (that pair crashes after character select). Do not copy smush_info's match feed, sockets, or replay dump.
