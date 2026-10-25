#!/usr/bin/env bash
# G-KNW evidence run (M9 Task 4): seed the M7 fixture shape through the CLI,
# then export, backup, verify the backup, simulate store loss, restore, compare
# exports byte for byte, and check that reads change nothing (CLI and MCP).
#
# Every riltra call uses an isolated RILTRA_HOME inside WORK, an empty scratch
# directory outside the repository. The script prints no absolute paths and
# exits 0 only when every check passes.
#
#   WORK=<empty-scratch-dir> bash docs/pilots/m9-gknw/method/g-knw.sh [riltra-binary]
#
# Run from the repository root. Default binary: target/debug/riltra.exe.

set -u

BIN=${1:-target/debug/riltra.exe}
if [ -z "${WORK:-}" ]; then echo "set WORK to an empty scratch directory"; exit 2; fi
if [ -n "$(ls -A "$WORK" 2>/dev/null)" ]; then echo "WORK must be empty"; exit 2; fi
if [ ! -x "$BIN" ]; then echo "riltra binary not executable: $BIN"; exit 2; fi

HOME_MAIN="$WORK/home"
HOME_VERIFY="$WORK/home-verify"
REPO="$WORK/repo"
EV="$WORK/evidence"
mkdir -p "$HOME_MAIN" "$HOME_VERIFY" "$REPO" "$EV" || exit 2
printf 'alpha beta gamma' > "$REPO/a.py"

# Windows-native paths for the riltra binary: Git Bash does not convert a path
# carried inside an environment value, so RILTRA_HOME and file flags use cygpath.
native() {
  if command -v cygpath >/dev/null 2>&1; then cygpath -w "$1"; else printf '%s' "$1"; fi
}
NHOME=""
riltra() { RILTRA_HOME="$NHOME" "$BIN" "$@"; }

PASS=0
FAIL=0
pass() { PASS=$((PASS + 1)); echo "PASS $1"; }
fail() { FAIL=$((FAIL + 1)); echo "FAIL $1"; }
check() { local label=$1; shift; if "$@"; then pass "$label"; else fail "$label"; fi; }
has() { grep -qF -- "$2" "$1"; }
field() { grep -o "\"$2\":[^,}]*" "$1" | head -1 | cut -d: -f2; }
sha() { sha256sum "$1" | cut -d' ' -f1; }

# step <label> <stdout-file> <riltra args...>: expect exit 0.
step() {
  local label=$1 out=$2
  shift 2
  riltra "$@" >"$out" 2>"$out.err"
  local rc=$?
  if [ "$rc" -eq 0 ]; then pass "$label"; else fail "$label (exit $rc)"; fi
  return "$rc"
}

# stepj <label> <stdout-file> <json> <riltra args...>: JSON on stdin, expect exit 0.
stepj() {
  local label=$1 out=$2 json=$3
  shift 3
  printf '%s' "$json" | riltra "$@" >"$out" 2>"$out.err"
  local rc=$?
  if [ "$rc" -eq 0 ]; then pass "$label"; else fail "$label (exit $rc)"; fi
  return "$rc"
}

# export_to <file> [format flags]: the one export procedure used everywhere.
export_to() {
  local out=$1
  shift
  step "export $(basename "$out")" "$out.envelope" \
    knowledge export --repo krepo "$@" --out "$(native "$out")"
}

# --- seed (mirrors the M7 fixture `port_seed` in crates/cli/tests/knowledge.rs)
NHOME=$(native "$HOME_MAIN")
NREPO=$(native "$REPO")
step "init repository krepo" "$EV/init.out" init --repo "$NREPO" --name krepo

stepj "create d1 with evidence" "$EV/create-d1.out" \
  '{"id":"d1","kind":"decision","title":"First","body":"body one","status":"active","author":"a","evidence":[{"path":"a.py","byte_start":0,"byte_end":5}]}' \
  knowledge create --repo krepo --input -
stepj "create d2" "$EV/create-d2.out" \
  '{"id":"d2","kind":"decision","title":"Second","body":"body two","status":"active","author":"a","evidence":[]}' \
  knowledge create --repo krepo --input -
