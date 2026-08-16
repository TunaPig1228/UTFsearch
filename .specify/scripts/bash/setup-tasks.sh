#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
FEATURE_REL="$(sed -n 's/.*"feature_directory": "\(.*\)".*/\1/p' "$ROOT/.specify/feature.json")"
FEATURE_DIR="$ROOT/$FEATURE_REL"
printf '{"FEATURE_DIR":"%s","TASKS_TEMPLATE":"%s/.specify/templates/tasks-template.md","AVAILABLE_DOCS":["research.md","data-model.md","quickstart.md","plan.md","spec.md","contracts/"]}\n' \
  "$FEATURE_DIR" "$ROOT"
