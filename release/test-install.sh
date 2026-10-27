#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Phase A acceptance tests for release/install.sh.
#
# Usage: release/test-install.sh --archive <riltra-<version>-<platform>.tar.gz>
#
# The archive and its adjacent .sha256 must come from release/package.sh. The
# test uses a throwaway HOME, so nothing is installed for the invoking user.
set -euo pipefail

archive=
while [ $# -gt 0 ]; do
  case "$1" in
    --archive) [ $# -ge 2 ] || { echo "usage: $0 --archive <tar.gz>" >&2; exit 2; }; archive=$2; shift 2 ;;
    -h|--help) echo "usage: $0 --archive <tar.gz>"; exit 0 ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
done
[ -n "$archive" ] || { echo "usage: $0 --archive <tar.gz>" >&2; exit 2; }
archive="$(cd "$(dirname "$archive")" && pwd)/$(basename "$archive")"
[ -f "$archive" ] || { echo "test-install: archive not found: $archive" >&2; exit 2; }

here="$(cd "$(dirname "$0")" && pwd)"
installer="$here/install.sh"
[ -f "$installer" ] || { echo "test-install: installer not found: $installer" >&2; exit 2; }

work="$(mktemp -d)"
[ -n "$work" ] || { echo "test-install: cannot create a temporary directory" >&2; exit 2; }
trap 'rm -rf "$work"' EXIT HUP INT TERM
export HOME="$work/home"
mkdir -p "$HOME"
payload="$HOME/.local/share/riltra"
link="$HOME/.local/bin/riltra"

pass() { printf 'ok   %s\n' "$1"; }
fail() { printf 'FAIL %s\n' "$1" >&2; exit 1; }

hash_file() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | awk '{print $1}';
  else shasum -a 256 "$1" | awk '{print $1}'; fi
}

write_sha() {
  local file=$1
  ( cd "$(dirname "$file")" && { sha256sum "$(basename "$file")" 2>/dev/null || shasum -a 256 "$(basename "$file")"; } >"$file.sha256" )
}

run_install() { sh "$installer" --archive "$1" >"$work/install.log" 2>&1; }

expect_install_fail() { # expect_install_fail <name> <archive>
  if run_install "$2"; then
    cat "$work/install.log" >&2
    fail "$1 (installer exited 0)"
  fi
  pass "$1"
}

expect_unchanged() { # expect_unchanged <name> <expected hash>
  local now
  now="$(hash_file "$payload/riltra")"
  [ "$now" = "$2" ] || fail "$1 (installed binary changed)"
  pass "$1"
}

no_staging() {
  for entry in "$HOME/.local/share"/.riltra-*; do
    if [ -e "$entry" ] || [ -L "$entry" ]; then
      fail "staging directory left behind: $entry"
    fi
  done
}

# --- positive install --------------------------------------------------------
sh "$installer" --archive "$archive" >"$work/install.log" 2>&1 || { cat "$work/install.log" >&2; fail "positive install"; }
[ -x "$payload/riltra" ] || fail "payload binary"
[ -L "$link" ] || fail "user-visible symlink"
[ "$(readlink "$link")" = "$payload/riltra" ] || fail "symlink target"
for doc in LICENSE NOTICE THIRD_PARTY_LICENSES.txt sbom.cdx.json INSTALL.md; do
  [ -s "$payload/$doc" ] || fail "payload document $doc"
done
pass "positive install"
"$link" --version | grep -q '^riltra ' || fail "installed binary runs"
pass "installed binary runs"

