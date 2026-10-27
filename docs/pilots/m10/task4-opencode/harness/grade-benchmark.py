#!/usr/bin/env python3
"""M10 Task 4 OpenCode pilot grader.

Reads the eight finished first-attempt sessions, verifies prompt delivery, grades
answers against the frozen expectations, records tool usage and filesystem access,
and runs the A4 reviewer cargo check on a byte-identical copy of each final workspace.

Writes only under <root>/export/grading. Never modifies a run directory.
"""
import hashlib
import json
import os
import re
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import time
import difflib
from pathlib import Path

ROOT = Path(r"<ROOT>")
FREEZE = Path(r"<REPO>\docs\pilots\m10\freeze")
CACHE = Path(r"<USERPROFILE>\AppData\Local\Temp\opencode\m10-cache")
OUT = ROOT / "export" / "grading"
TASKS = ["A1", "A2", "A3", "A4"]
ARMS = ["R", "B"]


def sha256_bytes(b):
    return hashlib.sha256(b).hexdigest()


def sha256_file(p):
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def norm_lf(s):
    return s.replace("\r\n", "\n").replace("\r", "\n")


def read_text(p):
    return Path(p).read_text(encoding="utf-8", errors="replace")


def answer_lines(text):
    return [ln.rstrip() for ln in norm_lf(text).split("\n") if ln.strip()]


CONTENT_PATTERNS = {
    "A1": re.compile(r"^\w+ \S+:\d+ bytes \d+-\d+$"),
    "A2": re.compile(r"^\w+ \S+:\d+$"),
    "A3": re.compile(r"^(?:value \d+|reader \S+:\d+)$"),
    "A4": re.compile(r"^state=(?:defect|restored)\b"),
}


def literal_lines(text):
    t = norm_lf(text)
    if t.endswith("\n"):
        t = t[:-1]
    return [ln.rstrip() for ln in t.split("\n")] if t else []


def content_lines(task, text):
    return [ln.strip() for ln in literal_lines(text) if CONTENT_PATTERNS[task].match(ln.strip())]


def expected_lines(task):
    return answer_lines(read_text(FREEZE / "expected" / f"{task}.txt"))


def db_rows(run_dir):
    src = run_dir / "profile" / "data" / "opencode" / "opencode.db"
    if not src.exists():
        return None, "db missing"
    dst_dir = OUT / "db-copies" / run_dir.name
    dst_dir.mkdir(parents=True, exist_ok=True)
    dst = dst_dir / "opencode.db"
    for suffix in ["", "-wal", "-shm"]:
        s = Path(str(src) + suffix)
        if s.exists():
            shutil.copy2(s, Path(str(dst) + suffix))
    try:
        con = sqlite3.connect(dst)
        con.row_factory = sqlite3.Row
        rows = {}
        rows["session"] = [dict(r) for r in con.execute("select * from session_v2")]
        rows["messages"] = [dict(r) for r in con.execute("select * from session_message order by seq")]
        con.close()
        return rows, None
    except Exception as e:
        return None, f"db error {e}"


def parse_messages(rows):
    user_text = None
    assist = []
    tools = []
    errors = []
    for m in rows.get("messages", []):
        try:
            data = json.loads(m["data"]) if isinstance(m["data"], str) else m["data"]
        except Exception:
            continue
        if m["type"] == "user":
            if user_text is None:
                user_text = data.get("text", "")
        elif m["type"] == "assistant":
            texts = []
            for c in data.get("content", []):
                if c.get("type") == "text":
                    texts.append(c.get("text", ""))
                elif c.get("type") == "tool":
                    st = c.get("state", {}) or {}
                    meta = (st.get("metadata") or {}).get("metadata") or {}
                    tools.append({
                        "name": c.get("name"),
                        "status": st.get("status"),
                        "input": st.get("input"),
                        "output": st.get("output"),
                        "exit": meta.get("exit"),
                        "truncated": meta.get("truncated"),
                    })
            assist.append({"text": "\n".join(texts), "data": data})
        elif isinstance(data, dict) and data.get("error"):
            errors.append(data["error"])
    return user_text, assist, tools, errors


