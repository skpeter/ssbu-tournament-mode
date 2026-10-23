# SSBU Tournament Mode

Skyline + ARCropolis plugin that makes local Versus behave like a real tournament set:

- Results screen every game (save replays)
- CSS + SSS between every game
- Structured stage striking (Midwest companion pack)
- Minimal set scoreboard
- Optional SET! on the final game (off by default)

Target: Smash Ultimate **≤ 13.0.4**.

See [PLAN.md](PLAN.md) for the full reviewed design.

## Status (v0.3.9)

**Approach:** leave **in-game First to = 1 Win**. Set length comes from the
**ruleset preset name** (`FT2` / `Midwest FT3`) so you can switch mid-bracket
without reboot. TOML `set_length` is fallback.

Game-1 who-bans-first is **IRL only**. Phase D: in-match score is an
**imgui-smash** overlay (same host as ssbu-online-deluxe). Needs
`libimgui_smash.nro`. Do not use skyline-web toasts mid-scene.

See [docs/PHASE_D.md](docs/PHASE_D.md).

**Remove `libcss_first.nro` from the SD.**

## Install (after build)

Requires [ARCropolis](https://github.com/Raytwo/ARCropolis/releases) (includes Skyline).

1. `cargo skyline build --release`
2. `powershell -File scripts/package-sd.ps1`
3. Extract the zip onto the SD root
4. Copy `config/tournament_mode.toml.example` → `sd:/ultimate/tournament_mode.toml`
5. Save Versus presets named e.g. `Midwest FT2` and `Midwest FT3`, each with **First to = 1 Win**
6. Keep `set_length = 3` in TOML as fallback if the name scan misses
7. Install the **Midwest Stagelist SSS + Striking** pack (companion; striking in Phase C)
8. Boot, select a named preset, read `sd:/tournament_mode.log` for `ruleset_name: hit …`

Plugin path:

```
atmosphere/contents/01006A800016E000/romfs/skyline/plugins/libssbu_tournament_mode.nro
```

Hold **L** on boot to skip plugins if something panics.

## Conflicts

| Mod | Notes |
|---|---|
| **libcss_first.nro** | **Incompatible.** Same CSS↔SSS hooks are inside this plugin — delete the standalone NRO. |
| Midwest SSS + Striking | **Required** companion for striking (Phase C). |
| HDR full | Already includes CSS-first; using both double-swaps. Prefer one or the other. |
| results-screen-skip / Auto-Save Replays / smush_info HID skip | Fight over results input; prefer one results owner. |
| Training Modpack | Usually OK offline. |
| **libimgui_smash.nro** | **Required** for the in-match score overlay (same host plugin [ssbu-online-deluxe](https://github.com/saad-script/ssbu-online-deluxe) uses). Download from [imgui-smash releases](https://github.com/Coolsonickirby/imgui-smash/releases). |

## Build

```sh
cargo install cargo-skyline
cargo skyline build --release
```

NRO: `target/aarch64-skyline-switch/release/libssbu_tournament_mode.nro`

## Phase A checklist

See [docs/PHASE_A.md](docs/PHASE_A.md).

## License

AGPL-3.0-or-later (CSS-first logic from HDR / standalone `css_first`). See `LICENSE` and `NOTICE.md`.  
Do not redistribute Nintendo assets. SET! assets stay optional/separate.
