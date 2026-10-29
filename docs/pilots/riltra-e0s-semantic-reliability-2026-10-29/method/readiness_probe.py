# SPDX-License-Identifier: Apache-2.0
"""E0-S diagnosis: when does rust-analyzer publish diagnostics for the opened file?

This is a supplementary probe, not one of the counted E0-S queries. It follows
the sequence of the riltra LSP client (crates/cli/src/lsp.rs): the server is
started with no arguments and no extra environment, its working directory is
the canonical repository root, `initialize` sends empty client capabilities,
then `initialized`, then `textDocument/didOpen` for src/main.rs. The client then
waits for the first `textDocument/publishDiagnostics` for that file (capped at
15 seconds), and only then sends `textDocument/definition` at 0-based (0, 3).
The probe records every message with a time stamp. It uses no riltra code and
writes no riltra store.

Usage:
  python readiness_probe.py --rust-analyzer <exe> --fixture <dir> --label <name> [--target-present]
Output: one JSON object on stdout. Repository roots are written as <ROOT>.
"""

import argparse
import json
import os
import queue
import subprocess
import threading
import time
from pathlib import Path
from urllib.parse import unquote

WAIT_CAP_S = 15.0  # crates/cli/src/lsp.rs, await_analysis
DEFINITION_CAP_S = 60.0


def file_uri(path):
    # Same shape as riltra's file_uri: backslashes to slashes, no percent-encoding.
    return "file:///" + str(path).replace("\\", "/").lstrip("/")


def send(proc, message):
    body = json.dumps(message).encode("utf-8")
    proc.stdin.write(b"Content-Length: %d\r\n\r\n" % len(body) + body)
    proc.stdin.flush()


def pump(stream, out):
    while True:
        length = None
        while True:
            line = stream.readline()
            if not line:
                out.put(None)
                return
            line = line.strip()
            if not line:
                break
            if line.lower().startswith(b"content-length:"):
                length = int(line.split(b":", 1)[1])
        if length is None:
            continue
        out.put(json.loads(stream.read(length).decode("utf-8")))


def drain(stream):
    while stream.read(4096):
        pass


def sanitize_uri(uri, root_uri):
    """Replace the repository root and the user profile in a URI with placeholders."""
    if not uri:
        return uri
    decoded = unquote(uri)
    home = unquote(file_uri(Path.home()))
    if decoded.startswith(unquote(root_uri)):
        return "<ROOT>" + decoded[len(unquote(root_uri)):]
    if decoded.lower().startswith(home.lower()):
        return "<USERHOME>" + decoded[len(home):]
    return decoded


def probe(ra, fixture, label, target_present):
    root = Path(os.path.realpath(fixture))
    root_uri = file_uri(root)
    main_path = root / "src" / "main.rs"
    main_uri = file_uri(main_path)
    text = main_path.read_text(encoding="utf-8")
    proc = subprocess.Popen([ra], cwd=str(root), stdin=subprocess.PIPE,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    inbox = queue.Queue()
    threading.Thread(target=pump, args=(proc.stdout, inbox), daemon=True).start()
    threading.Thread(target=drain, args=(proc.stderr,), daemon=True).start()
    t0 = time.perf_counter()

    def now_ms():
        return round((time.perf_counter() - t0) * 1000, 1)

    events = []
    publish = []
    server_requests = []
    progress = 0

    def wait_for(predicate, cap_s):
        deadline = time.perf_counter() + cap_s
        while True:
            left = deadline - time.perf_counter()
            if left <= 0:
                return None
            try:
                message = inbox.get(timeout=min(left, 0.5))
            except queue.Empty:
                continue
            if message is None:
                return None
            handle(message)
            if predicate(message):
                return message

    def handle(message):
        nonlocal progress
        method = message.get("method")
        if method is not None and "id" in message:
            # Server-to-client request: answer with null, as the client does for unknown requests.
            server_requests.append({"t_ms": now_ms(), "method": method})
            send(proc, {"jsonrpc": "2.0", "id": message["id"], "result": None})
            return "request"
        if method == "textDocument/publishDiagnostics":
            uri = message.get("params", {}).get("uri", "")
            publish.append({"t_ms": now_ms(), "uri": sanitize_uri(uri, root_uri),
                            "exact_match": uri == main_uri,
                            "decoded_match": unquote(uri).lower() == unquote(main_uri).lower(),
                            "diagnostics": len(message.get("params", {}).get("diagnostics", []))})
            return "publish"
        if method == "$/progress":
            progress += 1
            return "progress"
        if method is not None:
            events.append({"t_ms": now_ms(), "method": method})
        return "other"

    send(proc, {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
        "processId": os.getpid(), "rootUri": root_uri, "capabilities": {},
        "workspaceFolders": [{"uri": root_uri, "name": "root"}]}})
    init = wait_for(lambda m: m.get("id") == 1, 30.0)
    init_ms = now_ms()
    server_version = (init or {}).get("result", {}).get("serverInfo", {}).get("version", "")
    send(proc, {"jsonrpc": "2.0", "method": "initialized", "params": {}})
    send(proc, {"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
        "textDocument": {"uri": main_uri, "languageId": "rust", "version": 1, "text": text}}})
    opened_ms = now_ms()
    readiness = wait_for(
        lambda m: m.get("method") == "textDocument/publishDiagnostics"
        and unquote(m.get("params", {}).get("uri", "")).lower() == unquote(main_uri).lower(),
        WAIT_CAP_S)
    ready_ms = now_ms()
    send(proc, {"jsonrpc": "2.0", "id": 2, "method": "textDocument/definition", "params": {
        "textDocument": {"uri": main_uri}, "position": {"line": 0, "character": 3}}})
    answer = wait_for(lambda m: m.get("id") == 2, DEFINITION_CAP_S)
    answer_ms = now_ms()
    locations = answer.get("result") if answer else None
    if isinstance(locations, dict):
        locations = [locations]
    definition = [
        {"line": loc.get("range", {}).get("start", {}).get("line"),
         "character": loc.get("range", {}).get("start", {}).get("character"),
         "uri_is_main": loc.get("uri") == main_uri}
        for loc in (locations or [])
    ]
    try:
        proc.stdin.close()
    except OSError:
        pass
    try:
        proc.wait(timeout=10)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait()
    return {
        "label": label,
        "target_present_before": target_present,
        "server_version": server_version,
        "initialize_ms": init_ms,
        "opened_ms": opened_ms,
        "readiness_seen": readiness is not None,
        "readiness_ms": ready_ms,
        "readiness_within_cap": readiness is not None and ready_ms <= WAIT_CAP_S * 1000 + 50,
        "definition_answer_ms": answer_ms,
        "definition_locations": definition,
        "publish_events": publish[:12],
        "publish_count": len(publish),
        "progress_messages": progress,
        "server_requests": server_requests[:12],
        "other_notifications": events[:12],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--rust-analyzer", required=True)
    parser.add_argument("--fixture", required=True)
    parser.add_argument("--label", required=True)
    parser.add_argument("--target-present", action="store_true")
    args = parser.parse_args()
    result = probe(args.rust_analyzer, args.fixture, args.label, args.target_present)
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
