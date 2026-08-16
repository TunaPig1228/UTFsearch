#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
FEATURE_DIR="$ROOT/$(python -c "import json;print(json.load(open(r'$ROOT/.specify/feature.json'))['feature_directory'])" 2>/dev/null || sed -n 's/.*"feature_directory": "\(.*\)".*/\1/p' "$ROOT/.specify/feature.json")"
printf '{"FEATURE_DIR":"%s","FEATURE_SPEC":"%s/spec.md","IMPL_PLAN":"%s/plan.md","TASKS":"%s/tasks.md"}\n' "$FEATURE_DIR" "$FEATURE_DIR" "$FEATURE_DIR" "$FEATURE_DIR"
