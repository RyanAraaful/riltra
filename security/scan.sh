#!/usr/bin/env bash
# Secret / local-path scan entry point for this repository (pinned: gitleaks 8.30.1).
#
# Modes:
#   tree               scan the committed tree of HEAD
#   range <revrange>   scan commits in a git revision range (e.g. base..head)
#   staged             scan staged changes (pre-commit hook)
#   outgoing           scan commits being pushed, read from pre-push stdin (+ tip trees)
#   dir <path>         scan an arbitrary directory
#
# Environment:
#   GITLEAKS_BIN     scanner binary (default: gitleaks from PATH)
#   GITLEAKS_CONFIG  config file (default: <this script's repository>/.gitleaks.toml)
#
# Exit codes: 0 clean | 1 findings | 3 scanner unavailable | 4 usage/config error.
set -u
set -o pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
CONFIG="${GITLEAKS_CONFIG:-$HERE/../.gitleaks.toml}"
GITLEAKS="${GITLEAKS_BIN:-gitleaks}"

usage() {
  awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print } NR > 1 && !/^#/ { exit }' "$0"
  exit 4
}

if ! command -v "$GITLEAKS" >/dev/null 2>&1; then
  echo "scan: gitleaks not found: $GITLEAKS" >&2
  echo "scan: install pinned gitleaks 8.30.1 (see security/README.md) or set GITLEAKS_BIN." >&2
  exit 3
fi

if [ ! -f "$CONFIG" ]; then
  echo "scan: config not found: $CONFIG" >&2
  exit 4
fi

run_dir() { ( cd "$1" && "$GITLEAKS" dir --config "$CONFIG" --redact --no-banner . ); }
run_git() { "$GITLEAKS" git --config "$CONFIG" --redact --no-banner "$@" .; }

mode="${1:-}"
[ -n "$mode" ] || usage
shift

case "$mode" in
  tree)
    d="$(mktemp -d)"
    trap 'rm -rf "$d"' EXIT
    git archive HEAD | tar -x -C "$d" || { echo "scan: git archive HEAD failed" >&2; exit 4; }
    run_dir "$d"
    ;;
  range)
    [ $# -eq 1 ] || usage
    run_git --log-opts="$1"
    ;;
  staged)
    run_git --staged
    ;;
  outgoing)
    rc=0
    while read -r _lref lsha _rref rsha; do
      [ -n "${lsha:-}" ] || continue
      lsha="${lsha%$'\r'}"
      rsha="${rsha:-}"
      rsha="${rsha%$'\r'}"
      case "$lsha" in 0000000000000000000000000000000000000000) continue ;; esac
      if [ -z "$rsha" ] || [ "$rsha" = "0000000000000000000000000000000000000000" ]; then
        run_git --log-opts="$lsha --not --remotes" || rc=1
      else
        run_git --log-opts="$rsha..$lsha" || rc=1
      fi
      d="$(mktemp -d)"
      git archive "$lsha" | tar -x -C "$d" || { echo "scan: git archive $lsha failed" >&2; rc=1; }
      run_dir "$d" || rc=1
      rm -rf "$d"
    done
    exit "$rc"
    ;;
  dir)
    [ $# -eq 1 ] || usage
    run_dir "$1"
    ;;
  *)
    usage
    ;;
esac

rc=$?
exit "$rc"
