## Learned User Preferences

- Prefer leaving in-game Versus **First to = 1 Win** and tracking set length in software; do not fight vanilla FT>1 rematch/results/SSS multi-pick behavior.
- When FT-override / spoof approaches keep regressing or crashing, pivot the design rather than continuing fragile engine overrides.
- Prefer incorporating CSS-first into this plugin instead of running standalone `libcss_first.nro` alongside it.
- Prefer an in-match scoreboard overlay during play over end-of-game toasts; reject skyline-web DialogOk/toasts (applet face buttons unreliable; web UI mid scene_transition can crash).
- Prefer SSS on-screen chrome from Midwest Lua / layout.arc over Rust hijacking random SSS text panes.
- Game-1 who-bans-first / RPS is IRL-only — do not reintroduce in-plugin RPS or L/R confirm UI; scoreboard HUD is higher priority.

## Learned Workspace Facts

- This repo is a Skyline/ARCropolis Smash Ultimate tournament set-mode plugin (`ssbu-tournament-mode`).
- Target game version is Smash Ultimate ≤ 13.0.4.
- Runtime config lives at `sd:/ultimate/tournament_mode.toml`; plugin log at `sd:/tournament_mode.log`.
- Set length: prefer parsing `FT2`/`FT3` from the selected Versus **ruleset preset name**; TOML `set_length` (alias `first_to_override`) is fallback. Engine First to stays 1 Win.
- Do not load `libcss_first.nro` with this plugin (same CSS↔SSS hooks); CSS-first is built in (AGPL; see NOTICE).
- Stage striking depends on the companion Midwest Stagelist SSS + Striking pack (separate from this plugin).
- SSS prompt/score chrome should be driven by Midwest `stage_select_actor3` via `TournamentMode.*` and/or a real layout.arc, not Rust pane-name guessing.
- FTP deploy (sys-ftpd) is used for flashing builds/logs; do not store host/credentials in repo docs.
