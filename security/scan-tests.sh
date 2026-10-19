#!/usr/bin/env bash
# Regression tests for security/scan.sh and .gitleaks.toml.
# Usage: bash security/scan-tests.sh
# Requires: git, tar, and gitleaks 8.30.1 (via GITLEAKS_BIN or PATH).
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(git -C "$HERE" rev-parse --show-toplevel)"
SCAN="$HERE/scan.sh"
TD="$HERE/testdata"
DEFAULT_BRANCH=master
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

# absolute workspace paths outside user profiles must be detected
expect_code "local drive path blocked" 1 "$TD/blocked/local-drive-path.txt"
expect_code "local drive path alt-slash blocked" 1 "$TD/blocked/local-drive-path-alt-slash.txt"
expect_code "local drive path json-escaped blocked" 1 "$TD/blocked/local-drive-path-json.txt"
expect_code "UNC path blocked" 1 "$TD/blocked/unc-path.txt"

# 6-8: allowed fixtures must pass
expect_code "relative path allowed" 0 "$TD/allowed/relative-path.txt"
expect_code "env placeholders allowed" 0 "$TD/allowed/env-placeholders.txt"
expect_code "system and runner paths allowed" 0 "$TD/allowed/drive-paths.txt"

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

# allowlists are per fixture file, not per directory: a new file under
# security/testdata is still scanned even though the known fixtures are allowed
SCANNER="${GITLEAKS_BIN:-gitleaks}"
( cd "$ROOT" && "$SCANNER" dir --config "$ROOT/.gitleaks.toml" --redact --no-banner security/testdata ) >"$TMP/testdata-scan.out" 2>&1
got=$?
if [ "$got" -eq 0 ]; then ok "known fixtures are allowlisted by exact path"; else no "known fixtures are allowlisted by exact path (want exit 0, got $got; see $TMP/testdata-scan.out)"; fi

unlisted="$TD/blocked/zz-unlisted-leak.txt"
printf 'C:/Users/%s/unlisted-leak.txt\n' "$TEST_USER" >"$unlisted"
( cd "$ROOT" && "$SCANNER" dir --config "$ROOT/.gitleaks.toml" --redact --no-banner security/testdata ) >"$TMP/unlisted.out" 2>&1
got=$?
rm -f "$unlisted"
if [ "$got" -eq 1 ]; then ok "unlisted file under security/testdata is detected"; else no "unlisted file under security/testdata is detected (want exit 1, got $got; see $TMP/unlisted.out)"; fi

# push (outgoing) and range modes detect leaks in commit history
pushrepo="$TMP/push-case"
mkdir -p "$pushrepo"
git -C "$pushrepo" init -q
printf 'clean\n' >"$pushrepo/README.md"
git -C "$pushrepo" add README.md
git -C "$pushrepo" -c user.name=scan-test -c user.email=scan-test@example.invalid -c commit.gpgsign=false commit -q -m "clean base"
base=$(git -C "$pushrepo" rev-parse HEAD)
cp "$TD/blocked/local-drive-path.txt" "$pushrepo/push-leak.txt"
git -C "$pushrepo" add push-leak.txt
git -C "$pushrepo" -c user.name=scan-test -c user.email=scan-test@example.invalid -c commit.gpgsign=false commit -q -m "leak"
leak=$(git -C "$pushrepo" rev-parse HEAD)

printf 'refs/heads/main %s refs/heads/main %s\n' "$base" "0000000000000000000000000000000000000000" | ( cd "$pushrepo" && "$SCAN" outgoing ) >"$TMP/outgoing-clean.out" 2>&1
got=$?
if [ "$got" -eq 0 ]; then ok "outgoing clean commits pass"; else no "outgoing clean commits pass (want exit 0, got $got)"; fi

printf 'refs/heads/main %s refs/heads/main %s\n' "$leak" "0000000000000000000000000000000000000000" | ( cd "$pushrepo" && "$SCAN" outgoing ) >"$TMP/outgoing-leak.out" 2>&1
got=$?
if [ "$got" -eq 1 ]; then ok "outgoing leak is blocked"; else no "outgoing leak is blocked (want exit 1, got $got)"; fi

( cd "$pushrepo" && "$SCAN" range "$base..$leak" ) >"$TMP/range-leak.out" 2>&1
got=$?
if [ "$got" -eq 1 ]; then ok "range leak is blocked"; else no "range leak is blocked (want exit 1, got $got)"; fi

