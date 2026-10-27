# SPDX-License-Identifier: Apache-2.0
# One pilot run, following PROCEDURE.md: clean snapshot, frozen edit, Arm R index, Cargo steps,
# one isolated Claude Code session, reviewer test (A4), grading. A failure stops only this run.
# Usage (after sourcing env.sh): bash "$H/session.sh" <R|B> <A1|A2|A3|A4> <attempt>
H=$(dirname "$0")
. "$H/lib.sh"
ARM=$1 TASK=$2 ATT=$3
RUNS=${RUNS:-$ROOT/runs}
RUN="$RUNS/$ARM-$TASK-a$ATT"
SNAP="$RUN/snapshot"
[ -e "$RUN" ] && { echo "exists: $RUN (attempts are never overwritten)" >&2; exit 2; }
mkdir -p "$SNAP" "$RUN/cargo" "$RUN/index" "$RUN/session" "$RUN/reviewer"

# fail <cause>: stop this run, record UNAVAILABLE (infrastructure), return success to the caller.
fail() { echo "UNAVAILABLE infrastructure: $*" > "$RUN/status.txt"; echo "$ARM-$TASK-a$ATT UNAVAILABLE: $*"; exit 0; }
riltra_r() { RILTRA_HOME="$(cygpath -w "$RUN/riltra-home")" "$BIN/riltra.exe" "$@"; }
manifest() { (cd "$SNAP" && find . -type f | LC_ALL=C sort | xargs sha256sum); }

# index_r <label>: Arm R index of the snapshot as it is now. Times the index build only.
index_r() {
  mkdir -p "$RUN/riltra-home"
  riltra_r init --repo "$(cygpath -w "$SNAP")" --name m10 > "$RUN/index/init-$1.json" 2>&1 || fail "riltra init ($1)"
  local t0 t1 rc
  t0=$(date +%s.%N)
  riltra_r index --repo m10 > "$RUN/index/index-$1.json" 2> "$RUN/index/index-$1.err"; rc=$?
  t1=$(date +%s.%N)
  echo "exit=$rc seconds=$(awk "BEGIN{printf \"%.3f\", $t1-$t0}")" > "$RUN/index/index-$1.timing"
  [ "$rc" = 0 ] || fail "riltra index ($1) exit $rc"
  riltra_r index status --repo m10 > "$RUN/index/status-$1.json" 2>&1
}

# Setup: clean snapshot from the pinned commit (PROCEDURE.md section 1). No .git directory.
git -C "$REPO" archive "$COMMIT" | tar -x -C "$SNAP" || fail "git archive"
git -C "$REPO" show "$COMMIT:crates/cli/src/json.rs" > "$RUN/reviewer/pinned-json.rs" || fail "pinned json.rs"

# Arm R index order (PROCEDURE.md section 4): A1 and A2 on the unedited copy; A3 before its edit;
# A4 after its defect. Arm B has no index.
if [ "$ARM" = R ] && [ "$TASK" != A4 ]; then index_r initial; fi
case "$TASK" in
  A3) (cd "$SNAP" && git apply "$FREEZE/edits/a3-edit.patch") || fail "a3 patch" ;;
  A4) (cd "$SNAP" && git apply "$FREEZE/edits/a4-defect.patch") || fail "a4 patch" ;;
esac
if [ "$ARM" = R ] && [ "$TASK" = A4 ]; then index_r initial; fi
if [ "$ARM" = R ] && [ "$TASK" = A3 ]; then riltra_r index status --repo m10 > "$RUN/index/status-after-edit.json" 2>&1; fi

manifest > "$RUN/snapshot-start.sha256"
config_hash > "$RUN/config-before.sha256"

# Cargo steps before the session (PROCEDURE.md section 6; sections 3.4). A4 also runs the frozen test.
if [ "$TASK" = A4 ]; then
  (cd "$SNAP" && CARGO_TARGET_DIR="$CACHE" cargo clean -p riltra && CARGO_TARGET_DIR="$CACHE" cargo test --locked --bin riltra json::tests) > "$RUN/cargo/pre-test.log" 2>&1
  echo $? > "$RUN/cargo/pre-test.exit"
else
  (cd "$SNAP" && CARGO_TARGET_DIR="$CACHE" cargo clean -p riltra) > "$RUN/cargo/pre-clean.log" 2>&1
  echo $? > "$RUN/cargo/pre-clean.exit"
