# SPDX-License-Identifier: Apache-2.0
"""E0-S: Rust semantic provider reliability, cold homes and warm queries.

Inputs that are frozen and never edited:
  - the M9 G-SEM fixture (docs/pilots/m9-rc-ee7482d/gsem/fixture)
  - the byte checker (docs/pilots/m9-rc-ee7482d/evidence/check-gsem.ps1)

Phases, run in this order:
  prepare  write expected.json from the frozen fixture bytes. It is written
           once, before any riltra query, and never recomputed from riltra output.
  cold     fresh RILTRA_HOME per trial: Rust definition and references through
           rust-analyzer, then a TypeScript SCIP control (producer, import,
           two queries). Use --trials to run one trial per invocation.
  warm     three warm rounds in cold trial 1's home: Rust definition and
           references, after that home already has an index, a provider
           workspace (fixture-rs/target) and a previous answer.
  report   classify each query, run check-gsem.ps1 on each trial, write
           results, and copy a sanitized copy of the raw outputs.

Each query gets one status: CORRECT-COMPLETE, PARTIAL, WRONG, EMPTY-DIAGNOSED,
EMPTY-SILENT, FALLBACK, ERROR, TIMEOUT or PARSE-ERROR. A failed attempt is
recorded as it is. Nothing is re-run to replace it.
"""

import argparse
import hashlib
import json
import os
import re
import shutil
import statistics
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]
PILOT = HERE.parent
FROZEN = REPO / "docs" / "pilots" / "m9-rc-ee7482d" / "gsem" / "fixture"
CHECK_GSEM = REPO / "docs" / "pilots" / "m9-rc-ee7482d" / "evidence" / "check-gsem.ps1"
EXPECTED = HERE / "expected.json"
DEFAULT_BIN = REPO / "target" / "debug" / "riltra.exe"

FIXTURE_FILES = [
    "rs/Cargo.toml",
    "rs/Cargo.lock",
    "rs/src/main.rs",
    "ts/package.json",
    "ts/tsconfig.json",
    "ts/src/main.ts",
    "ts/src/greet.ts",
]
# Inputs are 1-based line and column. The answers use 0-based lines and
# columns plus byte offsets, which are the checkable location.
RS_POSITION = ["--line", "1", "--col", "4"]  # `greet` in `fn greet`
TS_POSITION = ["--line", "3", "--col", "13"]  # `greet` in `greet("world")`
TIMEOUT_S = 300
COLD_TRIALS = 3
WARM_ROUNDS = 3
WORD = "greet"

# Source constants the timings depend on (cited from the code under test).
CONSTANTS = {
    "analysis_wait_cap_s": 15,  # crates/cli/src/lsp.rs, await_analysis
    "request_timeout_s": 30,  # crates/cli/src/lsp.rs, ServerSpec::new
    "empty_answer_retries": 5,  # crates/cli/src/lsp.rs, definition/references
    "retry_sleep_s": 2,
}


def now_utc():
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_expected():
    if not EXPECTED.exists():
        sys.exit("expected.json is missing; run the prepare phase first")
    return json.loads(EXPECTED.read_text(encoding="utf-8"))


def occurrences(data, pattern):
    return [
        {"byte_start": m.start(1), "byte_end": m.end(1)}
        for m in re.finditer(pattern, data)
    ]


