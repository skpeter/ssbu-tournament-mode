# SSBU Tournament Mode (Rivals-style Set Mode)

## Goal

Ship a Skyline + ARCropolis package for Super Smash Bros. Ultimate (target up to **13.0.4**) that makes local Versus behave like Rivals of Aether 2 tournament/set mode when config **`set_length` > 1** (in-game First to stays at **1 Win**):

1. Show the full results screen after every game (so replays can be saved).
2. Return players to CSS + SSS between every game of the set (characters and stages reselectable).
3. Drive structured stage striking on SSS (RPS → ban order → winner strikes first on game 2+).
4. Show a minimal in-match set scoreboard that fades out near fighters.
5. Optional: play Japanese **SET!** VO/graphic on the final game of the set.

## Non-goals (v1)

- Online / arena / battle arenas synchronization.
- Full bracket / stream overlay integration.
- Replacing HDR's entire stagelist editor UI.
- Supporting Squad Strike / Tourney mode menus (local Versus + First to only).

## Product behavior

### Activation

- Active when `enabled` and software **set length > 1**.
- **In-game ruleset First to must be 1 Win.** Vanilla FT>1 is a separate rematch
  machine; we do not spoof it. Engine FT1 gives results + CSS/SSS for free.
- **Set length source (preferred):** selected Versus **preset name** containing
  `FT2` / `FT3` / `FT n` (e.g. `Midwest FT3`). Switching presets mid-bracket
  updates length without reboot. TOML `set_length` is fallback only.
- When set length ≤ 1, plugin is inert for set logic (CSS-first may still run).

### Set flow

```
Preset name encodes N (e.g. "Midwest FT3"), ruleset First to = 1
  → Game 1: CSS → SSS (RPS optional + starter strikes) → Match
  → Results (vanilla FT1) → save replay if desired
  → Game k: CSS → SSS (winner strikes; counterpicks; optional DSR) → Match
  → When a player reaches N wins: set ends (optional SET! on deciding game)
```

### Stage striking (Midwest / HDR script integration)

Reference pack: `Midwest Stagelist SSS + Striking - No Alts` (Ontario naming in `info.toml`), which ships compiled `ui/script_patch/common/stage_select_actor3.lc` based on HDR + Stage Alts (`HDR.*` Lua bridge).

Upstream source of truth for striking logic:

- HDR `romfs/source/ui/script_patch/common/stage_select_actor3.lua`
- HDR `src/lua.rs` (`HDR.get_bans`, `HDR.get_dsr`, `HDR.is_css_first`, perma-strike helpers)
- Related: `blu-dev/stage-alts-2`