stepj "create d3" "$EV/create-d3.out" \
  '{"id":"d3","kind":"decision","title":"Third","body":"body three","status":"active","author":"a","evidence":[]}' \
  knowledge create --repo krepo --input -
stepj "create b1 benchmark with measurement" "$EV/create-b1.out" \
  '{"id":"b1","kind":"benchmark","title":"Perf","body":"body bench","status":"active","author":"a","evidence":[],"measurement":{"method":"m","environment":"e","sample_count":3,"metrics":{"latency":{"value":1.5,"unit":"ms"},"p99":{"value":42.0,"unit":"ms"}}}}' \
  knowledge create --repo krepo --input -
stepj "supersede d1 by d2 (revision 1 to 2)" "$EV/update-d1.out" \
  '{"title":"First v2","body":"First v2 body zqxbody","status":"superseded","author":"a","evidence":[],"superseded_by":{"id":"d2","revision":1},"reason":"replaced by d2"}' \
  knowledge update --repo krepo --id d1 --expected-revision 1 --input -
stepj "confirm d2 (revision 1 to 2)" "$EV/update-d2.out" \
  '{"title":"Second v2","body":"body two v2","status":"active","author":"a","evidence":[],"confirm_state":"confirmed"}' \
  knowledge update --repo krepo --id d2 --expected-revision 1 --input -
stepj "retract d3 (revision 1 to 2)" "$EV/update-d3.out" \
  '{"title":"Third","body":"Third body zqxbody","status":"retracted","author":"a","evidence":[],"reason":"was wrong"}' \
  knowledge update --repo krepo --id d3 --expected-revision 1 --input -
stepj "link r1 (confirmed)" "$EV/link-r1.out" \
  '{"relation_id":"r1","src_id":"d2","src_revision":2,"dst_id":"d1","dst_revision":2,"kind":"supports","author":"a","rationale":"the reason","confirm_state":"confirmed"}' \
  knowledge link --repo krepo --input -
stepj "link r2" "$EV/link-r2.out" \
  '{"relation_id":"r2","src_id":"d3","src_revision":2,"dst_id":"d1","dst_revision":2,"kind":"relates_to","author":"a","rationale":"maybe"}' \
  knowledge link --repo krepo --input -
stepj "unlink r2" "$EV/unlink-r2.out" \
  '{"author":"a","reason":"mistake"}' \
  knowledge unlink --repo krepo --relation r2 --input -

step "verify live store after seed" "$EV/verify-seed.out" knowledge verify --repo krepo
check "seed holds 4 records and 3 relations" has "$EV/verify-seed.out" '"records":4,"relations":3'

# --- initial export: twice, to show determinism
export_to "$EV/export-initial.jsonl"
export_to "$EV/export-initial-again.jsonl"
check "repeated JSONL export is byte-identical" \
  cmp -s "$EV/export-initial.jsonl" "$EV/export-initial-again.jsonl"
export_to "$EV/export-initial.md" --format md
export_to "$EV/export-initial-again.md" --format md
check "repeated Markdown export is byte-identical" \
  cmp -s "$EV/export-initial.md" "$EV/export-initial-again.md"
GEN=$(field "$EV/export-initial.jsonl.envelope" generation)

# --- backup
H_LIVE_PRE_BACKUP=$(sha "$HOME_MAIN/knowledge/krepo.db")
step "backup live store" "$EV/backup.out" knowledge backup --repo krepo --out "$(native "$EV/bk")"
check "backup writes snapshot and manifest" test -f "$EV/bk.db" -a -f "$EV/bk.manifest.json"
check "snapshot has no WAL or SHM companions" test ! -e "$EV/bk.db-wal" -a ! -e "$EV/bk.db-shm"
check "manifest binds repo krepo" has "$EV/bk.manifest.json" '"repo":"krepo"'
check "manifest records schema 2 and 4 records, 3 relations" \
  has "$EV/bk.manifest.json" '"schema_version":2,"records":4,"relations":3'
check "backup generation equals export generation" \
  [ "$(field "$EV/backup.out" generation)" = "$GEN" ]
check "backup leaves live store bytes unchanged" \
  [ "$(sha "$HOME_MAIN/knowledge/krepo.db")" = "$H_LIVE_PRE_BACKUP" ]

