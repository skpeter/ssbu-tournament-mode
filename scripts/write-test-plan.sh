#!/usr/bin/env bash
set -euo pipefail
SLUG=ssbu-tournament-mode
DATETIME=$(date +%Y%m%d-%H%M%S)
OUT="$HOME/.gstack/projects/$SLUG/samul-main-test-plan-${DATETIME}.md"
mkdir -p "$HOME/.gstack/projects/$SLUG"
cat > "$OUT" <<'EOF'
# Test Plan — ssbu-tournament-mode

## Hardware matrix (13.0.4)

| Case | Expect |
|---|---|
| First to 1 | Plugin inert; vanilla flow |
| First to 3, plugin only | Results every game; CSS+SSS between games; scoreboard updates |
| First to 3 + Midwest pack | Striking RPS G1; winner first strike G2; counterpicks auto-banned |
| Mid-set Home → Versus exit | Score/strike state reset; no ghost bans |
| Ruleset change mid-set | State reset |
| Midwest absent | Documented degrade (vanilla SSS or clear log); no crash |
| HDR absent | No Lua nil crash; TournamentMode guards |
| results-screen-skip installed | No fight; results still usable |
| Bad/missing TOML | Defaults; log warning; no abort |
| SET! flag off/on | Off default; on only final game if assets/hash OK |
| Scoreboard proximity | Hides near top when fighter under pane |
| Softlock recovery | Hold-to-reset restores SSS control |

## Host-side

- Config parse unit tests (pure Rust)
- Strike FSM unit tests for presets `1-2-any`, `3-4-any`
- Presence API table smoke (document expected returns)

## Release gate

- Timed ready-SD install ≤ 5 minutes
- FT3 local set completes without softlock
EOF
echo "TEST_PLAN=$OUT"
