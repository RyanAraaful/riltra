#!/usr/bin/env bash
# Regression tests for security/scan.sh and .gitleaks.toml.
# Usage: bash security/scan-tests.sh
# Requires: git, tar, and gitleaks 8.30.1 (via GITLEAKS_BIN or PATH).
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(git -C "$HERE" rev-parse --show-toplevel)"
SCAN="$HERE/scan.sh"
TD="$HERE/testdata"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
PASS=0
FAIL=0
# Synthetic trigger values are assembled at runtime so this script itself stays
# clean for the scanner it tests (the raw fixtures live in security/testdata).
SYN_P1="ghp_"
SYN_P2="aB3xK9qW2mZ7pL4vN8cR1sD6fG5hJ0kT2eYz"
TOKEN_VALUE="${SYN_P1}${SYN_P2}"
TEST_USER="alice"
ok() { PASS=$((PASS + 1)); echo "ok   - $1"; }
no() { FAIL=$((FAIL + 1)); echo "FAIL - $1"; }

case_dir() { # fixture -> fresh temp dir containing it
  d="$(mktemp -d "$TMP/case-XXXXXX")"
  mkdir -p "$d/fixture"
  cp "$1" "$d/fixture/"
  printf '%s' "$d"
}

expect_code() { # label expected_exit fixture
  label="$1"; want="$2"; fixture="$3"
  d="$(case_dir "$fixture")"
  "$SCAN" dir "$d" >"$TMP/$label.out" 2>&1
  got=$?
  if [ "$got" -eq "$want" ]; then ok "$label"; else no "$label (want exit $want, got $got)"; fi
}

# 1-5: blocked fixtures must be detected
expect_code "win-user-path blocked" 1 "$TD/blocked/win-user-path.txt"
expect_code "win-user-path alt-slash blocked" 1 "$TD/blocked/win-user-path-alt-slash.txt"
expect_code "win-user-path json-escaped blocked" 1 "$TD/blocked/win-user-path-json.txt"
expect_code "unix-home-path blocked" 1 "$TD/blocked/unix-home-path.txt"
expect_code "synthetic token blocked" 1 "$TD/blocked/synthetic-token.txt"

# 6-8: allowed fixtures must pass
expect_code "relative path allowed" 0 "$TD/allowed/relative-path.txt"
expect_code "env placeholders allowed" 0 "$TD/allowed/env-placeholders.txt"
expect_code "harmless drive paths allowed" 0 "$TD/allowed/drive-paths.txt"

# 9: scanner output must not contain the secret value
d="$(case_dir "$TD/blocked/synthetic-token.txt")"
"$SCAN" dir "$d" >"$TMP/redact.out" 2>&1
got=$?
if [ "$got" -eq 1 ] && ! grep -qF "$TOKEN_VALUE" "$TMP/redact.out"; then
  ok "scanner output redacts secrets"
else
  no "scanner output redacts secrets (exit $got; see $TMP/redact.out)"
fi

# 10: missing scanner fails closed with exit 3
d="$(case_dir "$TD/allowed/relative-path.txt")"
GITLEAKS_BIN="$TMP/definitely-missing" "$SCAN" dir "$d" >"$TMP/missing.out" 2>&1
got=$?
if [ "$got" -eq 3 ]; then ok "missing scanner fails closed (exit 3)"; else no "missing scanner fails closed (want exit 3, got $got)"; fi

# 11: committed file matching .gitignore is still scanned
repo="$TMP/git-case"
mkdir -p "$repo"
git -C "$repo" init -q
printf 'secret-data.txt\n' >"$repo/.gitignore"
printf 'C:/Users/%s/leak.txt\n' "$TEST_USER" >"$repo/secret-data.txt"
git -C "$repo" add .gitignore secret-data.txt -f
git -C "$repo" -c user.name=scan-test -c user.email=scan-test@example.invalid -c commit.gpgsign=false commit -q -m "add tracked-but-ignored file"
( cd "$repo" && "$SCAN" tree ) >"$TMP/ignored.out" 2>&1
got=$?
if [ "$got" -eq 1 ]; then ok "committed-but-ignored file is scanned"; else no "committed-but-ignored file is scanned (want exit 1, got $got)"; fi

# 12: staged changes are scanned before commit
printf '/home/%s/staged-leak.txt\n' "$TEST_USER" >"$repo/staged-leak.txt"
git -C "$repo" add staged-leak.txt
( cd "$repo" && "$SCAN" staged ) >"$TMP/staged.out" 2>&1
got=$?
if [ "$got" -eq 1 ]; then ok "staged leak is blocked"; else no "staged leak is blocked (want exit 1, got $got)"; fi

# 13: this repository's committed tree is clean
( cd "$ROOT" && "$SCAN" tree ) >"$TMP/tree.out" 2>&1
got=$?
if [ "$got" -eq 0 ]; then ok "repository tree is clean"; else no "repository tree is clean (want exit 0, got $got; see $TMP/tree.out)"; fi

echo
echo "scan-tests: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