def cmd_prepare(args):
    if EXPECTED.exists() and not args.force:
        sys.exit("expected.json already exists: pre-registration is written once")
    main_rs = (FROZEN / "rs" / "src" / "main.rs").read_bytes()
    main_ts = (FROZEN / "ts" / "src" / "main.ts").read_bytes()
    greet_ts = (FROZEN / "ts" / "src" / "greet.ts").read_bytes()
    expected = {
        "registered_at_utc": now_utc(),
        "rule": {
            "rs_definition": "span of `greet` in `fn greet` (pattern: fn (greet)\\b)",
            "rs_references": "every identifier `greet` (pattern: \\b(greet)\\b), declaration included",
            "ts_references": (
                "identifier `greet` in src/main.ts used as the symbol (import binding and call); "
                "the module specifier \"./greet\" is not a reference and is excluded"
            ),
            "ts_definition": "span of `greet` in `function greet` in src/greet.ts",
        },
        "fixture_sha256": {rel: sha256_file(FROZEN / rel) for rel in FIXTURE_FILES},
        "rs": {
            "definition": [
                {"path": "src/main.rs", **span}
                for span in occurrences(main_rs, rb"fn (greet)\b")
            ],
            "references": [
                {"path": "src/main.rs", **span}
                for span in occurrences(main_rs, rb"\b(greet)\b")
            ],
        },
        "ts": {
            "references": [
                {"path": "src/main.ts", **span}
                for span in occurrences(main_ts, rb"(?<![\w\"'/.])(greet)\b")
            ],
            "definition": [
                {"path": "src/greet.ts", **span}
                for span in occurrences(greet_ts, rb"function (greet)\b")
            ],
        },
    }
    EXPECTED.write_text(json.dumps(expected, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(EXPECTED.read_text(encoding="utf-8"))


def scratch_paths(scratch):
    return {
        "raw": scratch / "raw",
        "rows": scratch / "rows.jsonl",
        "env": scratch / "env.json",
    }


def append_row(scratch, row):
    with open(scratch / "rows.jsonl", "a", encoding="utf-8", newline="\n") as handle:
        handle.write(json.dumps(row, sort_keys=True) + "\n")


def save_raw(raw_dir, name, text):
    raw_dir.mkdir(parents=True, exist_ok=True)
    (raw_dir / name).write_bytes(text.encode("utf-8"))


def run_riltra(bin_path, home, args):
    env = dict(os.environ)
    env["RILTRA_HOME"] = str(home)
    started = time.perf_counter()
    try:
        proc = subprocess.run(
            [str(bin_path), *args],
            env=env,
            capture_output=True,
            timeout=TIMEOUT_S,
        )
        code = proc.returncode
        stdout = proc.stdout.decode("utf-8", "replace")
        stderr = proc.stderr.decode("utf-8", "replace")
    except subprocess.TimeoutExpired as exc:
        code = "TIMEOUT"
        stdout = (exc.stdout or b"").decode("utf-8", "replace")
        stderr = (exc.stderr or b"").decode("utf-8", "replace")
    elapsed_ms = round((time.perf_counter() - started) * 1000, 1)
    return code, stdout, stderr, elapsed_ms


def step(scratch, bin_path, home, trial, phase, query, repo, args, raw_name):
    """Run one riltra command, keep its outputs, and record one row."""
    code, stdout, stderr, elapsed_ms = run_riltra(bin_path, home, args)
    raw = scratch / "raw" / trial
    save_raw(raw, raw_name, stdout)
    if stderr:
        save_raw(raw, raw_name + ".stderr.txt", stderr)
    append_row(
        scratch,
        {
            "trial": trial,
            "phase": phase,
            "query": query,
            "repo": repo,
            "exit": code,
            "elapsed_ms": elapsed_ms,
            "file": raw_name,
            "started_utc": now_utc(),
        },
    )
    return code


def lsp_query(scratch, bin_path, home, rs_dir, ra, trial, phase, query):
    target_before = (rs_dir / "target").exists()
    args = ["semantic", "query", "--repo", "rs", "--path", "src/main.rs", *RS_POSITION]
    if query == "references":
        args.append("--refs")
    args += ["--lsp-server", ra]
    code, stdout, stderr, elapsed_ms = run_riltra(bin_path, home, args)
    raw = scratch / "raw" / trial
    name = f"rust-analyzer-{query}.json"
    save_raw(raw, name, stdout)
    if stderr:
        save_raw(raw, name + ".stderr.txt", stderr)
    append_row(
        scratch,
        {
            "trial": trial,
            "phase": phase,
            "query": query,
            "repo": "rs",
            "exit": code,
            "elapsed_ms": elapsed_ms,
            "target_before": target_before,
            "file": name,
            "started_utc": now_utc(),
        },
    )


def check_copies(home, expected):
    for rel, digest in expected["fixture_sha256"].items():
        repo_dir, inner = rel.split("/", 1)
        copied = home / f"fixture-{repo_dir}" / inner
        if sha256_file(copied) != digest:
            sys.exit(f"fixture copy differs from the frozen bytes: {rel}")


def ts_control(scratch, bin_path, home, ts_dir, scip_main, trial, phase):
    step(scratch, bin_path, home, trial, phase, "ts-init", "ts",
         ["init", "--repo", str(ts_dir), "--name", "ts"], "ts-init.json")
    step(scratch, bin_path, home, trial, phase, "ts-index", "ts",
         ["index", "--repo", "ts"], "ts-index.json")
    node = shutil.which("node") or "node"
    scip_out = ts_dir / "index.scip"
    started = time.perf_counter()
    try:
        proc = subprocess.run(
            [node, str(scip_main), "index", "--cwd", str(ts_dir), "--output", str(scip_out)],
            cwd=str(ts_dir),
            capture_output=True,
            timeout=TIMEOUT_S,
        )
        producer_code = proc.returncode
        producer_text = proc.stdout.decode("utf-8", "replace") + proc.stderr.decode("utf-8", "replace")
    except subprocess.TimeoutExpired:
        producer_code = "TIMEOUT"
        producer_text = ""
    elapsed_ms = round((time.perf_counter() - started) * 1000, 1)
    save_raw(scratch / "raw" / trial, "scip-typescript-producer.txt", producer_text)
    append_row(scratch, {"trial": trial, "phase": phase, "query": "ts-producer",
                         "repo": "ts", "exit": producer_code, "elapsed_ms": elapsed_ms,
                         "file": "scip-typescript-producer.txt", "started_utc": now_utc()})
    if scip_out.exists():
        step(scratch, bin_path, home, trial, phase, "ts-import", "ts",
             ["semantic", "import", "--repo", "ts", "--scip", str(scip_out)],
             "scip-typescript-import.json")
    else:
        append_row(scratch, {"trial": trial, "phase": phase, "query": "ts-import",
                             "repo": "ts", "exit": "NO-INDEX", "elapsed_ms": 0,
                             "file": "scip-typescript-import.json", "started_utc": now_utc()})
    step(scratch, bin_path, home, trial, phase, "ts-references", "ts",
         ["semantic", "query", "--repo", "ts", "--path", "src/main.ts", *TS_POSITION, "--refs"],
         "scip-typescript-query.json")
    step(scratch, bin_path, home, trial, phase, "ts-definition", "ts",
         ["semantic", "query", "--repo", "ts", "--path", "src/main.ts", *TS_POSITION],
         "ts-definition-from-callsite.json")


def write_env(scratch, bin_path, ra, scip_main):
    if scratch_paths(scratch)["env"].exists():
        return
    node = shutil.which("node") or "node"

    def version(cmd):
        try:
            proc = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
            return (proc.stdout + proc.stderr).strip()
        except (OSError, subprocess.TimeoutExpired) as exc:
            return f"unavailable: {exc.__class__.__name__}"

    try:
        head = subprocess.run(["git", "-C", str(REPO), "rev-parse", "HEAD"],
                              capture_output=True, text=True, timeout=60).stdout.strip()
        status = subprocess.run(["git", "-C", str(REPO), "status", "--porcelain"],
                                capture_output=True, text=True, timeout=60).stdout
    except OSError:
        head, status = "unavailable", "unavailable"
    env = {
        "recorded_at_utc": now_utc(),
        "riltra_binary": {"name": bin_path.name, "sha256": sha256_file(bin_path),
                          "bytes": bin_path.stat().st_size},
        "riltra_source_head": head,
        "git_status_porcelain": status.splitlines(),
        "rust_analyzer": {"name": Path(ra).name, "version": version([ra, "--version"]),
                          "sha256": sha256_file(ra)},
        "node": version([node, "--version"]),
        "scip_typescript": {"version": version([node, str(scip_main), "--version"])},
        "host": {"os": "Windows 11 Pro 10.0.26300", "python": sys.version.split()[0],
                 "cpu_count": os.cpu_count()},
        "constants": CONSTANTS,
        "scope_note": ("Cold means a fresh RILTRA_HOME and a fresh fixture copy with no "
                       "target/ directory. The operating-system page cache is not cleared."),
    }
    scratch_paths(scratch)["env"].write_text(json.dumps(env, indent=2) + "\n", encoding="utf-8", newline="\n")


def cmd_cold(args):
    expected = load_expected()
    bin_path, ra, scip_main = Path(args.bin), args.rust_analyzer, Path(args.scip_typescript)
    scratch = Path(args.scratch)
    scratch.mkdir(parents=True, exist_ok=True)
    write_env(scratch, bin_path, ra, scip_main)
    # --prefix and --phase keep a later block (for example the 20-home replicate)
    # apart from the counted cold trials in the report; the defaults are the counted ones.
    for k in args.trials:
        trial = f"{args.prefix}-{k}"
        home = scratch / trial / "home"
        if home.exists():
            sys.exit(f"{trial} already ran; the trial is not repeated")
        home.mkdir(parents=True)
        shutil.copytree(FROZEN / "rs", home / "fixture-rs")
        shutil.copytree(FROZEN / "ts", home / "fixture-ts")
        check_copies(home, expected)
        rs_dir, ts_dir = home / "fixture-rs", home / "fixture-ts"
        phase = args.phase
        step(scratch, bin_path, home, trial, phase, "rs-init", "rs",
             ["init", "--repo", str(rs_dir), "--name", "rs"], "rs-init.json")
        step(scratch, bin_path, home, trial, phase, "rs-index", "rs",
             ["index", "--repo", "rs"], "rs-index.json")
        lsp_query(scratch, bin_path, home, rs_dir, ra, trial, phase, "definition")
        lsp_query(scratch, bin_path, home, rs_dir, ra, trial, phase, "references")
        ts_control(scratch, bin_path, home, ts_dir, scip_main, trial, phase)
        print(f"{trial} done")


def cmd_warm(args):
    bin_path, ra = Path(args.bin), args.rust_analyzer
    scratch = Path(args.scratch)
    home = scratch / "cold-1" / "home"
    if not home.exists():
        sys.exit("cold-1 must run first: warm rounds reuse its home")
    rs_dir = home / "fixture-rs"
    for r in range(1, WARM_ROUNDS + 1):
        trial = f"warm-{r}"
        lsp_query(scratch, bin_path, home, rs_dir, ra, trial, "warm", "definition")
        lsp_query(scratch, bin_path, home, rs_dir, ra, trial, "warm", "references")
        print(f"{trial} done")


def cmd_replicate(args):
    """Diagnostic replicates: fresh home, Rust definition only. Not part of the counted protocol."""
    expected = load_expected()
    bin_path, ra = Path(args.bin), args.rust_analyzer
    scratch = Path(args.scratch)
    for k in range(1, args.count + 1):
        trial = f"diag-{k}"
        home = scratch / trial / "home"
        if home.exists():
            sys.exit(f"{trial} already ran; the replicate is not repeated")
        home.mkdir(parents=True)
        shutil.copytree(FROZEN / "rs", home / "fixture-rs")
        for rel, digest in expected["fixture_sha256"].items():
            if rel.startswith("rs/") and sha256_file(home / "fixture-rs" / rel[3:]) != digest:
                sys.exit(f"fixture copy differs from the frozen bytes: {rel}")
        rs_dir = home / "fixture-rs"
        step(scratch, bin_path, home, trial, "diagnostic", "rs-init", "rs",
             ["init", "--repo", str(rs_dir), "--name", "rs"], "rs-init.json")
        step(scratch, bin_path, home, trial, "diagnostic", "rs-index", "rs",
             ["index", "--repo", "rs"], "rs-index.json")
        lsp_query(scratch, bin_path, home, rs_dir, ra, trial, "diagnostic", "definition")
        print(f"{trial} done")


FIXTURE_BY_KEY = {
    ("rs", "src/main.rs"): FROZEN / "rs" / "src" / "main.rs",
    ("ts", "src/main.ts"): FROZEN / "ts" / "src" / "main.ts",
    ("ts", "src/greet.ts"): FROZEN / "ts" / "src" / "greet.ts",
}
EXPECTED_KEY = {
    ("rs", "definition"): ("rs", "definition"),
    ("rs", "references"): ("rs", "references"),
    ("ts", "ts-references"): ("ts", "references"),
    ("ts", "ts-definition"): ("ts", "definition"),
}
def classify(row, scratch, expected):
    base = {k: row.get(k) for k in ("trial", "phase", "query", "repo", "exit", "elapsed_ms", "target_before")}
    code = row["exit"]
    if code == "TIMEOUT":
        return {**base, "status": "TIMEOUT"}
    if code != 0:
        return {**base, "status": "ERROR"}
    text = (scratch / "raw" / row["trial"] / row["file"]).read_text(encoding="utf-8")
    try:
        doc = json.loads(text)
    except json.JSONDecodeError:
        return {**base, "status": "PARSE-ERROR"}
    key = (row["repo"], row["query"])
    exp_repo, exp_role = EXPECTED_KEY[key]
    expected_set = {(e["path"], e["byte_start"], e["byte_end"]) for e in expected[exp_repo][exp_role]}
    hits_key = "definitions" if row["query"] in ("definition", "ts-definition") else "references"
    hits = doc.get(hits_key, [])
    diags = doc.get("server_diagnostics", [])
    fallback = doc.get("fallback")
    observed, wrong = [], []
    for hit in hits:
        span = hit["span"]
        item = (hit["path"], span["byte_start"], span["byte_end"])
        observed.append(item)
        source = FIXTURE_BY_KEY.get((row["repo"], hit["path"]))
        grounded = source is not None and (
            source.read_bytes()[span["byte_start"]:span["byte_end"]] == WORD.encode()
        )
        if item not in expected_set or not grounded:
            wrong.append(item)
    observed_set = set(observed)
    missing = sorted(expected_set - observed_set)
    labels = {
        "verification": sorted({h["verification"] for h in hits}),
        "precision": sorted({h["precision"] for h in hits}),
    }
    if fallback is not None:
        status = "FALLBACK"
    elif not hits:
        status = "EMPTY-DIAGNOSED" if diags else "EMPTY-SILENT"
    elif wrong:
        status = "WRONG"
    elif missing:
        status = "PARTIAL"
    else:
        status = "CORRECT-COMPLETE"
    return {
        **base,
        "status": status,
        "hits": len(hits),
        "duplicates": len(observed) - len(observed_set),
        "missing": [f"{p}:{s}-{e}" for p, s, e in missing],
        "wrong": [f"{p}:{s}-{e}" for p, s, e in wrong],
        "analysis_timeout": any("analysis wait timed out" in d for d in diags),
        "converged_after_retry": any("converged after" in d for d in diags),
        "verification": ",".join(labels["verification"]),
        "precision": ",".join(labels["precision"]),
        "fallback": None if fallback is None else "present",
        "diagnostics": len(diags),
    }


def wilson(successes, n, z=1.96):
    if n == 0:
        return None, None
    p = successes / n
    denom = 1 + z * z / n
    center = (p + z * z / (2 * n)) / denom
    half = z * ((p * (1 - p) / n + z * z / (4 * n * n)) ** 0.5) / denom
    return round(max(0.0, center - half), 3), round(min(1.0, center + half), 3)


def summarize(results):
    groups = {}
    for r in results:
        group = (r["phase"], r["repo"], r["query"])
        groups.setdefault(group, []).append(r)
    table = []
    for (phase, repo, query), items in sorted(groups.items()):
        counts = {}
        for item in items:
            counts[item["status"]] = counts.get(item["status"], 0) + 1
        n = len(items)
        success = counts.get("CORRECT-COMPLETE", 0)
        answered = sum(counts.get(s, 0) for s in ("CORRECT-COMPLETE", "PARTIAL", "WRONG"))
        times = [i["elapsed_ms"] for i in items if isinstance(i.get("elapsed_ms"), (int, float))]
        lo, hi = wilson(success, n)
        table.append({
            "phase": phase,
            "repo": repo,
            "query": query,
            "n": n,
            "counts": counts,
            "availability_complete": f"{success}/{n}",
            "wilson95": [lo, hi],
            "answered": answered,
            "grounded_given_answer": (
                f"{answered - counts.get('WRONG', 0)}/{answered}" if answered else "n/a"
            ),
            "empty_silent": counts.get("EMPTY-SILENT", 0),
            "median_ms": round(statistics.median(times), 1) if times else None,
            "max_ms": round(max(times), 1) if times else None,
        })
    return table


def sanitize_roots(scratch):
    roots = []
    for root, token in ((scratch, "<SCRATCH>"), (REPO, "<REPO>"), (Path.home(), "<USERHOME>")):
        for candidate in {Path(root), Path(root).resolve()}:
            text = str(candidate)
            forms = {text, text.replace("\\", "/"), text.replace("\\", "\\\\"),
                     text.replace("/", "\\\\")}
            for form in forms:
                roots.append((form, token))
    roots.sort(key=lambda pair: -len(pair[0]))
    return roots


def sanitize_text(text, roots):
    count = 0
    # Windows verbatim prefixes carry no user data; remove them before the path roots.
    for prefix in ("\\\\\\\\?\\\\", "\\\\?\\"):
        count += text.count(prefix)
        text = text.replace(prefix, "")
    for form, token in roots:
        if form and form in text:
            count += text.count(form)
            text = text.replace(form, token)
    return text, count


LEFTOVER = re.compile(r"\b[A-Za-z]:[\\/]|/Users/|Users[\\/]ASUS")


def cmd_report(args):
    scratch = Path(args.scratch)
    expected = load_expected()
    rows = [json.loads(line) for line in (scratch / "rows.jsonl").read_text(encoding="utf-8").splitlines() if line.strip()]
    results = [classify(row, scratch, expected) for row in rows if (row["repo"], row["query"]) in EXPECTED_KEY]
    table = summarize(results)
    out = Path(args.out)
    (out / "results").mkdir(parents=True, exist_ok=True)
    roots = sanitize_roots(scratch)
    # Byte check: reuse the frozen checker on each trial's raw outputs and home.
    for trial_dir in sorted((scratch / "raw").iterdir()):
        trial = trial_dir.name
        home = scratch / ("cold-1" if trial.startswith("warm") else trial) / "home"
        env = dict(os.environ, E0S_HOME=str(home), E0S_RAW=str(trial_dir), E0S_LABEL=trial,
                   E0S_CHECK=str(CHECK_GSEM))
        proc = subprocess.run(
            ["pwsh", "-NoProfile", "-Command",
             "& $env:E0S_CHECK -HomeDir $env:E0S_HOME -RawDir $env:E0S_RAW -Label $env:E0S_LABEL "
             "| ConvertTo-Json -Depth 5"],
            capture_output=True, text=True, env=env, timeout=600)
        clean, _ = sanitize_text(proc.stdout + proc.stderr, roots)
        save_raw(out / "byte-check", f"{trial}.json", clean)
    accounting = []
    raw_out = out / "raw"
    for path in sorted(scratch.rglob("*")):
        if not path.is_file():
            continue
        rel = path.relative_to(scratch)
        if "home" in rel.parts or rel.parts[0] == "diag":
            continue
        if rel.parts[0] == "raw":
            target = raw_out.joinpath(*rel.parts[1:])
        else:
            target = raw_out / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        before = hashlib.sha256(text.encode("utf-8")).hexdigest()
        clean, replacements = sanitize_text(text, roots)
        if LEFTOVER.search(clean):
            print(f"LEFTOVER local path pattern in {rel}", file=sys.stderr)
        target.write_text(clean, encoding="utf-8", newline="\n")
        accounting.append({"file": str(target.relative_to(out)).replace("\\", "/"),
                           "source_sha256_utf8": before, "replacements": replacements})
    # Diagnosis: readiness probes (readiness_probe.py) on fresh fixture copies, sanitized.
    diag_out = out / "diagnosis"
    diag_out.mkdir(parents=True, exist_ok=True)
    probes = []
    for path in sorted((scratch / "diag").glob("*.json")):
        text = path.read_text(encoding="utf-8")
        clean, replacements = sanitize_text(text, roots)
        # The probe already replaced the profile prefix with <USERHOME>; the scratch part of the path follows it.
        clean, extra = re.subn(r"<USERHOME>/AppData/Local/Temp/claude/[^\"\s]*?/scratchpad/e0s/",
                               "<SCRATCH>/", clean)
        accounting.append({"file": f"diagnosis/{path.name}",
                           "source_sha256_utf8": hashlib.sha256(text.encode("utf-8")).hexdigest(),
                           "replacements": replacements + extra})
        (diag_out / path.name).write_text(clean, encoding="utf-8", newline="\n")
        data = json.loads(text)
        probes.append({
            "file": path.name,
            "target_present_before": data["target_present_before"],
            "readiness_seen": data["readiness_seen"],
            "readiness_ms": data["readiness_ms"],
            "definition_answer_ms": data["definition_answer_ms"],
            "definition_locations": len(data["definition_locations"]),
            "publish_count": data["publish_count"],
        })
    ready_ms = [p["readiness_ms"] for p in probes if p["readiness_seen"]]
    probe_summary = {
        "probes": len(probes),
        "readiness_seen": len(ready_ms),
        "readiness_ms_min": min(ready_ms) if ready_ms else None,
        "readiness_ms_median": round(statistics.median(ready_ms), 1) if ready_ms else None,
        "readiness_ms_max": max(ready_ms) if ready_ms else None,
        "readiness_above_15s_cap": sum(1 for r in ready_ms if r > 15000),
        "runs": probes,
    }
    (diag_out / "summary.json").write_text(json.dumps(probe_summary, indent=2) + "\n", encoding="utf-8", newline="\n")
    (out / "results" / "redaction-accounting.json").write_text(
        json.dumps(accounting, indent=2) + "\n", encoding="utf-8", newline="\n")
    tsv_cols = ["trial", "phase", "repo", "query", "status", "exit", "elapsed_ms",
                "target_before", "hits", "duplicates", "missing", "wrong",
                "analysis_timeout", "converged_after_retry", "verification", "precision",
                "fallback", "diagnostics"]
    lines = ["\t".join(tsv_cols)]
    for r in results:
        lines.append("\t".join(str(r.get(c, "")) for c in tsv_cols))
    (out / "results" / "queries.tsv").write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    (out / "results" / "summary.json").write_text(
        json.dumps({"expected_registered_at_utc": expected["registered_at_utc"], "groups": table},
                   indent=2) + "\n", encoding="utf-8", newline="\n")
    print(json.dumps(table, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("prepare")
    p.add_argument("--force", action="store_true")
    p.set_defaults(func=cmd_prepare)
    for name, func in (("cold", cmd_cold), ("warm", cmd_warm), ("replicate", cmd_replicate)):
        q = sub.add_parser(name)
        q.add_argument("--bin", default=str(DEFAULT_BIN))
        q.add_argument("--scratch", required=True)
        q.add_argument("--rust-analyzer", required=True)
        if name == "replicate":
            q.add_argument("--count", type=int, default=5)
        if name == "cold":
            q.add_argument("--prefix", default="cold")
            q.add_argument("--phase", default="cold")
            q.add_argument("--scip-typescript", default=str(
                Path(os.environ.get("APPDATA", "")) / "npm" / "node_modules" / "@sourcegraph"
                / "scip-typescript" / "dist" / "src" / "main.js"))
            q.add_argument("--trials", type=int, nargs="+", default=list(range(1, COLD_TRIALS + 1)))
        q.set_defaults(func=func)
    q = sub.add_parser("report")
    q.add_argument("--scratch", required=True)
    q.add_argument("--out", default=str(PILOT))
    q.set_defaults(func=cmd_report)
    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
