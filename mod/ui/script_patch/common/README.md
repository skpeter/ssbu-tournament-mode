# Midwest companion SSS patch (Phase C)

Ship a rebuilt `stage_select_actor3.lc` here that:

1. Calls `TournamentMode.is_present()` / `is_active()` when the Lua bridge lands.
2. Falls back to stock Midwest/HDR striking if the plugin is absent.
3. Shows `get_prompt()` / `get_score_line()`; turn-gates bans on game 2+ when
   `get_strike_controller()` is non-zero (game 1 stays ungated / IRL).

**Do not** place a conflicting `.lc` until coordinated with the Midwest pack.
Users should keep that pack installed; this folder holds the delta patch.

Reference sources (pulled into repo `refs/` during planning):

- HDR `stage_select_actor3.lua`
- HDR `src/lua.rs` (`HDR.get_bans`, `get_dsr`, perma-strike)
