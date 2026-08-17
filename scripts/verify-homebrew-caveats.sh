#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <installation-log> <artwork>" >&2
  exit 2
fi

installation_log=$1
artwork=$2
first_art_line=$(head -n 1 "$artwork")
start_line=$(grep -n -F -m 1 "$first_art_line" "$installation_log" | cut -d: -f1 || true)

if [[ -z $start_line ]]; then
  echo "error: Hydra artwork was not found in Homebrew output" >&2
  exit 1
fi

art_lines=$(wc -l < "$artwork" | tr -d ' ')
end_line=$((start_line + art_lines - 1))
extracted=$(mktemp "${TMPDIR:-/tmp}/hydra-caveats.XXXXXX")
trap 'rm -f "$extracted"' EXIT
sed -n "${start_line},${end_line}p" "$installation_log" > "$extracted"
cmp "$artwork" "$extracted"

help_line=$(grep -n -F -m 1 "  hydra --help" "$installation_log" | cut -d: -f1 || true)
skill_line=$(grep -n -F -m 1 "  hydra skill install codex" "$installation_log" | cut -d: -f1 || true)
if [[ -z $help_line || -z $skill_line || $help_line -le $end_line || $skill_line -le $help_line ]]; then
  echo "error: Homebrew guidance is missing or ordered incorrectly" >&2
  exit 1
fi
