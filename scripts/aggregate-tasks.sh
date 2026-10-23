#!/usr/bin/env bash
set -euo pipefail
TASKS_DIR="$HOME/.gstack/projects/ssbu-tournament-mode"
ALL=$(mktemp)
for phase in ceo-review design-review eng-review devex-review; do
  for f in "$TASKS_DIR"/tasks-$phase-*.jsonl; do
    [ -f "$f" ] || continue
    cat "$f" >> "$ALL"
  done
done
if command -v jq >/dev/null 2>&1 && [ -s "$ALL" ]; then
  jq -r '
    [inputs] | . as $all |
    $all
  ' < /dev/null 2>/dev/null || true
  jq -s -r '
    sort_by({P1:0,P2:1,P3:2}[.priority] // 99)
    | .[]
    | "- [ ] **\(.id) (\(.priority), human: \(.effort_human) / CC: \(.effort_cc)) — \(.component)** — \(.title)\n  - Surfaced by: \(.phase) — \(.source_finding)\n  - Files: \(.files | join(", "))"
  ' "$ALL"
else
  echo "_No tasks or jq missing_"
  cat "$ALL" || true
fi
rm -f "$ALL"

GSTACK_BIN="$HOME/.cursor/skills/gstack/bin"
"$GSTACK_BIN/gstack-learnings-log" '{"skill":"autoplan","type":"operational","key":"codex-usage-limit","insight":"Codex exec can auth-pass then fail with ChatGPT usage limit; autoplan must degrade to subagent-only and continue","confidence":80,"source":"observed"}' 2>/dev/null || true
"$GSTACK_BIN/gstack-learnings-log" '{"skill":"autoplan","type":"operational","key":"empty-repo-head","insight":"git rev-parse HEAD fails before first commit; use main fallback for restore paths","confidence":70,"source":"observed"}' 2>/dev/null || true