( cd "$pushrepo" && "$SCAN" range "$leak..$leak" ) >"$TMP/range-empty.out" 2>&1
got=$?
if [ "$got" -eq 0 ]; then ok "empty range passes"; else no "empty range passes (want exit 0, got $got)"; fi

# 13: this repository's committed tree is clean
( cd "$ROOT" && "$SCAN" tree ) >"$TMP/tree.out" 2>&1
got=$?
if [ "$got" -eq 0 ]; then ok "repository tree is clean"; else no "repository tree is clean (want exit 0, got $got; see $TMP/tree.out)"; fi

# New-branch pushes scan only commits introduced after the merge-base with the
# default branch; an unavailable default ref or merge-base fails closed.
nbrepo="$TMP/newbranch-case"
mkdir -p "$nbrepo"
git -C "$nbrepo" init -q
printf 'clean\n' >"$nbrepo/README.md"
git -C "$nbrepo" add README.md
git -C "$nbrepo" -c user.name=scan-test -c user.email=scan-test@example.invalid -c commit.gpgsign=false commit -q -m "clean base"
cp "$TD/blocked/local-drive-path.txt" "$nbrepo/default-leak.txt"
git -C "$nbrepo" add default-leak.txt
git -C "$nbrepo" -c user.name=scan-test -c user.email=scan-test@example.invalid -c commit.gpgsign=false commit -q -m "historical leak on default branch"
git -C "$nbrepo" update-ref refs/remotes/origin/main "$(git -C "$nbrepo" rev-parse HEAD)"
git -C "$nbrepo" checkout -q -b feature
printf 'clean\n' >"$nbrepo/feature.txt"
git -C "$nbrepo" add feature.txt
git -C "$nbrepo" -c user.name=scan-test -c user.email=scan-test@example.invalid -c commit.gpgsign=false commit -q -m "clean feature commit"

( cd "$nbrepo" && "$SCAN" new-branch main ) >"$TMP/newbranch-clean.out" 2>&1
got=$?
if [ "$got" -eq 0 ]; then ok "new branch passes when the only leak predates the merge-base"; else no "new branch passes when the only leak predates the merge-base (want exit 0, got $got; see $TMP/newbranch-clean.out)"; fi

cp "$TD/blocked/local-drive-path.txt" "$nbrepo/own-leak.txt"
git -C "$nbrepo" add own-leak.txt
git -C "$nbrepo" -c user.name=scan-test -c user.email=scan-test@example.invalid -c commit.gpgsign=false commit -q -m "introduce leak"
( cd "$nbrepo" && "$SCAN" new-branch main ) >"$TMP/newbranch-leak.out" 2>&1
got=$?
if [ "$got" -eq 1 ]; then ok "new branch blocks a leak it introduces"; else no "new branch blocks a leak it introduces (want exit 1, got $got)"; fi

noref="$TMP/newbranch-noref"
mkdir -p "$noref"
git -C "$noref" init -q
printf 'clean\n' >"$noref/README.md"
git -C "$noref" add README.md
git -C "$noref" -c user.name=scan-test -c user.email=scan-test@example.invalid -c commit.gpgsign=false commit -q -m "clean base"
( cd "$noref" && "$SCAN" new-branch main ) >"$TMP/newbranch-noref.out" 2>&1
got=$?
if [ "$got" -eq 4 ] && grep -q "fail-closed" "$TMP/newbranch-noref.out"; then ok "new branch without the default ref fails closed"; else no "new branch without the default ref fails closed (want exit 4 with fail-closed message, got $got; see $TMP/newbranch-noref.out)"; fi

first_branch="$(git -C "$noref" symbolic-ref --short HEAD)"
git -C "$noref" checkout -q --orphan unrelated
printf 'other\n' >"$noref/other.txt"
git -C "$noref" add other.txt
git -C "$noref" -c user.name=scan-test -c user.email=scan-test@example.invalid -c commit.gpgsign=false commit -q -m "unrelated root"
git -C "$noref" update-ref refs/remotes/origin/main "$(git -C "$noref" rev-parse HEAD)"
git -C "$noref" checkout -q "$first_branch"
( cd "$noref" && "$SCAN" new-branch main ) >"$TMP/newbranch-unrelated.out" 2>&1
got=$?
if [ "$got" -eq 4 ] && grep -q "merge-base" "$TMP/newbranch-unrelated.out"; then ok "new branch without a merge-base fails closed"; else no "new branch without a merge-base fails closed (want exit 4 with merge-base message, got $got; see $TMP/newbranch-unrelated.out)"; fi