def tool_categories(tools):
    cats = {"native": {}, "riltra_mcp": 0, "riltra_cli": 0}
    for t in tools:
        name = t["name"] or ""
        cats["native"][name] = cats["native"].get(name, 0) + 1
        if name.startswith("riltra_"):
            cats["riltra_mcp"] += 1
        if name == "shell":
            cmd = str((t.get("input") or {}).get("command", ""))
            if re.search(r"(?<![\w.-])riltra(\.exe)?\s", cmd):
                cats["riltra_cli"] += 1
    return cats


def fs_audit(tools, workspace):
    ws = str(workspace).lower()
    suspicious = []
    all_paths = []
    forbidden = [
        r"<REPO>", r"\\\.config\\opencode", r"\\\.local\\share\\opencode",
        r"m10-task4-precheck\\expected", r"m10-task4-precheck\\evidence",
        r"m10-task4-precheck\\snapshot-pin", r"m10-task4-precheck\\pin",
        r"m10-task4-precheck\\sessions\\(?!bench\\)", r"\.\.\\\.\.",
    ]
    for t in tools:
        inp = t.get("input") or {}
        vals = [str(v) for v in inp.values() if isinstance(v, (str, int, float))]
        blob = " ".join(vals)
        if t["name"] == "shell":
            low = blob.lower().replace("/", "\\")
            for f in forbidden:
                if re.search(f, low):
                    suspicious.append({"tool": t["name"], "reason": f, "input": blob[:800]})
            for m in re.finditer(r"[A-Za-z]:\\[^\s\"'<>|;]+", blob):
                all_paths.append(m.group(0))
        else:
            for v in vals:
                if re.match(r"^[A-Za-z]:[\\/]", v) or v.startswith(".."):
                    try:
                        p = Path(v).resolve()
                        if not str(p).lower().startswith(ws):
                            suspicious.append({"tool": t["name"], "reason": "outside workspace", "input": v[:500]})
                    except Exception:
                        pass
                    all_paths.append(v)
    return {"suspicious": suspicious, "paths": sorted(set(all_paths))}


def verify_a1_facts(lines, snapshot):
    facts = []
    for ln in lines:
        m = re.match(r"^(\w+) (.+?):(\d+) bytes (\d+)-(\d+)$", ln)
        if not m:
            facts.append({"line": ln, "ok": False, "why": "format"})
            continue
        name, rel, line_no, start, end = m.group(1), m.group(2), int(m.group(3)), int(m.group(4)), int(m.group(5))
        p = snapshot / rel
        if not p.exists():
            facts.append({"line": ln, "ok": False, "why": "missing file"})
            continue
        b = p.read_bytes()
        ok = b[start:end].decode("utf-8", "replace") == name
        line_ok = b.count(b"\n") + 1 >= line_no
        facts.append({"line": ln, "ok": ok and line_ok, "bytes_match": ok, "line_in_range": line_ok})
    return facts


def verify_a3_facts(lines, patched_root):
    facts = []
    value_lines = [l for l in lines if l.startswith("value ")]
    reader_lines = [l for l in lines if l.startswith("reader ")]
    p = patched_root / "crates" / "cli" / "src" / "knowledge.rs"
    txt = read_text(p) if p.exists() else ""
    m = re.search(r"const MAX_JSON_DEPTH:\s*usize\s*=\s*(\d+);", txt)
    facts.append({"check": "value_line", "ok": bool(m) and value_lines == [f"value {m.group(1)}"]})
    for ln in reader_lines:
        mm = re.match(r"^reader (.+?):(\d+)$", ln)
        if not mm:
            facts.append({"line": ln, "ok": False, "why": "format"})
            continue
        fp = patched_root / mm.group(1)
        n = int(mm.group(2))
        src = read_text(fp).split("\n") if fp.exists() else []
        line_txt = src[n - 1] if 0 < n <= len(src) else ""
        facts.append({"line": ln, "ok": "MAX_JSON_DEPTH" in line_txt, "line_text": line_txt.strip()})
    return facts


