# Phase C — Striking / UI

## Shipping

| Piece | Status |
|---|---|
| Game-1 who-bans-first | **IRL only** (no in-plugin RPS / L/R confirm) |
| skyline-web RPS | Abandoned (applet input broken) |
| Hijack SSS text panes | Abandoned (goose chase) |
| Midwest SSS shows `get_prompt()` | Next real SSS chrome path |
| `TournamentMode` Lua | Done |
| Win attribution | Done |
| Turn-gated bans | Game 2+ (game-1 winner strikes first) |

## Game 1 flow

1. Do RPS in real life; ban in whatever order you agreed.
2. Plugin does not gate SSS inputs on game 1 (`get_strike_controller() == 0`).
3. On game 2+, first-strike control follows the game-1 winner.

On-screen SSS chrome belongs in Midwest `stage_select_actor3` calling
`TournamentMode.get_prompt()` / `get_score_line()`, not in Rust guessing pane names.
