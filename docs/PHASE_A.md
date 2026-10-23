# Phase A — Kill-gate (v0.2 approach)

Must pass on **13.0.4** hardware (or a faithful emu) before Phase B/C feature work.

## Setup (v0.2.1+)

1. Install `libssbu_tournament_mode.nro` **0.2.1+**
2. `sd:/ultimate/tournament_mode.toml` with `enabled = true` and fallback `set_length = 3`
3. **In-game First to = 1 Win** on every tournament preset
4. Save named presets that include the FT, e.g. `Midwest FT2` and `Midwest FT3`
5. Fresh log; remove `libcss_first.nro` if present

Set length is read from the **selected ruleset name** (`FT2` / `FT3` token) when
the MeleeRule BSS scan finds it — switch presets mid-bracket without reboot.
TOML `set_length` is fallback only.

Vanilla FT>1 is a different match machine; do not use it for tournament sets.

## Pass criteria

| # | Check | Pass if |
|---|---|---|
| A0 | Boot | `probe: ok`, hooks installed, `approach=engine_FT1+config_set_length` |
| A1 | Results every game | After game 1, full results (vanilla FT1) |
| A2 | CSS between games | After results, character select available |
| A3 | SSS between games | One stage pick (not multi-FT slots) |
| A4 | Inert | `set_length = 1` or `enabled = false` → no set tracking |
| A5 | Reset | Leaving to rules/main menu logs `set: reset` |
| A6 | Warn | If in-game FT>1, log shows the WARN line |

## Log signals

```
probe: ... approach=engine_FT1+set_length_from_ruleset_name
config: set_length=3
ruleset_name: hit text+0x... "Midwest FT3" → set_length=3
ruleset[MeleeRule]: engine_ft_wins=1 set_length=3 src=name ...
game_end: ... set_length=3 ...
```

If the name scan misses (name not in BSS yet):

```
ruleset_name: no UTF-16 FT# name in BSS scan — using toml set_length
```

Pinned primary name slot (0.2.2+): `text+0x530456c` (e.g. `"Competitive FT3"`).

Do **not** start Phase C striking / UI until A1–A3 pass with engine FT1
(`engine_ft_wins=1`, results after game 1, single-stage SSS).