def run_cargo_a4(run_name, workspace, out_dir):
    copy_dir = out_dir / "cargo-copies" / run_name
    if copy_dir.exists():
        shutil.rmtree(copy_dir)
    copy_dir.mkdir(parents=True)
    # Copy only what cargo needs; the deep docs paths exceed MAX_PATH at this destination.
    for name in ("Cargo.toml", "Cargo.lock"):
        shutil.copy2(workspace / name, copy_dir / name)
    shutil.copytree(workspace / "crates", copy_dir / "crates")
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(CACHE)
    env["RUSTFLAGS"] = ""
    env["NO_COLOR"] = "1"
    t0 = time.time()
    clean = subprocess.run(["cargo", "clean", "-p", "riltra"], cwd=copy_dir, env=env,
                           capture_output=True, text=True, errors="replace")
    test = subprocess.run(["cargo", "test", "--locked", "--bin", "riltra", "json::tests"],
                          cwd=copy_dir, env=env, capture_output=True, text=True, errors="replace")
    dur = round(time.time() - t0, 3)
    text = (clean.stdout or "") + (clean.stderr or "") + (test.stdout or "") + (test.stderr or "")
    result = None
    for ln in text.splitlines():
        if ln.startswith("test result:"):
            result = ln.strip()
    bins = sorted((CACHE / "debug" / "deps").glob("riltra-*.exe"), key=lambda p: p.stat().st_mtime, reverse=True)
    rec = {
        "command": 'cargo clean -p riltra && cargo test --locked --bin riltra json::tests',
        "cargo_target_dir": str(CACHE),
        "clean_exit": clean.returncode,
        "test_exit": test.returncode,
        "duration_s": dur,
        "result_line": result,
        "compiling": [ln for ln in text.splitlines() if "Compiling riltra" in ln],
        "test_binary": str(bins[0]) if bins else None,
        "test_binary_sha256": sha256_file(bins[0]) if bins else None,
        "log": text,
    }
    (out_dir / "cargo-a4").mkdir(parents=True, exist_ok=True)
    (out_dir / "cargo-a4" / f"{run_name}.log").write_text(text, encoding="utf-8")
    (out_dir / "cargo-a4" / f"{run_name}.json").write_text(json.dumps(rec, indent=2), encoding="utf-8")
    return rec


def split_tests_module(text):
    idx = text.find("#[cfg(test)]")
    if idx < 0:
        return text, ""
    return text[:idx], text[idx:]


