#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Re-derives the M10 frozen benchmark inputs from the pinned commit with git and
# text tools only: no Riltra binary and no agent output. Read-only; it writes only
# to temporary directories that it removes on exit.
#
# Usage, from the repository root:
#   bash docs/pilots/m10/freeze/verify/verify-freeze.sh
#   CARGO_TARGET_DIR=<dependency cache> M10_LOG_DIR=<log directory> \
#     bash docs/pilots/m10/freeze/verify/verify-freeze.sh --cargo
# CARGO_TARGET_DIR and M10_LOG_DIR must lie outside the repository.
#
# --cargo is the A4 preflight (PROCEDURE.md §5). Two independent source copies are
# run in sequence: P1 defective, P2 restored, P3 defective, P4 restored. Each step
# runs `cargo clean -p riltra` first. C1 is a control: restored, without the clean
# step, to show whether a stale riltra artifact is reused.
set -euo pipefail

commit=7f86891c0942cc1d7efd6ec29c6c0b6b1606e727
tree=b2bd50be9ec829fd122635696a2cb95c29fe2bcc
lock_sha=9b9ba4dc0d7e7ad6f653dbba99b859e6a3fb9030ee73a3aa9858ef4e544ef905
freeze=docs/pilots/m10/freeze
src=crates/cli/src
root=$(pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0

pass() { echo "ok   $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

if [ "$(git rev-parse "$commit^{tree}" 2>/dev/null)" = "$tree" ]; then pass "pinned tree"; else bad "pinned tree"; fi
if [ "$(git show "$commit:Cargo.lock" | sha256sum | cut -d' ' -f1)" = "$lock_sha" ]; then
  pass "pinned Cargo.lock"
else
  bad "pinned Cargo.lock"
fi

# A1: each span slices exactly to its name, and the line is that name's fn item.
while read -r name loc _ span; do
  path=${loc%:*}; line=${loc##*:}; start=${span%-*}; end=${span#*-}
  git show "$commit:$path" > "$tmp/blob"
  slice=$(head -c "$end" "$tmp/blob" | tail -c "$((end - start))")
  if [ "$slice" = "$name" ] && sed -n "${line}p" "$tmp/blob" | grep -qF "fn $name("; then
    pass "A1 $name $loc"
  else
    bad "A1 $name $loc"
  fi
done < "$freeze/expected/A1.txt"

# A2: call sites are the lines that call either name, excluding definitions.
derived=$(git grep -n -E '\b(canonical_root|resolve_recorded_root)\(' "$commit" -- "$src" \
  | grep -v -E 'fn (canonical_root|resolve_recorded_root)\(' \
  | awk -F: -v c="$commit" '{ path = $2; line = $3; text = $0
      sub("^" c ":[^:]*:[0-9]+:", "", text)
      name = (text ~ /canonical_root\(/) ? "canonical_root" : "resolve_recorded_root"
      print name, path ":" line }' \
  | sort)
if diff <(sort "$freeze/expected/A2.txt") <(printf '%s\n' "$derived") > /dev/null; then
  pass "A2 call sites ($(wc -l < "$freeze/expected/A2.txt") lines)"
else
  bad "A2 call sites differ from expected/A2.txt"
fi

# A3: readers are the lines that name the constant outside its definition.
readers=$(git grep -n -w MAX_JSON_DEPTH "$commit" -- "$src" | grep -v 'const MAX_JSON_DEPTH' \
  | sed -E "s|^$commit:([^:]+):([0-9]+):.*|reader \1:\2|" | sort)
if diff <(tail -n +2 "$freeze/expected/A3.txt" | sort) <(printf '%s\n' "$readers") > /dev/null \
  && [ "$(head -n 1 "$freeze/expected/A3.txt")" = "value 128" ]; then
  pass "A3 readers and post-edit value"
else
  bad "A3 readers or post-edit value"
fi

# Frozen edits apply to the pinned tree and produce the stated lines.
for p in a3-edit a4-defect; do
  mkdir -p "$tmp/$p"
  git archive "$commit" | tar -x -C "$tmp/$p"
  if (cd "$tmp/$p" && git apply "$root/$freeze/edits/$p.patch"); then pass "$p applies"; else bad "$p applies"; fi
done
if [ "$(sed -n '267p' "$tmp/a3-edit/$src/knowledge.rs")" = "const MAX_JSON_DEPTH: usize = 128;" ]; then
  pass "A3 edit line"
else
  bad "A3 edit line"
fi
if [ "$(git show "$commit:$src/json.rs" | grep -cF "'\\n' => out.push_str")" = 1 ] \
  && [ "$(grep -cF "'\\n' => out.push_str" "$tmp/a4-defect/$src/json.rs")" = 0 ]; then
  pass "A4 defect removes the newline arm"
else
  bad "A4 defect removes the newline arm"
fi
if git show "$commit:$src/json.rs" | sed -n '30p' | grep -qF "fn escapes_quote_backslash_and_newline()"; then
  pass "A4 test anchor, line 30"
else
  bad "A4 test anchor, line 30"
fi

# A4 preflight: one step per run, each in a source copy, run in sequence.
run_step() { # run_step <id> <defect|restored> <clean|noclean> <copy dir>
  local id=$1 state=$2 mode=$3 dir=$4 out code result failing compiled_here bin_path bin_sha start end
  if [ "$mode" = clean ]; then
    (cd "$dir" && cargo clean -p riltra) > "$M10_LOG_DIR/$id.clean.log" 2>&1
  fi
  start=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  set +e
  out=$(cd "$dir" && cargo test --locked --bin riltra json::tests 2>&1)
  code=$?
  set -e
  end=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  result=$(printf '%s\n' "$out" | grep -E '^test result:' || true)
  failing=$(printf '%s\n' "$out" | grep -E '\.\.\. FAILED$' || true)
  compiled_here=$(printf '%s\n' "$out" | grep -F "Compiling riltra v0.1.0 (" | grep -c -F "$(basename "$dir")" || true)
  bin_path=$(printf '%s\n' "$out" | sed -n 's/.*Running unittests src[^(]*(\(.*\))$/\1/p' | head -n 1)
  if command -v cygpath > /dev/null 2>&1 && [ -n "$bin_path" ]; then bin_path=$(cygpath -u "$bin_path"); fi
  bin_sha=$( [ -n "$bin_path" ] && sha256sum "$bin_path" 2>/dev/null | cut -d' ' -f1 || true )
  {
    echo "id=$id state=$state mode=$mode copy=$(basename "$dir") start=$start end=$end exit=$code"
    echo "CARGO_TARGET_DIR=$CARGO_TARGET_DIR RUSTFLAGS=${RUSTFLAGS:-}"
    cargo --version
    rustc --version
    echo "test binary: $bin_path sha256=$bin_sha"
    echo "--- output"
    printf '%s\n' "$out"
  } > "$M10_LOG_DIR/$id.log"
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$id" "$state" "$mode" "$code" "$compiled_here" "$bin_sha" "$result" \
    >> "$M10_LOG_DIR/summary.tsv"
  case "$id:$state:$code" in
    P1:defect:101|P3:defect:101)
      if [ "$compiled_here" -ge 1 ] && printf '%s' "$result" | grep -qF '3 passed; 1 failed' \
        && printf '%s' "$failing" | grep -qF 'json::tests::escapes_quote_backslash_and_newline'; then
        pass "$id defect: own build, exit 101, only the named test fails"
      else
        bad "$id defect: exit=$code compiled_here=$compiled_here"
      fi ;;
    P2:restored:0|P4:restored:0)
      if [ "$compiled_here" -ge 1 ] && printf '%s' "$result" | grep -qF '4 passed; 0 failed'; then
        pass "$id restored: own build, exit 0, 4 passed"
      else
        bad "$id restored: exit=$code compiled_here=$compiled_here"
      fi ;;
    C1:*)
      echo "info C1 control (restored, no clean): exit=$code compiled_here=$compiled_here result=$result" ;;
    *)
      bad "$id unexpected exit=$code" ;;
  esac
}

