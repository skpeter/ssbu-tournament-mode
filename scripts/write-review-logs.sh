#!/usr/bin/env bash
set -euo pipefail
BIN="$HOME/.cursor/skills/gstack/bin"
COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo none)
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)

"$BIN/gstack-review-log" "{\"skill\":\"plan-ceo-review\",\"timestamp\":\"$TIMESTAMP\",\"status\":\"issues_open\",\"unresolved\":2,\"critical_gaps\":1,\"mode\":\"SELECTIVE_EXPANSION\",\"via\":\"autoplan\",\"commit\":\"$COMMIT\"}"
"$BIN/gstack-review-log" "{\"skill\":\"plan-eng-review\",\"timestamp\":\"$TIMESTAMP\",\"status\":\"issues_open\",\"unresolved\":3,\"critical_gaps\":2,\"issues_found\":7,\"mode\":\"FULL_REVIEW\",\"via\":\"autoplan\",\"commit\":\"$COMMIT\"}"
"$BIN/gstack-review-log" "{\"skill\":\"plan-design-review\",\"timestamp\":\"$TIMESTAMP\",\"status\":\"issues_open\",\"unresolved\":2,\"via\":\"autoplan\",\"commit\":\"$COMMIT\"}"
"$BIN/gstack-review-log" "{\"skill\":\"plan-devex-review\",\"timestamp\":\"$TIMESTAMP\",\"status\":\"issues_open\",\"initial_score\":3,\"overall_score\":3,\"product_type\":\"skyline-plugin\",\"tthw_current\":\"20-45m\",\"tthw_target\":\"5m\",\"unresolved\":3,\"via\":\"autoplan\",\"commit\":\"$COMMIT\"}"
"$BIN/gstack-review-log" "{\"skill\":\"autoplan-voices\",\"timestamp\":\"$TIMESTAMP\",\"status\":\"issues_open\",\"source\":\"subagent-only\",\"phase\":\"ceo\",\"via\":\"autoplan\",\"consensus_confirmed\":0,\"consensus_disagree\":0,\"commit\":\"$COMMIT\"}"
"$BIN/gstack-review-log" "{\"skill\":\"autoplan-voices\",\"timestamp\":\"$TIMESTAMP\",\"status\":\"issues_open\",\"source\":\"subagent-only\",\"phase\":\"design\",\"via\":\"autoplan\",\"consensus_confirmed\":0,\"consensus_disagree\":0,\"commit\":\"$COMMIT\"}"
"$BIN/gstack-review-log" "{\"skill\":\"autoplan-voices\",\"timestamp\":\"$TIMESTAMP\",\"status\":\"issues_open\",\"source\":\"subagent-only\",\"phase\":\"dx\",\"via\":\"autoplan\",\"consensus_confirmed\":0,\"consensus_disagree\":0,\"commit\":\"$COMMIT\"}"
"$BIN/gstack-review-log" "{\"skill\":\"autoplan-voices\",\"timestamp\":\"$TIMESTAMP\",\"status\":\"issues_open\",\"source\":\"subagent-only\",\"phase\":\"eng\",\"via\":\"autoplan\",\"consensus_confirmed\":0,\"consensus_disagree\":0,\"commit\":\"$COMMIT\"}"

"$BIN/gstack-question-log" '{"skill":"autoplan","question_id":"autoplan-final-gate","question_summary":"Final approval gate for ssbu-tournament-mode plan","category":"approval","door_type":"two-way","options_count":5,"user_choice":"A","recommended":"A","session_id":"25364-1791475147-3e0506d9"}' 2>/dev/null || true

"$BIN/gstack-decision-log" --project ssbu-tournament-mode --decision "SSS packaging: require Midwest + companion delta patch" --rationale "Avoid dual .lc romfs conflict; user approved Final Gate A" --door two-way 2>/dev/null || true

echo "LOGS_OK commit=$COMMIT ts=$TIMESTAMP"
