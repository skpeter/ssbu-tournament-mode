#!/usr/bin/env bash
set -euo pipefail
cd "$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
eval "$("$HOME/.cursor/skills/gstack/bin/gstack-slug" 2>/dev/null || true)"
SLUG="${SLUG:-ssbu-tournament-mode}"
mkdir -p "$HOME/.gstack/projects/$SLUG"
BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null | tr '/' '-' || echo main)
DATETIME=$(date +%Y%m%d-%H%M%S)
COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo none)
RESTORE="$HOME/.gstack/projects/$SLUG/${BRANCH}-autoplan-restore-${DATETIME}.md"
{
  echo "# /autoplan Restore Point"
  echo "Captured: $(date -u +%Y-%m-%dT%H:%M:%SZ) | Branch: $BRANCH | Commit: $COMMIT"
  echo
  echo "## Re-run Instructions"
  echo "1. Copy \"Original Plan State\" below back to your plan file"
  echo "2. Invoke /autoplan"
  echo
  echo "## Original Plan State"
  cat PLAN.md
} > "$RESTORE"
echo "RESTORE_PATH=$RESTORE"
echo "SLUG=$SLUG"
echo "BRANCH=$BRANCH"
echo "BASE_BRANCH=main"
_LOCALDOC=$(ls -t "$HOME/.gstack/projects/$SLUG"/*-design-*.md 2>/dev/null | head -1 || true)
echo "DESIGN_DOC=${_LOCALDOC:-none}"
python3 - <<'PY'
import re
text=open("PLAN.md",encoding="utf-8").read().lower()
ui=len(re.findall(r"\b(component|screen|form|button|modal|layout|dashboard|sidebar|nav|dialog|scoreboard|prompt|hud|graphic)\b", text))
dx=len(re.findall(r"\b(api|endpoint|cli|command|flag|sdk|library|package|npm|import|skill\.md|mcp|agent|onboarding|integration|debug|implement|error message|toml|config|plugin)\b", text))
print(f"UI_MATCHES={ui}")
print(f"DX_MATCHES={dx}")
print("UI_SCOPE=" + ("yes" if ui>=2 else "no"))
print("DX_SCOPE=" + ("yes" if dx>=2 else "no"))
PY
