# SPDX-License-Identifier: Apache-2.0
# Grades one run against the frozen expectations (PROCEDURE.md section 7). Prints one line per
# criterion, then "STATUS PASS" or "STATUS FAIL". Usage: grade.sh <A1|A2|A3|A4> <run dir> <expected dir>
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
    changed=$(grep -E '^[<>] ' "$run/snapshot-changes.diff" | awk '{print $3}' | sort -u | paste -sd' ' -)
    if [ "$changed" = "./crates/cli/src/json.rs" ]; then check ok "only-json.rs-changed"
    else check no "only-json.rs-changed ($changed)"; fi
    if diff <(sed -n '/#\[cfg(test)\]/,$p' "$run/reviewer/pinned-json.rs") \
            <(sed -n '/#\[cfg(test)\]/,$p' "$run/snapshot/crates/cli/src/json.rs") > /dev/null
    then check ok "test-code-unchanged"; else check no "test-code-unchanged"; fi ;;
esac
if [ "$fails" = 0 ]; then echo "STATUS PASS"; else echo "STATUS FAIL"; fi
