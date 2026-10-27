# M10 freeze procedure

Exact commands for the frozen inputs in this folder. Each step reuses an existing command or script: `git archive`, `git apply`, the Riltra CLI syntax of `release/smoke.sh`, `cargo`, and `verify/verify-freeze.sh`. It adds no orchestration. Paths in this document are placeholders; none is a tracked file path.

## 0. Pins

Checked by `verify/verify-freeze.sh`.

| Item | Value |
|---|---|
| Commit | `7f86891c0942cc1d7efd6ec29c6c0b6b1606e727` |
| Tree | `b2bd50be9ec829fd122635696a2cb95c29fe2bcc` |
| `Cargo.lock` SHA-256 | `9b9ba4dc0d7e7ad6f653dbba99b859e6a3fb9030ee73a3aa9858ef4e544ef905` |
| Toolchain | cargo 1.99.1 (5f94df478 2026-08-27); rustc 1.99.1 (58cf8f9c3 2026-10-13) |

Frozen files and their SHA-256 values are in `SHA256SUMS.txt`.

Variables, all outside the repository:

```bash
REPO=<absolute path of the checkout>
FREEZE="$REPO/docs/pilots/m10/freeze"
COMMIT=7f86891c0942cc1d7efd6ec29c6c0b6b1606e727
CACHE=<dependency cache, shared by runs, outside every workspace>
RUN=<per-run directory>
BIN=<directory for the Arm R binary>
```

## 1. Clean snapshot (every run, both arms)

```bash
SNAP="$RUN/snapshot"
mkdir -p "$SNAP" && git -C "$REPO" archive "$COMMIT" | tar -x -C "$SNAP"
```

`git archive` only reads the repository. The snapshot has no `.git` directory. `riltra index status` then reports no head and no branch.

## 2. Frozen edits (the run's snapshot only)

```bash
(cd "$SNAP" && git apply "$FREEZE/edits/a3-edit.patch")     # A3 runs only
(cd "$SNAP" && git apply "$FREEZE/edits/a4-defect.patch")   # A4 runs only
```

Checks: after A3, `sed -n '267p' "$SNAP/crates/cli/src/knowledge.rs"` prints `const MAX_JSON_DEPTH: usize = 128;`. After A4, the `'\n'` arm at `crates/cli/src/json.rs` line 13 is gone.

## 3. Arm R binary (once per environment, built from the pinned tree only)

```bash
PIN="$RUN/pin"
mkdir -p "$PIN" && git -C "$REPO" archive "$COMMIT" | tar -x -C "$PIN"
(cd "$PIN" && CARGO_TARGET_DIR="$CACHE" cargo clean -p riltra \
  && CARGO_TARGET_DIR="$CACHE" cargo build --locked -p riltra)
mkdir -p "$BIN" && cp "$CACHE/debug/riltra.exe" "$BIN/"   # `riltra` on Linux and macOS
sha256sum "$BIN/riltra.exe"                               # record in provenance
```

Never build this binary from an A3 or A4 copy: the A4 defect would reach the tool. Copy the binary out of the cache at once. Later builds overwrite `$CACHE/debug/riltra.exe`.

## 4. Arm R index

Riltra arguments follow `release/smoke.sh`. On Windows, convert paths with `cygpath -w`.

```bash
export RILTRA_HOME="$RUN/riltra-home"; mkdir -p "$RILTRA_HOME"
"$BIN/riltra.exe" init --repo "$(cygpath -w "$SNAP")" --name m10
"$BIN/riltra.exe" index --repo m10          # time the step; keep the JSON
"$BIN/riltra.exe" index status --repo m10   # freshness before the session
```

Order by task:

- A1 and A2: index the unedited snapshot.
- A3: index the unedited snapshot, then apply the A3 edit. The agent starts on a stale index. Observed on this host: `index status` reports `"freshness":"stale"` with reasons `["content-changed"]`.
- A4: apply the A4 defect, then index. The agent edits code after the index exists.

## 5. Arm B (no Riltra)

```bash
unset RILTRA_HOME
command -v riltra        # must print nothing
```

Arm B runs have no index. Apply the A3 or A4 edit as in §2. The session sees only the snapshot and the frozen prompt.

## 6. Cargo runs (A4)

The harness runs this once before each agent session. The reviewer runs it before the reviewer's verification. Both use the run's own copy.

```bash
export CARGO_TARGET_DIR="$CACHE"
(cd "$SNAP" && cargo clean -p riltra && cargo test --locked --bin riltra json::tests)
```

`cargo clean -p riltra` is required. Control C1 in `verify/verify-freeze.sh` showed the risk: a restored copy run without the clean step reused the defective test binary. It printed no compile line and exited 101, and its binary SHA-256 matched the earlier defective run.

Record for each run: the command, `CARGO_TARGET_DIR`, `RUSTFLAGS` (empty), exit code, the `Compiling riltra` line, the test binary path, and its SHA-256.

## 7. Verification and grading

- Static checks (A1 to A3, both patches, pins), from the repository root:

  ```bash
  bash "$FREEZE/verify/verify-freeze.sh"
  ```

- A4 preflight (two copies, run in sequence: P1 defective, P2 restored, P3 defective, C1 control, P4 restored):

  ```bash
  CARGO_TARGET_DIR="$CACHE" M10_LOG_DIR="$LOG" bash "$FREEZE/verify/verify-freeze.sh" --cargo
  ```

  `LOG` is a directory outside the repository. The script writes `summary.tsv` and one log per step there.

- Agent answers, exact and order-insensitive where the prompt allows it. Any empty diff passes. An extra line in A2 fails.

  ```bash
  diff <(sort agent-A1.txt) <(sort "$FREEZE/expected/A1.txt")
  diff <(sort agent-A2.txt) <(sort "$FREEZE/expected/A2.txt")
  diff <(head -n 1 agent-A3.txt) <(head -n 1 "$FREEZE/expected/A3.txt")
  diff <(tail -n +2 agent-A3.txt | sort) <(tail -n +2 "$FREEZE/expected/A3.txt" | sort)
  ```

- A4: the reviewer applies §6 to the agent's final copy and compares the result line with `expected/A4.txt`.

## 8. Records

Each run keeps the provenance fields of the design gate (§7), plus: the commands above, `CARGO_TARGET_DIR`, exit codes, the Arm R binary SHA-256 from §3, and `riltra index` and `index status` output. Logs that contain local paths are redacted before commit (design gate B-7).
