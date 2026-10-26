# Template. Replace each placeholder for the host before use. The committed harness holds no local path.
export ROOT=<SCRATCH-ROOT>
export REPO=<REPO-CHECKOUT>
export CACHE=<CARGO-CACHE-OUTSIDE-WORKSPACES>
export CLAUDE_BIN=<CLAUDE-CODE-EXECUTABLE>
export FREEZE="$REPO/docs/pilots/m10/freeze"
export COMMIT=7f86891c0942cc1d7efd6ec29c6c0b6b1606e727
export H="$ROOT/harness"

# Redaction inputs for harness/redact.js. *_WIN is the Windows form, *_POSIX the Git Bash form.
# PATH_SLUGS: comma-separated Claude Code project-folder names derived from local paths.
# Values stay local and are never committed.
export ROOT_WIN=<SCRATCH-ROOT-WINDOWS>
export ROOT_POSIX=<SCRATCH-ROOT-POSIX>
export REPO_WIN=<REPO-CHECKOUT-WINDOWS>
export REPO_POSIX=<REPO-CHECKOUT-POSIX>
export HOME_WIN=<USER-PROFILE-WINDOWS>
export HOME_POSIX=<USER-PROFILE-POSIX>
export HOME_SHORT_WIN=<USER-PROFILE-SHORT-NAME-WINDOWS>
export HOME_SHORT_POSIX=<USER-PROFILE-SHORT-NAME-POSIX>
export SCRATCHPAD_WIN=<SESSION-SCRATCHPAD-WINDOWS>
export SCRATCHPAD_POSIX=<SESSION-SCRATCHPAD-POSIX>
export PATH_SLUGS=<SLUG-USER-PROFILE>,<SLUG-RUN-SCRATCHPAD>,<SLUG-SNAPSHOT-FOLDER>,<SLUG-SCRATCH-ROOT>
export USER_NAME=<ACCOUNT-DISPLAY-NAME>
