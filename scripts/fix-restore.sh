#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
SLUG=ssbu-tournament-mode
BRANCH=$(git rev-parse --abbrev-ref HEAD | tr '/' '-')
DATETIME=$(date +%Y%m%d-%H%M%S)
RESTORE="$HOME/.gstack/projects/$SLUG/${BRANCH}-autoplan-restore-${DATETIME}.md"
mkdir -p "$HOME/.gstack/projects/$SLUG"
{
  echo "# /autoplan Restore Point"
  echo "Captured: $(date -u +%Y-%m-%dT%H:%M:%SZ) | Branch: $BRANCH | Commit: none"
  echo
  echo "## Original Plan State"
  cat PLAN.md
} > "$RESTORE"
echo "RESTORE_PATH=$RESTORE"
# prepend restore comment if missing
if ! grep -q '/autoplan restore point' PLAN.md; then
  tmp=$(mktemp)
  printf '<!-- /autoplan restore point: %s -->\n' "$RESTORE" > "$tmp"
  cat PLAN.md >> "$tmp"
  mv "$tmp" PLAN.md
fi