# The shared bin directory holds only the symlink; notices stay in the payload.
bin_entries=0
for entry in "$HOME/.local/bin"/* "$HOME/.local/bin"/.[!.]* "$HOME/.local/bin"/..?*; do
  if [ -e "$entry" ] || [ -L "$entry" ]; then bin_entries=$((bin_entries + 1)); fi
done
[ "$bin_entries" = "1" ] || fail "shared bin directory contains more than the symlink"
pass "shared bin directory holds only the symlink"
if [ -n "$(find "$payload" -type f ! -user "$(id -u)" -print)" ]; then fail "payload has files not owned by the invoking user"; fi
pass "payload files are user-owned"
no_staging

before="$(hash_file "$payload/riltra")"
sh "$installer" --archive "$archive" >"$work/reinstall.log" 2>&1 || { cat "$work/reinstall.log" >&2; fail "reinstall"; }
[ "$(hash_file "$payload/riltra")" = "$before" ] || fail "reinstall changed the binary"
pass "reinstall"

# --- checksum failures preserve the installation -----------------------------
case_dir="$work/cases"
mkdir -p "$case_dir"
base="$(basename "$archive")"
mkcase() { # mkcase <name> -> path to a same-named archive copy with its checksum
  local dir="$case_dir/$1"
  mkdir -p "$dir"
  cp "$archive" "$dir/$base"
  cp "$archive.sha256" "$dir/$base.sha256"
  printf '%s\n' "$dir/$base"
}

mismatch="$(mkcase mismatch)"
printf x >>"$mismatch"
expect_install_fail "checksum mismatch rejected" "$mismatch"
expect_unchanged "checksum mismatch leaves the binary" "$before"
no_staging

missing="$(mkcase missing)"
rm "$missing.sha256"
expect_install_fail "missing checksum rejected" "$missing"
expect_unchanged "missing checksum leaves the binary" "$before"
no_staging

fresh_missing="$(mkcase missing-fresh)"
rm "$fresh_missing.sha256"
fresh_home="$work/home-fresh"
mkdir -p "$fresh_home"
if HOME="$fresh_home" sh "$installer" --archive "$fresh_missing" >"$work/fresh.log" 2>&1; then
  fail "missing checksum rejected in a fresh home"
fi
[ ! -e "$fresh_home/.local/share/riltra" ] && [ ! -e "$fresh_home/.local/bin/riltra" ] ||
  fail "missing checksum installed something in a fresh home"
pass "missing checksum installs nothing in a fresh home"

malformed="$(mkcase malformed)"
printf 'not-a-checksum  %s\n' "$base" >"$malformed.sha256"
expect_install_fail "malformed checksum rejected" "$malformed"
expect_unchanged "malformed checksum leaves the binary" "$before"
no_staging

# --- unsupported platforms ---------------------------------------------------
shim="$work/shim"
mkdir -p "$shim"
cat >"$shim/uname" <<'EOF'
#!/bin/sh
case "$1" in
  -s) echo FreeBSD ;;
  -m) echo x86_64 ;;
  *) echo unknown ;;
esac
EOF
chmod +x "$shim/uname"
if PATH="$shim:$PATH" run_install "$archive"; then fail "unsupported operating system rejected"; fi
grep -q 'supported: Linux x64, macOS Intel x64' "$work/install.log" || fail "unsupported operating system names the supported list"
pass "unsupported operating system rejected"
expect_unchanged "unsupported operating system leaves the binary" "$before"

cat >"$shim/uname" <<'EOF'
#!/bin/sh
case "$1" in
  -s) echo Linux ;;
  -m) echo aarch64 ;;
  *) echo unknown ;;
esac
EOF
if PATH="$shim:$PATH" run_install "$archive"; then fail "ARM64 rejected"; fi
grep -q 'unsupported platform Linux/aarch64' "$work/install.log" || fail "ARM64 names the detected platform"
pass "ARM64 rejected"
expect_unchanged "ARM64 leaves the binary" "$before"

# --- existing unrelated executable ------------------------------------------
rm "$link"
printf 'foreign\n' >"$link"
expect_install_fail "foreign executable is not replaced" "$archive"
[ "$(cat "$link")" = "foreign" ] || fail "foreign executable changed"
pass "foreign executable is unchanged"
rm "$link"
ln -s /bin/true "$link"
expect_install_fail "foreign symlink is not replaced" "$archive"
[ "$(readlink "$link")" = "/bin/true" ] || fail "foreign symlink changed"
pass "foreign symlink is unchanged"
rm "$link"
ln -s "$payload/riltra" "$link"

printf 'foreign\n' >"$payload/notes.txt"
expect_install_fail "foreign payload file blocks reinstall" "$archive"
expect_unchanged "foreign payload file leaves the binary" "$before"
[ -f "$payload/notes.txt" ] || fail "foreign payload file was removed"
pass "foreign payload file is unchanged"
rm "$payload/notes.txt"

# --- unsafe archives ---------------------------------------------------------
make_evil() { # make_evil <path> <python expression for member setup>
  python3 - "$1" <<'PY'
import io
import sys
import tarfile

path = sys.argv[1]
with tarfile.open(path, "w:gz") as tar:
    info = tarfile.TarInfo("../escape.txt")
    data = b"escape"
    info.size = len(data)
    tar.addfile(info, io.BytesIO(data))
PY
  write_sha "$1"
}
evil="$(mkcase evil)"
make_evil "$evil"
expect_install_fail "path traversal archive rejected" "$evil"
[ ! -e "$HOME/../escape.txt" ] && [ ! -e "$work/escape.txt" ] || fail "traversal archive wrote outside staging"
pass "traversal archive wrote nothing outside staging"

absolute="$(mkcase absolute)"
python3 - "$absolute" <<'PY'
import io
import sys
import tarfile

with tarfile.open(sys.argv[1], "w:gz") as tar:
    info = tarfile.TarInfo("/tmp/riltra-escape")
    data = b"escape"
    info.size = len(data)
    tar.addfile(info, io.BytesIO(data))
PY
write_sha "$absolute"
expect_install_fail "absolute archive entry rejected" "$absolute"

symlink_archive="$(mkcase symlink)"
python3 - "$symlink_archive" <<'PY'
import sys
import tarfile

with tarfile.open(sys.argv[1], "w:gz") as tar:
    info = tarfile.TarInfo("riltra-evil/link")
    info.type = tarfile.SYMTYPE
    info.linkname = "/etc/passwd"
    tar.addfile(info)
PY
write_sha "$symlink_archive"
expect_install_fail "archive symlink entry rejected" "$symlink_archive"
expect_unchanged "unsafe archive leaves the binary" "$before"

# --- static no-elevation check ----------------------------------------------
if grep -Eq 'sudo|RunAs|/usr/local|HKLM|HKEY_LOCAL_MACHINE|Program Files|SetEnvironmentVariable|setx' "$installer"; then
  fail "installer contains an elevation or system-wide write"
fi
pass "installer has no elevation or system-wide write"

# --- clean-install smoke on the installed binary ----------------------------
bash "$here/smoke.sh" "$payload" || fail "clean-install smoke"
pass "installed CLI/MCP smoke"

echo "test-install: all checks passed"
