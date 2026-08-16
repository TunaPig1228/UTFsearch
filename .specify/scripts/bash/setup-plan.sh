#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
FEATURE_REL="$(sed -n 's/.*"feature_directory": "\(.*\)".*/\1/p' "$ROOT/.specify/feature.json")"
FEATURE_DIR="$ROOT/$FEATURE_REL"
HAS_GIT=false
[ -d "$ROOT/.git" ] && HAS_GIT=true
printf '{"FEATURE_SPEC":"%s/spec.md","IMPL_PLAN":"%s/plan.md","SPECS_DIR":"%s","BRANCH":"%s","HAS_GIT":%s}\n' \
  "$FEATURE_DIR" "$FEATURE_DIR" "$FEATURE_DIR" "$(basename "$FEATURE_REL")" "$HAS_GIT"