if [ "${1:-}" = "--cargo" ]; then
  : "${CARGO_TARGET_DIR:?set CARGO_TARGET_DIR to the dependency cache outside the repository}"
  : "${M10_LOG_DIR:?set M10_LOG_DIR to a log directory outside the repository}"
  case "$CARGO_TARGET_DIR$M10_LOG_DIR" in
    "$root"*) bad "CARGO_TARGET_DIR and M10_LOG_DIR must be outside the repository"; exit 1 ;;
  esac
  mkdir -p "$M10_LOG_DIR"
  printf 'id\tstate\tmode\texit\tcompiled_here\ttest_binary_sha256\tresult\n' > "$M10_LOG_DIR/summary.tsv"
  for copy in defect restored; do
    mkdir -p "$tmp/copy-$copy"
    git archive "$commit" | tar -x -C "$tmp/copy-$copy"
  done
  (cd "$tmp/copy-defect" && git apply "$root/$freeze/edits/a4-defect.patch")
  run_step P1 defect clean "$tmp/copy-defect"
  run_step P2 restored clean "$tmp/copy-restored"
  run_step P3 defect clean "$tmp/copy-defect"
  run_step C1 restored noclean "$tmp/copy-restored"
  run_step P4 restored clean "$tmp/copy-restored"
  sha_of() { awk -F'\t' -v id="$1" '$1 == id { print $6 }' "$M10_LOG_DIR/summary.tsv"; }
  if [ -n "$(sha_of P1)" ] && [ "$(sha_of P1)" != "$(sha_of P2)" ]; then
    pass "P1 and P2 test binaries differ"
  else
    bad "P1 and P2 test binaries differ"
  fi
  echo "info test binary SHA-256: P1=$(sha_of P1) P2=$(sha_of P2) P3=$(sha_of P3) C1=$(sha_of C1) P4=$(sha_of P4)"
  echo "info byte identity between runs is not asserted (incremental builds may differ); the proof is own-build evidence and the outcome"
  if [ -n "$(sha_of C1)" ] && [ "$(sha_of C1)" = "$(sha_of P3)" ]; then
    pass "control C1 reused the P3 defective binary (stale artifact without clean)"
  else
    echo "info control C1 did not reuse the P3 binary"
  fi
  echo "--- summary: $M10_LOG_DIR/summary.tsv"
  cat "$M10_LOG_DIR/summary.tsv"
fi

exit "$fail"
