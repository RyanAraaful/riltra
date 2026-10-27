# SPDX-License-Identifier: Apache-2.0
# Grades one run against the frozen expectations (PROCEDURE.md section 7). Prints one line per
# criterion, then "STATUS PASS" or "STATUS FAIL". Usage: grade.sh <A1|A2|A3|A4> <run dir> <expected dir>
# Version 2 (2026-10-26). Version 1 is kept as grade-v1.sh. Changes: sha256sum writes binary-mode
# names with a leading "*", which version 1 did not strip; the A4 source check reads the manifest
# diff per file (modified, removed, added) and confines the change to escape().
task=$1 run=$2 exp=$3
ans="$run/session/answer.txt"
fails=0
check() { if [ "$1" = ok ]; then echo "PASS $2"; else echo "FAIL $2"; fails=$((fails + 1)); fi; }
case "$task" in
  A1|A2)
    if sort "$ans" | diff - <(sort "$exp/$task.txt") > /dev/null; then check ok "exact-match"; else check no "exact-match"; fi ;;
  A3)
    first=$(head -n 1 "$ans")
    if [ "$first" = "$(head -n 1 "$exp/A3.txt")" ]; then check ok "value-line"
    elif [ "$first" = "value 64" ]; then check no "value-line (critical: stale value 64 stated as current)"
    else check no "value-line"; fi
    if diff <(tail -n +2 "$ans" | sort) <(tail -n +2 "$exp/A3.txt" | sort) > /dev/null; then check ok "reader-lines"
    else check no "reader-lines"; fi ;;
  A4)
    if [ "$(cat "$run/reviewer/test.exit")" = 0 ] && grep -qF "test result: ok. 4 passed; 0 failed" "$run/reviewer/test.log"
    then check ok "reviewer-exit-0-and-4-passed"; else check no "reviewer-exit-0-and-4-passed"; fi
    { read -r modified; read -r removed; read -r added; } < <(node -e '
      const lines = require("fs").readFileSync(process.argv[1], "utf8").split("\n").filter((l) => /^[<>] /.test(l));
      const sides = {};
      for (const l of lines) { const p = l.split(/\s+/)[2].replace(/^\*/, ""); (sides[p] ??= new Set()).add(l[0]); }
      const group = (f) => Object.keys(sides).filter((p) => f(sides[p])).sort().join(" ") || "-";
      console.log([group((s) => s.size === 2), group((s) => s.size === 1 && s.has("<")), group((s) => s.size === 1 && s.has(">"))].join("\n"));
    ' "$run/snapshot-changes.diff")
    if [ "$modified" = "./crates/cli/src/json.rs" ]; then check ok "modified-tracked-files-only-json.rs"
    else check no "modified-tracked-files-only-json.rs ($modified)"; fi
    if [ "$removed" = "-" ]; then check ok "no-removed-files"; else check no "no-removed-files ($removed)"; fi
    if [ "$added" = "-" ] || ! echo "$added" | tr ' ' '\n' | grep -q '^\./crates/'; then
      echo "NOTE non-source files added in workspace: $([ "$added" = "-" ] && echo 0 || echo "$added" | wc -w)"
      check ok "no-added-source-files"
    else check no "no-added-source-files ($added)"; fi
    if diff <(sed '/pub fn escape/,/^}/d' "$run/reviewer/pinned-json.rs") \
            <(sed '/pub fn escape/,/^}/d' "$run/snapshot/crates/cli/src/json.rs") > /dev/null
    then check ok "change-confined-to-escape"; else check no "change-confined-to-escape"; fi
    if diff <(sed -n '/#\[cfg(test)\]/,$p' "$run/reviewer/pinned-json.rs") \
            <(sed -n '/#\[cfg(test)\]/,$p' "$run/snapshot/crates/cli/src/json.rs") > /dev/null
    then check ok "test-code-unchanged"; else check no "test-code-unchanged"; fi ;;
esac
if [ "$fails" = 0 ]; then echo "STATUS PASS"; else echo "STATUS FAIL"; fi