# --- verify the backup before the live store is touched: restore it into a
# separate verification home (restore runs the structural check on the staged
# snapshot), then verify and export that home.
NHOME=$(native "$HOME_VERIFY")
step "verification home: init same repository root" "$EV/verify-init.out" \
  init --repo "$NREPO" --name krepo
step "verification home: restore backup" "$EV/verify-restore.out" \
  knowledge restore --repo krepo --file "$(native "$EV/bk")"
check "restore into empty home reports overwrote false" has "$EV/verify-restore.out" '"overwrote":false'
check "restore reports 4 records and 3 relations" has "$EV/verify-restore.out" '"records":4,"relations":3'
step "verification home: verify restored snapshot" "$EV/verify-backup.out" \
  knowledge verify --repo krepo
check "restored snapshot verifies ok" has "$EV/verify-backup.out" '"status":"ok"'
export_to "$EV/export-verify-home.jsonl"
check "snapshot export is byte-identical to initial export" \
  cmp -s "$EV/export-initial.jsonl" "$EV/export-verify-home.jsonl"

if command -v sqlite3 >/dev/null 2>&1; then
  cp "$EV/bk.db" "$WORK/bk-copy.db"
  sqlite3 "$(native "$WORK/bk-copy.db")" 'PRAGMA integrity_check; PRAGMA user_version;' \
    | tr -d '\r' >"$EV/sqlite3-check.out" 2>&1
  check "independent sqlite3 integrity_check is ok" \
    [ "$(sed -n 1p "$EV/sqlite3-check.out")" = ok ]
  check "independent sqlite3 reads user_version 2" \
    [ "$(sed -n 2p "$EV/sqlite3-check.out")" = 2 ]
else
  echo "SKIP independent sqlite3 check: sqlite3 not on PATH"
fi

# --- simulate store loss in the main home
NHOME=$(native "$HOME_MAIN")
DB="$HOME_MAIN/knowledge/krepo.db"
H_STORE_PRE_LOSS=$(sha "$DB")
for s in -wal -shm -journal; do
  if [ -e "$DB$s" ]; then echo "info: companion present before loss: krepo.db$s, $(wc -c <"$DB$s" | tr -d ' ') bytes"; fi
