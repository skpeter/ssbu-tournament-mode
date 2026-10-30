# Deferred / follow-ups

## Scoreboard

### Screen-space scoreboard hide

**What:** Hide the in-match scoreboard from fighter position on screen, sampled in the imgui draw callback.

**Why:** World Y can leave the line over a low recovery, or hide it while both fighters are visually in the top half. The original request was "near or above the scoreboard."

**Context:** v1 ships a world-Y band via smashline `on_line(Main)` at rev `be3ef308ed8e479922da192a391e0684184af2af`, reading `PostureModule` Y. If hardware shows a fighter covering the line, or the line hiding while both fighters are visually up high, record the miss and ship the board. The follow-up sample runs from the existing imgui draw callback, skips while place is `Results`, and must not call `world_to_screen` from fighter Main (that call crashed smush_info on the KO replay camera). Design: `docs/designs/in-match-scoreboard.md`.

**Effort:** L
**Priority:** P2
**Depends on:** A screen read that does not call `world_to_screen` from fighter Main. The world-Y board does not wait on this.

### Stage history line

**What:** A second scoreboard row listing the stages already played in the set.

**Why:** Players at the setup can see the set history without remembering it. This was the unlimited-time extra.

**Context:** The first board is one line: port-colored tags, score in the middle, no animation, bottom-center. A second row would sit in the blast zone. This plugin does not store stage history. Stage striking belongs to the Midwest SSS pack. The design leaves the line unbuilt. See `docs/designs/in-match-scoreboard.md`.

**Effort:** M
**Priority:** P3
**Depends on:** A source for stages already played. The world-Y board does not wait on this.

## Other follow-ups

- Upstream HDR contribution of `TournamentMode` API (avoid owning full SSS forever).
- In-game config submenu (strike preset, DSR, scoreboard) if TOs hate SD TOML.
- Doubles / team set scoring.
- Online arena set sync (explicit non-goal for v1).
- Bracket / stream overlay integration.
- SET! VO packaging once JP in-game hash spike concludes (keep separate download until then).
- Compat matrix automation against HDR / Training Modpack / Midwest releases.
- Emulator-specific timing quirks doc after first HW green run.
