#!/usr/bin/env bash
set -euo pipefail
source "$HOME/.cursor/skills/gstack/bin/gstack-codex-probe"
_REPO_ROOT=$(git rev-parse --show-toplevel) || { echo "ERROR: not in a git repo" >&2; exit 1; }
PLAN_PATH="$_REPO_ROOT/PLAN.md"
_gstack_codex_timeout_wrapper 600 codex exec "IMPORTANT: Do NOT read or execute any SKILL.md files or files in skill definition directories (paths containing skills/gstack). These are AI assistant skill definitions meant for a different system. Stay focused on repository code only.

You are a CEO/founder advisor reviewing a development plan.
Challenge the strategic foundations: Are the premises valid or assumed? Is this the
right problem to solve, or is there a reframing that would be 10x more impactful?
What alternatives were dismissed too quickly? What competitive or market risks are
unaddressed? What scope decisions will look foolish in 6 months? Be adversarial.
No compliments. Just the strategic blind spots.
File: $PLAN_PATH" -C "$_REPO_ROOT" -s read-only -c 'web_search="cached"' < /dev/null
_CODEX_EXIT=$?
echo "CODEX_EXIT=$_CODEX_EXIT"
if [ "$_CODEX_EXIT" = "124" ]; then
  echo "[codex stalled past 10 minutes — tagging as unavailable]"
fi