# Fingerprint ignores are exact: the synthetic historical finding is ignored,
# while similar content or a new path is still detected.
ignrepo="$TMP/ignore-case"
mkdir -p "$ignrepo/security/testdata/allowed"
git -C "$ignrepo" init -q
printf 'clean\n' >"$ignrepo/README.md"
git -C "$ignrepo" add README.md
git -C "$ignrepo" -c user.name=scan-test -c user.email=scan-test@example.invalid -c commit.gpgsign=false commit -q -m "clean base"
ignbase="$(git -C "$ignrepo" rev-parse HEAD)"
cp "$TD/blocked/local-drive-path.txt" "$ignrepo/security/testdata/allowed/drive-paths.txt"
git -C "$ignrepo" add security/testdata/allowed/drive-paths.txt
git -C "$ignrepo" -c user.name=scan-test -c user.email=scan-test@example.invalid -c commit.gpgsign=false commit -q -m "historical synthetic finding"
hist="$(git -C "$ignrepo" rev-parse HEAD)"
( cd "$ignrepo" && "$SCAN" range "$ignbase..$hist" ) >"$TMP/ignore-red.out" 2>&1
got=$?
if [ "$got" -eq 1 ]; then ok "historical synthetic finding fails without an ignore fingerprint"; else no "historical synthetic finding fails without an ignore fingerprint (want exit 1, got $got)"; fi

printf '%s\n' "$hist:security/testdata/allowed/drive-paths.txt:local-absolute-path:2" >"$ignrepo/.gitleaksignore"
( cd "$ignrepo" && "$SCAN" range "$ignbase..$hist" ) >"$TMP/ignore-green.out" 2>&1
got=$?
if [ "$got" -eq 0 ]; then ok "exact fingerprint ignores the historical synthetic finding"; else no "exact fingerprint ignores the historical synthetic finding (want exit 0, got $got; see $TMP/ignore-green.out)"; fi

cp "$TD/blocked/local-drive-path.txt" "$ignrepo/security/testdata/allowed/drive-paths-copy.txt"
git -C "$ignrepo" add security/testdata/allowed/drive-paths-copy.txt
git -C "$ignrepo" -c user.name=scan-test -c user.email=scan-test@example.invalid -c commit.gpgsign=false commit -q -m "same content, new path"
( cd "$ignrepo" && "$SCAN" range "$ignbase..HEAD" ) >"$TMP/ignore-path.out" 2>&1
got=$?
if [ "$got" -eq 1 ]; then ok "similar content at a new path stays blocked"; else no "similar content at a new path stays blocked (want exit 1, got $got)"; fi

git -C "$ignrepo" rm -q security/testdata/allowed/drive-paths-copy.txt
cp "$TD/blocked/local-drive-path-alt-slash.txt" "$ignrepo/security/testdata/allowed/drive-paths.txt"
git -C "$ignrepo" add security/testdata/allowed/drive-paths.txt
git -C "$ignrepo" -c user.name=scan-test -c user.email=scan-test@example.invalid -c commit.gpgsign=false commit -q -m "new content in the ignored file"
( cd "$ignrepo" && "$SCAN" range "$ignbase..HEAD" ) >"$TMP/ignore-content.out" 2>&1
got=$?
if [ "$got" -eq 1 ]; then ok "new content in the ignored file stays blocked"; else no "new content in the ignored file stays blocked (want exit 1, got $got)"; fi

# The two historical synthetic findings pinned in .gitleaksignore must not fail
# the real branch range scan.
( cd "$ROOT" && "$SCAN" new-branch "$DEFAULT_BRANCH" ) >"$TMP/historical-range.out" 2>&1
got=$?
if [ "$got" -eq 0 ]; then ok "documented historical findings do not fail the new-branch scan"; else no "documented historical findings do not fail the new-branch scan (want exit 0, got $got; see $TMP/historical-range.out)"; fi

echo
echo "scan-tests: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