**Presence check:** patched SSS Lua must no-op into existing Midwest/HDR behavior unless this plugin is loaded. Prefer exporting a Lua singleton (`TournamentMode.is_present()` / `is_active()`) from the Skyline plugin (same pattern as HDR's `push_new_singleton`). Fallback: SD flag file if Lua registration timing is awkward.

Configurable strike sequences (SD config):

| Preset key | Meaning |
|---|---|
| `1-2-any` | P1 bans 1, P2 bans 2, remaining is played (or picker chooses among remaining) |
| `3-4-any` | Longer starter ban chains used by some TO stagelists |
| custom | Ordered list of `{side, bans}` steps + final pick rule |

Game 1:

- Optional on-screen RPS prompt ("do RPS, then confirm who bans first").
- After confirm, SSS input is delegated only to the striking player for each step.

Game 2+:

- No RPS prompt.
- First strike control awarded to **game 1 winner** (then continue configured sequence / counterpick rules).
- Auto-ban all stages marked counterpick for the active stagelist page.
- Setting: **auto-strike stages the picking player has already won on this set** (DSR / mDSR-style; HDR already exposes `get_dsr` — reuse semantics where possible).

### In-match scoreboard

- Small top HUD: `P1 wins — P2 wins` (or tag names if cheap).
- Mirror vanilla HUD proximity fade: reduce opacity / hide when fighters are near the pane so it never blocks gameplay reads.

### Optional SET! (final game)

- Asset pack from GameBanana mod 448516 / local `Downloads/game_set_over_game_afbf6.7z` (JP SET VO/graphic).
- On deciding game only, swap GAME! announcement/graphic to SET!.
- Investigation spike first: may already exist in JP/localization files; if so, trigger existing hash instead of redistributing assets.
- Feature-flagged; safe to ship disabled by default until assets/legal path is clear.

## Architecture

```
[Ruleset First to] --read--> SetController (Rust/Skyline)
        |                         |
        |                         +--> ResultsGate (force show results / allow continue)
        |                         +--> CssSssGate (force CSS+SSS between games)
        |                         +--> ScoreState (wins per port/team)
        |                         +--> StrikeSession (who controls SSS, ban steps, DSR)
        |                         +--> ScoreboardHud (layout + proximity fade)
        |                         +--> Set annoucer (optional SET!)
        v
TournamentMode.* Lua API  <-->  patched stage_select_actor3 (Midwest fork)
        ^
ARCropolis romfs: scoreboard layout, optional SET assets, SSS script_patch
```

### Crates / deps (known good on this machine)

Mirror `ssbu-hide-replay-tips` where possible; expand only as needed:

- `skyline` (`ultimate-research/skyline-rs`)
- `skyline-smash` / `smash` bindings as required for `FighterManager`, ruleset reads, UI hooks
- `arcropolis_api` callbacks for layout patching (proven in hide-replay-tips)
- Optional: `nro-hook` / `nn_hid_hook` patterns from `jugeeya/results-screen-skip` (inverse use: we keep results, don't skip them)
- Lua bridge: follow HDR `rlua_lua53_sys` + singleton registration approach for SSS communication

### Repo layout (proposed)

```
ssbu-tournament-mode/
  Cargo.toml                 # Skyline cdylib plugin
  src/
    lib.rs                   # main + install
    set_state.rs             # wins, game index, activation
    ruleset.rs               # read First to / players
    results.rs               # keep results screen in set
    flow.rs                  # CSS/SSS between games
    strike.rs                # strike session + Lua API
    scoreboard.rs            # HUD + proximity
    set_callout.rs           # optional SET!
    config.rs                # SD TOML
  mod/                       # ARCropolis-friendly pack
    ui/script_patch/...      # stage_select_actor3 patch
    ui/layout/...            # scoreboard
  config/tournament_mode.toml.example
  scripts/package-sd.ps1
  README.md
  PLAN.md
```

## Implementation plan

### Phase A — Spike (must succeed before feature work)

1. Confirm how vanilla First to > 1 skips results / locks CSS (hooks/offsets on 13.0.4).
2. Confirm reading First to from the active ruleset.
3. Confirm win attribution per game (which port/team won).
4. Confirm Lua singleton registration from a standalone NRO (without full HDR).
5. Inventory SET! assets vs in-game JP hashes.

### Phase B — Set controller MVP

1. Activate only when First to > 1.
2. Track set score across games; reset when leaving Versus / changing ruleset.
3. Force results screen visibility after each non-terminal? and terminal game.
4. Force CSS + SSS between games instead of instant rematch / locked chars.

### Phase C — Striking integration

1. Fork Midwest `stage_select_actor3` source from HDR lua; keep manual X-ban path when plugin absent.
2. Add `TournamentMode` Lua API: present/active, game index, strike controller port, sequence preset, auto-ban counterpicks, DSR list.
3. Game 1 who-bans is IRL-only (no in-plugin RPS); software turn order from game 2+.
4. Input gating (game 2+): only striking port's inputs move/ban; other port locked for that step.
5. Config presets `1-2-any` and `3-4-any`.

### Phase D — Scoreboard + SET!

1. Minimal top scoreboard layout via ARCropolis +/or runtime pane create.
2. Proximity fade matching info HUD behavior.
3. Optional SET! behind config flag.

### Phase E — Package / docs / test matrix

1. SD zip layout under `atmosphere/contents/01006A800016E000/...` + `ultimate/mods/...`.
2. README: requires Skyline + ARCropolis; works with Midwest striking pack; conflict notes vs HDR full / Training Modpack.
3. Hardware + emulator test checklist (13.0.4).

## Config sketch (`sd:/ultimate/tournament_mode.toml`)

```toml
enabled = true
strike_preset = "1-2-any"          # or "3-4-any"
auto_ban_counterpicks_after_g1 = true
auto_strike_picker_prior_wins = true  # DSR-like
scoreboard = true
set_callout_on_final = false
```

## Risks

| Risk | Impact | Mitigation |
|---|---|---|
| First-to flow hooks are version-fragile | Breaks on patch | Isolate offsets; document 13.0.4 only; probe at boot |
| Midwest ships `.lc` not source | Hard to patch | Rebuild from HDR lua via smash-lua; ship our `.lc` |
| HDR already owns SSS Lua bridge | Symbol/API clash | Separate `TournamentMode` singleton; detect HDR |
| Dual control / medal UI assumptions | Softlocks on SSS | Explicit input gate + cancel/reset hold |
| SET! asset redistribution | Legal/pack size | Prefer in-game JP asset; else optional separate download |
| Emulator vs hardware timing | False negatives in spikes | Test both; log to `sd:/tournament_mode.log` |

## Success criteria

- First to 3 local set: results every game, CSS+SSS every game, scoreboard correct, strike control follows winner, counterpicks auto-banned game 2+.
- With plugin removed, Midwest striking pack behaves as today.
- First to 1 unchanged from vanilla/mod baseline.

## Open questions (for review / gate)

1. Should v1 require the Midwest/HDR striking pack, or ship a standalone SSS patch?
2. Is SET! in v1 or a fast-follow behind the flag?
3. Config via TOML only, or also an in-game pause/rules submenu?
4. Exact semantics for `3-4-any` / gentleman's clause (who picks among remaining)?

---

## Autoplan amendments (applied)

Mode: **SELECTIVE EXPANSION**. Codex voices unavailable (usage limit) → `[subagent-only]` for all phases.

### Locked by auto-decide (6 principles)

| Decision | Choice | Principle |
|---|---|---|
| Phase A | Hard kill-gate before B/C/D; fail-closed inert on probe fail | P1, P5 |
| v1 player modes | **1v1 only**; doubles deferred to TODOS | P5 |
| Gentleman / `*-any` | Striking port picks among remaining stages | P5 |
| Softlock recovery | Hold-to-reset strike session + on-SSS “waiting / your turn” chrome | P1 |
| Boot logging | `sd:/tournament_mode.log` with present/active/inactive reason/version | P1 |
| SET! default | `set_callout_on_final = false`; not in default zip until JP hash spike | P3 |
| Config v1 | TOML only; in-game menu deferred | P3 |
| Lua safety | Guard `TournamentMode` and `HDR` nil; Midwest manual X-ban preserved when inactive | P5 |
| Magical moment | After game 1 of FT3: full results then CSS (not SET!) | P5 |
| Success criteria sequencing | Ship all planned features, but Phase B green on HW is the release blocker; striking/scoreboard/SET are sequenced after A | P1+P2 |

### Taste decisions (need your call at Final Gate)

1. **SSS packaging:** require Midwest + companion delta patch (**recommended**) vs standalone SSS fork vs upstream HDR API first.
2. **v1 definition of done:** full feature checklist in Goal (**your original**) vs set-flow-only MVP then 1.1 striking (**CEO recommend**).
3. **Config surface:** TOML only (**recommended for v1**) vs also pause/rules submenu.
4. **SET! packaging:** flag-only / separate download (**recommended**) vs include assets in main zip.

### UI specs added from Design review

**SSS hierarchy (first → last):** (1) whose turn / step “P2 bans 1 of 2”, (2) stage grid with banned dimmed, (3) RPS/confirm modal when active, (4) stagelist page chrome.

**RPS (game 1):** IRL only — plugin does not prompt or gate bans on game 1.

**Bridge beat:** after results advance, brief set status `1–0 · Game 2` before CSS (even a log + SSS banner is enough for v1).

**Scoreboard:** top-center, small digits `W–W`, high contrast white with dark shadow; fade to 0 when any fighter world-pos projects near top 15% of screen (match info-HUD spirit; exact distance tuned on HW). Sticky set score also shown on results (reuse text pane if possible).

**Final game without SET! assets:** show non-VO `FINAL` cue via existing UI text if cheap; else skip.

### Architecture ASCII (Eng)

```
[SSBU 13.0.4]
    |
    +-- Skyline NRO tournament_mode
    |     SetController --> ResultsGate / CssSssGate / ScoreState
    |                   --> StrikeSession --> TournamentMode.* Lua
    |                   --> ScoreboardHud / SetCallout(opt)
    |
    +-- ARCropolis romfs (scoreboard layout, SSS script_patch)
    |
    +-- Midwest stage_select_actor3.lc (required companion)
          may call HDR.* when HDR present (bans/dsr/perma)
```

### Error & Rescue Registry

| Error | Trigger | User sees | Rescue |
|---|---|---|---|
| `ProbeFail` | Offset/symbol miss on boot | Normal game; log `inert: probe_fail` | Update plugin for build |
| `ConfigParse` | Bad TOML | Defaults; log warning | Fix TOML / delete file |
| `MidwestMissing` | No striking pack | Set flow still works; unstructured SSS | Install Midwest pack |
| `HdrNil` | HDR API absent | Striking without DSR/CP metadata; no crash | Optional HDR/stage config |
| `StrikeSoftlock` | Input gate stuck | On-SSS hold-reset hint | Hold reset; leave SSS |
| `MidSetQuit` | Home/exit Versus | Next entry starts fresh set | Automatic reset FSM |
| `SetAssetsMissing` | SET! on, assets gone | GAME! unchanged; log once | Install asset pack or disable flag |

### Failure Modes Registry

| ID | Mode | Critical? | Mitigation in plan |
|---|---|---|---|
| F1 | Half-hooked First-to flow | Y | Fail-closed boot |
| F2 | SSS softlock | Y | Hold-reset + turn chrome |
| F3 | Dual `.lc` romfs fight | Y | Midwest-required packaging decision |
| F4 | Lua nil crash | Y | Presence guards |
| F5 | Ghost bans next set | Y | Reset FSM |
| F6 | Conflict with results-skip / Training / HDR | Y | Conflict matrix in README |
| F7 | Config panic abort | N | Pure parse + defaults |

### NOT in scope

- Online/arena sync, brackets/overlays, HDR stagelist editor, Squad Strike/Tourney menus, doubles, in-game config menu (v1), SET! as default package.

### What already exists

- Midwest/HDR SSS striking + `HDR.get_bans/get_dsr/is_css_first/perma_strike` (`refs/hdr-lua.rs`, `refs/stage_select_actor3.hdr.lua`)
- `ssbu-hide-replay-tips` ARCropolis callback + packaging pattern
- `jugeeya/results-screen-skip` FighterManager/HID patterns (inverse use)
- Local SET archive under Downloads; JP in-game hash TBD
- Vanilla First-to >1 machine to invert

### Dream state delta

```
CURRENT: First to >1 skips results, locks chars, cycles stages; striking is manual Midwest X-ban
THIS PLAN: Local FT sets with results+CSS/SSS+structured strikes+scoreboard
12-MONTH IDEAL: Set-engine API other packs call; optional upstream HDR; TO-friendly config; minimal fork surface
```

### Cross-phase themes

1. **Midwest / `.lc` ownership** — CEO + Eng (ecosystem conflict is the #1 ship risk).
2. **Phase A hook feasibility** — CEO + Eng (no product without it).
3. **Softlock / turn feedback** — Design + Eng.
4. **Ready-SD DX (zip + verify + errors)** — DX + Eng.

### Decision Audit Trail

| # | Phase | Decision | Classification | Principle | Rationale | Rejected |
|---|---|---|---|---|---|---|
| 1 | CEO | Mode SELECTIVE EXPANSION | Mechanical | autoplan default | Best for greenfield with expansions | HOLD/REDUCTION |
| 2 | CEO | Skip /office-hours | Mechanical | P6 + skip list | No design doc; keep pipeline moving | Run office-hours now |
| 3 | CEO | Phase A hard kill-gate | Mechanical | P1/P5 | Entire product depends on hooks | Soft checklist |
| 4 | CEO | 1v1 only v1 | Mechanical | P5 | Win attribution underspecified for teams | Doubles in v1 |
| 5 | CEO | Keep full Goal features, sequence after A | Taste | P1 vs CEO cut | Honor user completeness; sequence risk | Cut striking from v1 DoD |
| 6 | CEO | Recommend Midwest-required companion | Taste | P3 | Avoid dual `.lc` ownership | Standalone SSS / HDR-first |
| 7 | Design | RPS two-step + turn chrome + hold-reset | Mechanical | P1 | Prevent softlock/haunting ambiguity | Vague “do RPS” prompt |
| 8 | Design | Scoreboard top-center + proximity rule | Mechanical | P1 | Minimal distraction intent | Fancy HUD |
| 9 | DX | README + zip + boot verify before polish | Mechanical | P1/P5 | TTHW ≤5 on ready SD | Docs-last |
| 10 | DX | Magical moment = results→CSS after G1 | Mechanical | P5 | Clearest value | SET! as hero |
| 11 | Eng | Fail-closed probe | Mechanical | P5 | No half-hooked sets | Best-effort hooks |
| 12 | Eng | Gentleman = striking port picks remaining | Mechanical | P5 | Explicit default | Unspecified |
| 13 | Eng | TOML-only config v1 | Taste | P3 | Faster; escape hatch later | In-game menu v1 |
| 14 | All | Codex unavailable → subagent-only | Mechanical | infra | Usage limit Oct 8 | Block pipeline |

---

## GSTACK REVIEW REPORT

### CEO
- Mode: SELECTIVE EXPANSION
- Voices: Claude subagent only `[subagent-only]` (Codex usage limit)
- Consensus table (Claude only; Codex = N/A):

```
CEO DUAL VOICES — CONSENSUS TABLE:
  Dimension                           Claude  Codex  Consensus
  1. Premises valid?                   partial N/A    N/A
  2. Right problem to solve?           yes     N/A    N/A
  3. Scope calibration correct?        partial N/A    N/A
  4. Alternatives sufficiently explored? no    N/A    N/A
  5. Competitive/market risks covered? partial N/A    N/A
  6. 6-month trajectory sound?         partial N/A    N/A
```

- Key: right problem; kill-gate Phase A; do not become “another SSS pack.”

### Design
- UI scope: yes
- Voices: `[subagent-only]`
- Litmus (0–10): hierarchy 3→amended, states 2→registry added, journey 5, specificity 3→specs added, a11y 1 (deferred high-contrast note), layout 2→scoreboard spec, minimalism 6
- Consensus: N/A dual (Codex missing)

### DX
- DX scope: yes (plugin/TOML/Lua API)
- Overall ~2.6/10 → checklist in plan; TTHW ready-SD 20–45 → ≤5
- Voices: `[subagent-only]`

### Eng
- Voices: `[subagent-only]`
- Architecture partial; tests no→test plan written; perf yes; security partial; error paths no→registry; deploy risk no until Q1 locked

```
ENG DUAL VOICES — CONSENSUS TABLE:
  Dimension                           Claude  Codex  Consensus
  1. Architecture sound?               partial N/A    N/A
  2. Test coverage sufficient?         no      N/A    N/A
  3. Performance risks addressed?      yes     N/A    N/A
  4. Security threats covered?         partial N/A    N/A
  5. Error paths handled?              no→amended N/A N/A
  6. Deployment risk manageable?       no until Q1 N/A N/A
```

### Artifacts
- Test plan: `~/.gstack/projects/ssbu-tournament-mode/samul-main-test-plan-*.md`
- TODOS.md: written
- Refs pulled: HDR `stage_select_actor3.lua`, `hdr-lua.rs`, results-screen-skip, stage-alts
- Restore: see `~/.gstack/projects/ssbu-tournament-mode/*-autoplan-restore-*.md` (empty-repo HEAD quirk on first attempt)

STATUS: APPROVED — 2026-10-08 — user chose Final Gate **A** (accept all recommendations).
Taste locks: Midwest-required companion patch; full Goal sequenced after Phase A; TOML-only v1; SET! flag-off / separate download.

**Implementation progress:** v0.1.0 scaffolded + `cargo skyline build --release` green.
Phase A hooks (HDR offsets: `game_end`, `game_exit`, `scene_transition`, CSS↔SSS observe) log to `sd:/tournament_mode.log`.
Kill-gate still needs HW verification — see `docs/PHASE_A.md`.
