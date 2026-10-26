# SPDX-License-Identifier: Apache-2.0
# Precheck only: login, profile cleanliness, and Arm R / Arm B tool access. Not a benchmark run.
# Usage (after sourcing env.sh): bash "$H/probe.sh"
H=$(dirname "$0")
. "$H/lib.sh"
P="$ROOT/precheck"
mkdir -p "$P/ws"
printf 'Reply with exactly: OK\n' > "$P/prompt-ok.txt"
echo "config-hash-before $(config_hash)"
echo "riltra-on-PATH arm-B: [$(env -i PATH="$PATH" bash -c 'command -v riltra')]"
echo "riltra-on-PATH arm-R: [$(env -i PATH="$BIN:$PATH" bash -c 'command -v riltra')]"
for arm in B R; do
  rc=$(claude_session "$arm" "$P/ws" "$P/probe-$arm" "$P/prompt-ok.txt" 300)
  echo "probe-$arm exit=$rc"
  node "$H/summarize.js" "$P/probe-$arm/transcript.jsonl" "$P/probe-$arm/session"
done
echo "config-hash-after $(config_hash)"