fi

{
  echo "arm=$ARM"; echo "task=$TASK"; echo "attempt=$ATT"; echo "commit=$COMMIT"
  echo "tree=$(git -C "$REPO" rev-parse "$COMMIT^{tree}")"
  echo "archive_sha256=$(git -C "$REPO" archive "$COMMIT" | sha256sum | cut -d' ' -f1)"
  echo "prompt_sha256=$(sha256sum "$FREEZE/prompts/$TASK.txt" | cut -d' ' -f1)"
  echo "expected_sha256=$(sha256sum "$FREEZE/expected/$TASK.txt" | cut -d' ' -f1)"
  case "$TASK" in A3) echo "edit_sha256=$(sha256sum "$FREEZE/edits/a3-edit.patch" | cut -d' ' -f1)" ;;
                      A4) echo "edit_sha256=$(sha256sum "$FREEZE/edits/a4-defect.patch" | cut -d' ' -f1)" ;; esac
  [ "$ARM" = R ] && echo "riltra_sha256=$(sha256sum "$BIN/riltra.exe" | cut -d' ' -f1)"
  echo "claude_version=$("$CLAUDE_BIN" --version)"
  echo "cargo=$(cargo --version)"; echo "rustc=$(rustc --version)"
  echo "cargo_target_dir=$CACHE"; echo "rustflags=${RUSTFLAGS:-}"
  echo "config_sha256=$(cat "$RUN/config-before.sha256")"
} > "$RUN/provenance.txt"

# The isolated session (time caps: 15 min for A1 to A3, 30 min for A4, PROCEDURE.md and gate 3.2).
SECS=900; [ "$TASK" = A4 ] && SECS=1800
START=$(date -u +%Y-%m-%dT%H:%M:%SZ); T0=$(date +%s)
RC=$(claude_session "$ARM" "$SNAP" "$RUN" "$FREEZE/prompts/$TASK.txt" "$SECS")
T1=$(date +%s); END=$(date -u +%Y-%m-%dT%H:%M:%SZ)
echo "start=$START end=$END exit=$RC task_seconds=$((T1 - T0)) cap_seconds=$SECS" > "$RUN/session/timing.txt"
node "$H/summarize.js" "$RUN/transcript.jsonl" "$RUN/session" > /dev/null 2>&1 || echo "summarize failed" >> "$RUN/session/timing.txt"

manifest > "$RUN/snapshot-end.sha256"
diff "$RUN/snapshot-start.sha256" "$RUN/snapshot-end.sha256" > "$RUN/snapshot-changes.diff"
config_hash > "$RUN/config-after.sha256"

# Reviewer (A4): the frozen command on the agent's final copy, after a Cargo clean (PROCEDURE.md section 6).
if [ "$TASK" = A4 ]; then
  T0=$(date +%s)
  (cd "$SNAP" && CARGO_TARGET_DIR="$CACHE" cargo clean -p riltra && CARGO_TARGET_DIR="$CACHE" cargo test --locked --bin riltra json::tests) > "$RUN/reviewer/test.log" 2>&1
  echo $? > "$RUN/reviewer/test.exit"
  echo "test_seconds=$(( $(date +%s) - T0 ))" > "$RUN/reviewer/timing.txt"
fi

# Status: client errors are UNAVAILABLE with their cause; a time cap is a FAIL; otherwise grade.
CLIENT_ERR=$(node -e '
  try { const r = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")).result;
        process.stdout.write(r && r.is_error ? (r.error_text || "client error") : ""); } catch { process.stdout.write("no result event"); }
' "$RUN/session/summary.json" 2>/dev/null)
if [ -n "$CLIENT_ERR" ]; then
  echo "UNAVAILABLE client error: $CLIENT_ERR" > "$RUN/status.txt"
elif [ "$RC" = 124 ]; then
  echo "FAIL time cap reached (${SECS}s)" > "$RUN/status.txt"
elif [ "$RC" != 0 ]; then
  echo "UNAVAILABLE client exit $RC" > "$RUN/status.txt"
else
  bash "$H/grade.sh" "$TASK" "$RUN" "$FREEZE/expected" > "$RUN/grade.txt"
  echo "$(tail -n 1 "$RUN/grade.txt" | sed 's/^STATUS //')" > "$RUN/status.txt"
fi
echo "$ARM-$TASK-a$ATT $(cat "$RUN/status.txt")"