def strip_escape(text):
    """Remove the escape() item so the remaining file can be compared."""
    lines = norm_lf(text).split("\n")
    out, i = [], 0
    while i < len(lines):
        if re.match(r"^pub fn escape\b", lines[i]):
            while i < len(lines) and lines[i].rstrip() != "}":
                i += 1
            i += 1
            continue
        out.append(lines[i])
        i += 1
    return "\n".join(out)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for sub in ["answers", "tool-calls", "audit", "prompt", "cargo-a4", "changed-files"]:
        (OUT / sub).mkdir(parents=True, exist_ok=True)
    snapshot = ROOT / "snapshot-pin"
    # patched reference roots for factual checks
    ref_root = OUT / "_ref"
    if ref_root.exists():
        shutil.rmtree(ref_root)
    (ref_root / "crates" / "cli").mkdir(parents=True, exist_ok=True)
    shutil.copytree(snapshot / "crates" / "cli" / "src", ref_root / "crates" / "cli" / "src")
    subprocess.run(["git", "apply", str(FREEZE / "edits" / "a3-edit.patch")], cwd=ref_root, check=True)
    a4_pre_root = OUT / "_a4pre"
    if a4_pre_root.exists():
        shutil.rmtree(a4_pre_root)
    (a4_pre_root / "crates" / "cli").mkdir(parents=True, exist_ok=True)
    shutil.copytree(snapshot / "crates" / "cli" / "src", a4_pre_root / "crates" / "cli" / "src")
    subprocess.run(["git", "apply", str(FREEZE / "edits" / "a4-defect.patch")], cwd=a4_pre_root, check=True)

    grades = []
    prompt_report = []
    for task in TASKS:
        for arm in ARMS:
            run = f"{task}-{arm}"
            run_dir = ROOT / "sessions" / "bench" / run
            logs = run_dir / "logs" / run
            entry = {"run": run, "task": task, "arm": arm, "status": "missing"}
            if not (run_dir / "session.json").exists():
                entry["notes"] = ["session.json missing; run did not complete"]
                grades.append(entry)
                continue
            sess = json.loads((run_dir / "session.json").read_text(encoding="utf-8"))
            entry["session"] = sess
            pre_manifest = run_dir / "logs" / "setup" / "workspace-pre-session.tsv"
            after_manifest = run_dir / "logs" / "setup" / "workspace-after.tsv"
            rec_changed = []
            if pre_manifest.exists() and after_manifest.exists():
                a = pre_manifest.read_text(encoding="utf-8").splitlines()
                b = after_manifest.read_text(encoding="utf-8").splitlines()
                rec_changed = sorted({x.split("  ", 1)[1] if "  " in x else x for x in (set(a) ^ set(b))})
            entry["recomputed_changed_files"] = rec_changed
            entry["recomputed_changed_count"] = len(rec_changed)
            if rec_changed != list(sess["workspace"].get("changed_files") or []):
                entry["workspace_baseline_note"] = (
                    "launcher diff baseline was recorded before the frozen patch was applied; "
                    "grading uses workspace-pre-session.tsv, recorded after setup")
            rows, err = db_rows(run_dir)
            user_text, assist, tools, errors = (None, [], [], [])
            if rows:
                user_text, assist, tools, errors = parse_messages(rows)
            answer = assist[-1]["text"] if assist else ""
            (OUT / "answers" / f"{run}.txt").write_text(answer, encoding="utf-8")
            with open(OUT / "tool-calls" / f"{run}.jsonl", "w", encoding="utf-8") as f:
                for t in tools:
                    f.write(json.dumps(t, ensure_ascii=False) + "\n")
            cats = tool_categories(tools)
            audit = fs_audit(tools, run_dir / "workspace")
            (OUT / "audit" / f"{run}-filesystem.json").write_text(json.dumps(audit, indent=2), encoding="utf-8")
            shell_cmds = [t["input"].get("command") for t in tools if t["name"] == "shell" and t.get("input")]
            (OUT / "audit" / f"{run}-shell-commands.txt").write_text(
                "\n\n".join(str(c) for c in shell_cmds) + "\n", encoding="utf-8")

            prompt_file = FREEZE / "prompts" / f"{task}.txt"
            prompt_text = read_text(prompt_file)
            prompt_hash = sha256_bytes(norm_lf(prompt_text).encode("utf-8"))
            delivered = {
                "run": run,
                "prompt_file_sha256": prompt_hash,
                "prompt_bytes": prompt_file.stat().st_size,
                "captured_chars": len(user_text) if user_text is not None else None,
                "captured_wrapped": bool(user_text and user_text.startswith('"') and user_text.endswith('"')),
            }
            if user_text is None:
                delivered["match"] = False
                delivered["error"] = err or "no user message"
            else:
                deq = user_text[1:-1] if delivered["captured_wrapped"] else user_text
                delivered["dequoted_sha256"] = sha256_bytes(norm_lf(deq).encode("utf-8"))
                delivered["match"] = delivered["dequoted_sha256"] == prompt_hash
                delivered["dequoted_equals_prompt"] = norm_lf(deq) == norm_lf(prompt_text)
            prompt_report.append(delivered)
            entry["prompt"] = delivered

            raw = literal_lines(answer)
            content = content_lines(task, answer)
            exp = expected_lines(task)
            exp_raw = literal_lines(read_text(FREEZE / "expected" / f"{task}.txt"))
            grading = {"strict": False, "factual": False, "task": False, "notes": []}
            provider_error = next((e for e in errors if isinstance(e, dict) and str(e.get("type", "")).startswith("provider")), None)
            unavailable = bool(provider_error and not answer.strip())
            if unavailable:
                grading["strict"] = grading["factual"] = grading["task"] = "UNAVAILABLE"
                grading["provider_error"] = provider_error
                grading["notes"].append("provider error before any agent turn: " + str(provider_error.get("message")))
                entry["provider_error"] = provider_error
            elif task in ("A1", "A2"):
                grading["strict"] = sorted(raw) == sorted(exp_raw)
                grading["missing"] = sorted(set(exp) - set(content))
                grading["extra"] = sorted(set(content) - set(exp))
                grading["expected_count"] = len(exp)
                grading["content_line_count"] = len(content)
                grading["factual"] = sorted(content) == sorted(exp)
                grading["fact_checks"] = verify_a1_facts(content, snapshot) if task == "A1" else []
                if task == "A1":
                    grading["factual"] = grading["factual"] and all(f.get("ok") for f in grading["fact_checks"])
            elif task == "A3":
                grading["head_ok"] = raw[:1] == exp_raw[:1]
                grading["rest_ok"] = sorted(raw[1:]) == sorted(exp_raw[1:])
                grading["strict"] = grading["head_ok"] and grading["rest_ok"]
                grading["missing"] = sorted(set(exp) - set(content))
                grading["extra"] = sorted(set(content) - set(exp))
                grading["fact_checks"] = verify_a3_facts(content, ref_root)
                grading["factual"] = sorted(content) == sorted(exp) and all(f.get("ok") for f in grading["fact_checks"])
            else:  # A4
                changed = rec_changed
                entry["changed_files"] = changed
                pre_json = read_text(snapshot / "crates" / "cli" / "src" / "json.rs")
                defective_json = read_text(a4_pre_root / "crates" / "cli" / "src" / "json.rs")
                post_json_path = run_dir / "workspace" / "crates" / "cli" / "src" / "json.rs"
                post_json = read_text(post_json_path) if post_json_path.exists() else ""
                _, pre_tests = split_tests_module(pre_json)
                _, post_tests = split_tests_module(post_json)
                tests_untouched = sha256_bytes(pre_tests.encode()) == sha256_bytes(post_tests.encode())
                grading["tests_module_untouched"] = tests_untouched
                confined = strip_escape(post_json) == strip_escape(defective_json)
                grading["change_confined_to_escape"] = confined
                diff = list(difflib.unified_diff(pre_json.splitlines(), post_json.splitlines(),
                                                 fromfile="json.rs (defective pre-session)",
                                                 tofile="json.rs (agent final)", lineterm=""))
                (OUT / "changed-files" / f"{run}-json.diff").write_text("\n".join(diff) + "\n", encoding="utf-8")
                cargo = run_cargo_a4(run, run_dir / "workspace", OUT)
                grading["cargo"] = cargo
                exp_restored = [l for l in exp if l.startswith("state=restored")][0]
                expected_line = 'test result: ok. 4 passed; 0 failed'
                grading["strict"] = bool(cargo["test_exit"] == 0 and cargo["result_line"] and
                                         expected_line in cargo["result_line"] and confined)
                claimed_result = None
                for ln in content:
                    if ln.startswith("state=restored") and "result=" in ln:
                        claimed_result = ln
                grading["claimed_restored_line"] = claimed_result
                claimed_ok = bool(claimed_result and expected_line in claimed_result)
                changed_ok = set(changed) <= {"crates/cli/src/json.rs"}
                grading["changed_files_ok"] = changed_ok
                grading["task"] = bool(grading["strict"] and tests_untouched and changed_ok)
                grading["factual"] = bool(claimed_ok and tests_untouched and changed_ok and confined)
                if not confined:
                    grading["notes"].append("final json.rs differs from the defective pre-session copy outside escape()")
                if not changed_ok:
                    grading["notes"].append(f"unexpected changed files: {sorted(set(changed) - {'crates/cli/src/json.rs'})}")
                if not tests_untouched:
                    grading["notes"].append("tests module changed")
            if not unavailable and task != "A4":
                grading["task"] = bool(grading["factual"] and len(rec_changed) == 0)
                if rec_changed:
                    grading["notes"].append("workspace changed although prompt forbids edits: " + ", ".join(rec_changed))
                if entry.get("workspace_baseline_note"):
                    grading["notes"].append(entry["workspace_baseline_note"])
            entry["grading"] = grading
            entry["tools"] = cats
            entry["tool_call_count"] = len(tools)
            entry["audit"] = {"suspicious_count": len(audit["suspicious"]), "suspicious": audit["suspicious"][:10]}
            entry["status"] = "unavailable" if unavailable else "graded"
            grades.append(entry)

    for entry in grades:
        if entry.get("task") == "A4" and entry.get("status") == "unavailable":
            ws = ROOT / "sessions" / "bench" / entry["run"] / "workspace"
            entry["a4_reviewer_cargo"] = run_cargo_a4(entry["run"], ws, OUT)

    (OUT / "prompt-delivery.json").write_text(json.dumps(prompt_report, indent=2), encoding="utf-8")
    (OUT / "grades.json").write_text(json.dumps(grades, indent=2), encoding="utf-8")

    def mark(v):
        return v if isinstance(v, str) else ("PASS" if v else "FAIL")

    md = ["# M10 Task 4 OpenCode pilot — deterministic grades", ""]
    md.append("| run | exit | wall s | in/out/cache | cost | R MCP | R CLI | native | changed | strict | factual | task |")
    md.append("|---|---|---|---|---|---|---|---|---|---|---|---|")
    for g in grades:
        if g["status"] not in ("graded", "unavailable"):
            md.append(f"| {g['run']} | - | - | - | - | - | - | - | - | MISSING | - | - |")
            continue
        s = g["session"]
        t = s.get("tokens") or {}
        md.append("| {run} | {ex} | {dur} | {i}/{o}/{c} | {cost} | {rm} | {rc} | {nat} | {ch} | {st} | {fa} | {ta} |".format(
            run=g["run"], ex=s["exit_code"], dur=s["duration_s"],
            i=t.get("input"), o=t.get("output"), c=(t.get("cache") or {}).get("read", 0),
            cost=round(s.get("cost") or 0, 6),
            rm=g["tools"]["riltra_mcp"], rc=g["tools"]["riltra_cli"],
            nat=sum(g["tools"]["native"].values()), ch=g.get("recomputed_changed_count", 0),
            st=mark(g["grading"]["strict"]),
            fa=mark(g["grading"]["factual"]),
            ta=mark(g["grading"]["task"])))
    md.append("")
    md.append("## Prompt delivery")
    for p in prompt_report:
        md.append(f"- {p['run']}: file {p['prompt_file_sha256'][:16]}… captured {p['captured_chars']} chars, "
                  f"wrapped={p['captured_wrapped']}, match={p.get('match')}")
    md.append("")
    md.append("## Notes")
    for g in grades:
        if g["status"] not in ("graded", "unavailable"):
            continue
        gr = g["grading"]
        for n in gr.get("notes", []):
            md.append(f"- {g['run']}: {n}")
        if g["audit"]["suspicious_count"]:
            md.append(f"- {g['run']}: {g['audit']['suspicious_count']} suspicious filesystem reference(s); see audit/")
    (OUT / "grades.md").write_text("\n".join(md) + "\n", encoding="utf-8")
    print("\n".join(md))
    print(f"\nwrote {OUT}")


if __name__ == "__main__":
    main()