done
case "$DB" in "$WORK"/*) ;; *) echo "refusing to delete outside WORK"; exit 1 ;; esac
rm -f "$DB" "$DB-wal" "$DB-shm" "$DB-journal"
check "store and companions removed (simulated loss)" \
  test ! -e "$DB" -a ! -e "$DB-wal" -a ! -e "$DB-shm" -a ! -e "$DB-journal"
step "export with store absent" "$EV/export-lost.out" \
  knowledge export --repo krepo --out "$(native "$EV/export-lost.jsonl")"
check "absent store exports generation 0" has "$EV/export-lost.out" '"generation":0'
step "search with store absent" "$EV/search-lost.out" \
  knowledge search --repo krepo --query zqxbody --include-superseded --include-retracted
check "absent store search returns no hits" has "$EV/search-lost.out" '"total":0'
check "absent-store reads create nothing" test ! -e "$DB"

# --- restore from the backup
step "restore into main home" "$EV/restore.out" \
  knowledge restore --repo krepo --file "$(native "$EV/bk")"
check "restore reports overwrote false" has "$EV/restore.out" '"overwrote":false'
check "restore reports 4 records and 3 relations" has "$EV/restore.out" '"records":4,"relations":3'
check "restore generation equals backup generation" \
  [ "$(field "$EV/restore.out" generation)" = "$GEN" ]
check "restored store file exists" test -f "$DB"

# --- export after restore, same procedure, byte comparison
export_to "$EV/export-restored.jsonl"
export_to "$EV/export-restored.md" --format md
check "JSONL export after restore is byte-identical to initial" \
  cmp -s "$EV/export-initial.jsonl" "$EV/export-restored.jsonl"
check "Markdown export after restore is byte-identical to initial" \
  cmp -s "$EV/export-initial.md" "$EV/export-restored.md"
check "restored generation equals initial generation" \
  [ "$(field "$EV/export-restored.jsonl.envelope" generation)" = "$GEN" ]
step "verify restored main store" "$EV/verify-restored.out" knowledge verify --repo krepo
check "restored main store verifies ok" has "$EV/verify-restored.out" '"status":"ok"'

# --- read-only checks: CLI reads, then MCP reads; store file bytes must not change
H_RO0=$(sha "$DB")
step "read-only: verify before reads" "$EV/ro-verify-0.out" knowledge verify --repo krepo
G_RO0=$(field "$EV/ro-verify-0.out" generation)
step "read-only: show d1" "$EV/ro-show-d1.out" knowledge show --repo krepo --id d1
step "read-only: show d1 revision 1" "$EV/ro-show-d1-r1.out" \
  knowledge show --repo krepo --id d1 --revision 1
step "read-only: history d1" "$EV/ro-history-d1.out" knowledge history --repo krepo --id d1
step "read-only: search" "$EV/ro-search.out" \
  knowledge search --repo krepo --query zqxbody --include-superseded --include-retracted
step "read-only: related d2 outbound" "$EV/ro-related-d2.out" \
  knowledge related --repo krepo --id d2 --direction outbound
step "read-only: related d1 inbound, retracted included" "$EV/ro-related-d1.out" \
  knowledge related --repo krepo --id d1 --direction inbound --include-retracted
export_to "$EV/ro-export.jsonl"
step "read-only: verify after CLI reads" "$EV/ro-verify-1.out" knowledge verify --repo krepo
check "CLI reads leave store file bytes unchanged" [ "$(sha "$DB")" = "$H_RO0" ]
check "CLI reads leave generation unchanged" \
  [ "$(field "$EV/ro-verify-1.out" generation)" = "$G_RO0" ]
check "export after CLI reads is byte-identical" \
  cmp -s "$EV/export-initial.jsonl" "$EV/ro-export.jsonl"

cat >"$EV/mcp-requests.jsonl" <<'EOF'
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"g-knw","version":"0.0.0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"riltra_knowledge_show","arguments":{"repo":"krepo","id":"d1"}}}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"riltra_knowledge_search","arguments":{"repo":"krepo","query":"zqxbody","include_superseded":true,"include_retracted":true}}}
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"riltra_knowledge_related","arguments":{"repo":"krepo","id":"d1","direction":"inbound","include_retracted":true}}}
EOF
riltra mcp <"$EV/mcp-requests.jsonl" >"$EV/mcp-reads.out" 2>"$EV/mcp-reads.err"
rc=$?
check "MCP read session exits 0 at stdin EOF" [ "$rc" -eq 0 ]
check "MCP answers each of the 4 requests once" \
  [ "$(grep -c '^{"jsonrpc":"2.0","id":' "$EV/mcp-reads.out")" -eq 4 ]
check "MCP reports no tool error" [ "$(grep -c '"isError":true' "$EV/mcp-reads.out")" -eq 0 ]
check "MCP show sees the supersede lineage" has "$EV/mcp-reads.out" 'superseded_by'
check "MCP reads leave store file bytes unchanged" [ "$(sha "$DB")" = "$H_RO0" ]
for s in -wal -shm; do
  if [ -e "$DB$s" ]; then echo "info: sidecar present after reads: krepo.db$s (SQLite WAL behaviour, not a store change)"; fi
done

# --- final check and hashes (hashes only, no paths)
step "final verify" "$EV/verify-final.out" knowledge verify --repo krepo
check "final store verifies ok with 4 records and 3 relations" \
  has "$EV/verify-final.out" '"records":4,"relations":3'
echo "hash export-initial.jsonl $(sha "$EV/export-initial.jsonl") bytes $(wc -c <"$EV/export-initial.jsonl" | tr -d ' ')"
echo "hash export-initial.md $(sha "$EV/export-initial.md") bytes $(wc -c <"$EV/export-initial.md" | tr -d ' ')"
echo "hash bk.db $(sha "$EV/bk.db")"
echo "hash store before loss $H_STORE_PRE_LOSS"
echo "hash store after restore and reads $H_RO0"
echo "generation $GEN"
echo "checks: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
