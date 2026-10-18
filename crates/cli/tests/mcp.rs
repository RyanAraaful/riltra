// SPDX-License-Identifier: Apache-2.0
//! M8 CP1/CP3: `riltra mcp` stdio server. Each test spawns the real binary
//! with an isolated `RILTRA_HOME` and speaks newline-delimited JSON-RPC 2.0
//! over stdin/stdout (the MCP stdio transport framing). CP3 adds the three
//! read-only tools plus the F1 storage-safety regressions, all through the
//! real MCP read path.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

const TIMEOUT: Duration = Duration::from_secs(15);

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn home(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut p = std::env::temp_dir();
    p.push(format!("riltra-mcp-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

struct Session {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
}

fn spawn(home: &Path) -> Session {
    let mut child = Command::new(env!("CARGO_BIN_EXE_riltra"))
        .arg("mcp")
        .env("RILTRA_HOME", home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn riltra mcp");
    let stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            match line {
                Ok(l) => {
                    if tx.send(l).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    Session {
        child,
        stdin: Some(stdin),
        lines: rx,
    }
}

impl Session {
    fn send(&mut self, msg: &str) {
        let stdin = self.stdin.as_mut().expect("stdin open");
        stdin.write_all(msg.as_bytes()).expect("write request");
        stdin.write_all(b"\n").expect("write newline");
        stdin.flush().expect("flush");
    }

    /// Next stdout line, which must be a JSON-RPC 2.0 message (stdout is
    /// protocol-only by CP1 contract).
    fn recv(&self) -> serde_json::Value {
        let line = self
            .lines
            .recv_timeout(TIMEOUT)
            .expect("timed out waiting for a stdout line");
        let v: serde_json::Value = serde_json::from_str(&line)
            .unwrap_or_else(|e| panic!("non-JSON on MCP stdout: {line:?} ({e})"));
        assert_eq!(v["jsonrpc"], "2.0", "not JSON-RPC 2.0: {line}");
        v
    }

    /// Close stdin (client disconnect) and wait for a bounded, clean exit.
    fn finish(&mut self) -> (i32, String) {
        drop(self.stdin.take());
        let deadline = Instant::now() + TIMEOUT;
        let code = loop {
            if let Some(st) = self.child.try_wait().expect("try_wait") {
                break st.code().unwrap_or(-1);
            }
            assert!(
                Instant::now() < deadline,
                "riltra mcp did not exit after stdin EOF"
            );
            std::thread::sleep(Duration::from_millis(50));
        };
        let mut err = String::new();
        if let Some(mut e) = self.child.stderr.take() {
            let _ = e.read_to_string(&mut err);
        }
        assert!(!err.contains("panicked"), "server panicked: {err}");
        (code, err)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn assert_home_untouched(home: &Path) {
    let entries: Vec<_> = std::fs::read_dir(home)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert!(
        entries.is_empty(),
        "MCP startup must not touch storage: {entries:?}"
    );
}

fn initialize_request() -> &'static str {
    r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"riltra-mcp-test","version":"0.0.0"}}}"#
}

/// Spawn a session and complete the initialize handshake.
fn ready(cfg: &Path) -> Session {
    let mut s = spawn(cfg);
    s.send(initialize_request());
    let v = s.recv();
    assert_eq!(v["id"], 1, "initialize response: {v}");
    s.send(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
    s
}

#[test]
fn mcp_initialize_handshake_reports_identity() {
    let h = home("init");
    let mut s = spawn(&h);
    s.send(initialize_request());
    let v = s.recv();
    assert_eq!(v["id"], 1);
    assert_eq!(v["result"]["serverInfo"]["name"], "riltra");
    assert_eq!(
        v["result"]["serverInfo"]["version"],
        env!("CARGO_PKG_VERSION")
    );
    assert_eq!(v["result"]["protocolVersion"], "2025-11-25");
    assert!(
        v["result"]["capabilities"]["tools"].is_object(),
        "tools capability missing: {v}"
    );
    let (code, _err) = s.finish();
    assert_eq!(code, 0, "clean disconnect must exit 0");
    assert_home_untouched(&h);
}

#[test]
fn mcp_tools_list_advertises_exactly_three_readonly_tools() {
    let h = home("tools");
    let mut s = ready(&h);
    s.send(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#);
    let v = s.recv();
    assert_eq!(v["id"], 2);
    let tools = v["result"]["tools"].as_array().expect("tools array");
    let mut names: Vec<&str> = tools
        .iter()
        .map(|t| t["name"].as_str().expect("tool name"))
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "riltra_code_search",
            "riltra_repo_list",
            "riltra_repo_status"
        ],
        "CP3 advertises exactly the three read-only tools"
    );
    for t in tools {
        let name = t["name"].as_str().unwrap();
        assert!(
            t["description"].as_str().is_some_and(|d| !d.is_empty()),
            "{name}: description required"
        );
        let ann = &t["annotations"];
        assert_eq!(ann["readOnlyHint"], true, "{name}: readOnlyHint");
        assert_eq!(ann["openWorldHint"], false, "{name}: openWorldHint");
        assert_eq!(ann["idempotentHint"], true, "{name}: idempotentHint");
        let schema = &t["inputSchema"];
        assert_eq!(schema["type"], "object", "{name}: schema type");
    }
    let find = |n: &str| tools.iter().find(|t| t["name"] == n).unwrap();
    let list_schema = &find("riltra_repo_list")["inputSchema"];
    assert!(
        list_schema["properties"]
            .as_object()
            .is_none_or(|p| p.is_empty()),
        "repo_list takes no inputs: {list_schema}"
    );
    let status_schema = &find("riltra_repo_status")["inputSchema"];
    assert_eq!(status_schema["required"], json!(["repo"]));
    assert!(status_schema["properties"]["repo"].is_object());
    let search_schema = &find("riltra_code_search")["inputSchema"];
    let required: Vec<&str> = search_schema["required"]
        .as_array()
        .expect("required array")
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(required.contains(&"repo") && required.contains(&"query"));
    for key in ["repo", "query", "limit", "offset"] {
        assert!(
            search_schema["properties"][key].is_object(),
            "code_search missing property {key}: {search_schema}"
        );
    }
    let (code, _) = s.finish();
    assert_eq!(code, 0, "clean disconnect must exit 0");
    assert_home_untouched(&h);
}

#[test]
fn mcp_invalid_input_is_handled_without_panic() {
    let h = home("bad");
    let mut s = spawn(&h);
    s.send("this is not json");
    // Either a JSON-RPC error response arrives, or the message is dropped
    // before shutdown; both must be clean, bounded, and panic-free.
    if let Ok(line) = s.lines.recv_timeout(Duration::from_secs(3)) {
        let v: serde_json::Value = serde_json::from_str(&line).expect("stdout must stay JSON-RPC");
        assert_eq!(v["jsonrpc"], "2.0");
        assert!(v.get("error").is_some(), "expected a JSON-RPC error: {v}");
    }
    let (code, _) = s.finish();
    assert!(code == 0 || code == 3, "unexpected exit code {code}");
    assert_home_untouched(&h);
}

#[test]
fn mcp_stdin_eof_exits_without_hang_or_storage() {
    let h = home("eof");
    let mut s = spawn(&h);
    let (code, _) = s.finish();
    assert!(code == 0 || code == 3, "unexpected exit code {code}");
    assert_home_untouched(&h);
}

// --- CP3 helpers ------------------------------------------------------------

fn run_cli(cfg: &Path, args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_riltra"))
        .args(args)
        .env("RILTRA_HOME", cfg)
        .output()
        .expect("spawn riltra");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn cli_json(cfg: &Path, args: &[&str]) -> Value {
    let (code, out, err) = run_cli(cfg, args);
    assert_eq!(code, 0, "cli {args:?} failed: {err}");
    serde_json::from_str(&out)
        .unwrap_or_else(|e| panic!("cli {args:?} emitted invalid JSON: {e}\n{out}"))
}

fn init_repo(cfg: &Path, dir: &Path, name: &str) {
    let (code, _out, err) = run_cli(
        cfg,
        &["init", "--repo", &dir.to_string_lossy(), "--name", name],
    );
    assert_eq!(code, 0, "init {name}: {err}");
}

fn indexed_repo(cfg: &Path, dir: &Path, name: &str) {
    init_repo(cfg, dir, name);
    let (code, _out, err) = run_cli(cfg, &["index", "--repo", name]);
    assert_eq!(code, 0, "index {name}: {err}");
}

fn call_tool(s: &mut Session, id: i64, name: &str, args: &Value) -> Value {
    s.send(
        &json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {"name": name, "arguments": args}
        })
        .to_string(),
    );
    let v = s.recv();
    assert_eq!(v["id"], id, "response for {name}: {v}");
    assert!(
        v.get("error").is_none(),
        "tool calls must answer with a result, not a protocol error: {v}"
    );
    v["result"].clone()
}

fn is_error(result: &Value) -> bool {
    result["isError"].as_bool().unwrap_or(false)
}

/// The tool result's structured document; the text payload must parse to
/// exactly the same JSON (rmcp `structured`/`structured_error` contract).
fn doc(result: &Value) -> Value {
    let text = result["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("tool result has no content text: {result}"));
    let parsed: Value = serde_json::from_str(text)
        .unwrap_or_else(|e| panic!("tool content is not valid JSON: {e}\n{text}"));
    let structured = result["structuredContent"].clone();
    assert_eq!(parsed, structured, "text and structuredContent must agree");
    structured
}

/// Files under `dir` (relative paths, sorted) for no-creation assertions.
fn listing(dir: &Path) -> Vec<String> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                out.push(
                    path.strip_prefix(base)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

fn db_path(cfg: &Path, name: &str) -> PathBuf {
    cfg.join("index").join(format!("{name}.db"))
}

fn user_version(path: &Path) -> u32 {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap()
}

fn set_user_version(path: &Path, version: u32) {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute(&format!("PRAGMA user_version = {version}"), [])
        .unwrap();
}

fn schema_dump(path: &Path) -> String {
    let conn = rusqlite::Connection::open(path).unwrap();
    let mut statement = conn
        .prepare("SELECT type, name, sql FROM sqlite_master ORDER BY type, name")
        .unwrap();
    let rows = statement
        .query_map([], |r| {
            Ok(format!(
                "{:?}|{:?}|{:?}",
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?
            ))
        })
        .unwrap();
    rows.map(|r| r.unwrap()).collect::<Vec<_>>().join("\n")
}

fn try_dir_symlink(target: &Path, link: &Path) -> bool {
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(target, link).is_ok()
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
}

fn try_file_symlink(target: &Path, link: &Path) -> bool {
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(target, link).is_ok()
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
}

fn write_search_fixture(root: &Path) {
    std::fs::create_dir_all(root.join("sp ace")).unwrap();
    std::fs::write(
        root.join("a.py"),
        "def alpha_one():\n    pass\n\ndef alpha_two():\n    return alpha_one()\n",
    )
    .unwrap();
    std::fs::write(root.join("b.py"), "def beta():\n    pass\n").unwrap();
    std::fs::write(root.join("sp ace/c.py"), "def alpha_three():\n    pass\n").unwrap();
}

const V1_TABLES: &str = "CREATE TABLE snapshots (id INTEGER PRIMARY KEY AUTOINCREMENT,
     repo TEXT NOT NULL, root TEXT NOT NULL, created_unix INTEGER NOT NULL,
     head TEXT, branch TEXT, dirty INTEGER NOT NULL, untracked INTEGER NOT NULL,
     parser_version INTEGER NOT NULL, grammar_versions TEXT NOT NULL,
     config_fingerprint TEXT NOT NULL, pre_fingerprint TEXT NOT NULL,
     post_fingerprint TEXT NOT NULL, status_at_write TEXT NOT NULL);
     INSERT INTO snapshots (repo, root, created_unix, head, branch, dirty, untracked, parser_version, grammar_versions, config_fingerprint, pre_fingerprint, post_fingerprint, status_at_write)
     VALUES ('r', 'C:\\r', 1, 'abc', 'main', 0, 0, 1, 'g', 'c', 'p', 'p', 'fresh');
     PRAGMA user_version = 1;";

// --- T2: repo_list / repo_status -------------------------------------------

#[test]
fn t2_repo_list_matches_cli_and_creates_no_files() {
    let h = home("repolist");
    let r1 = home("repolist-r1");
    let r2 = home("repolist-r2");
    init_repo(&h, &r1, "alpha");
    init_repo(&h, &r2, "beta");
    let before = listing(&h);
    let mut s = ready(&h);
    let result = call_tool(&mut s, 2, "riltra_repo_list", &json!({}));
    assert!(!is_error(&result), "{result}");
    let d = doc(&result);
    assert_eq!(d, cli_json(&h, &["repo", "list"]));
    assert_eq!(d["repos"].as_array().unwrap().len(), 2);
    let (code, _) = s.finish();
    assert_eq!(code, 0);
    assert_eq!(
        listing(&h),
        before,
        "repo_list must not create or modify files"
    );
}

#[test]
fn t2_repo_status_unknown_missing_retargeted_and_no_index() {
    let h = home("rpstat");
    let r = home("rpstat-repo");
    init_repo(&h, &r, "r");
    let mut s = ready(&h);
    // Unknown repository.
    let res = call_tool(&mut s, 2, "riltra_repo_status", &json!({"repo": "nope"}));
    assert!(is_error(&res), "{res}");
    assert_eq!(
        doc(&res),
        json!({"kind": "not_registered", "message": "unknown repository: 'nope'"})
    );
    // Registered but its root is gone.
    std::fs::remove_dir_all(&r).unwrap();
    let res = call_tool(&mut s, 3, "riltra_repo_status", &json!({"repo": "r"}));
    assert!(is_error(&res), "{res}");
    let d = doc(&res);
    assert_eq!(d["kind"], "not_registered");
    let msg = d["message"].as_str().unwrap();
    assert!(msg.starts_with("registered root is missing: '"), "{msg}");
    assert!(msg.contains("rpstat-repo"), "{msg}");
    // Registered but retargeted through a symlink at the recorded path.
    let r2 = home("rpstat-original");
    let moved = home("rpstat-target");
    init_repo(&h, &r2, "r2");
    std::fs::remove_dir(&moved).unwrap();
    std::fs::rename(&r2, &moved).unwrap();
    if try_dir_symlink(&moved, &r2) {
        let res = call_tool(&mut s, 4, "riltra_repo_status", &json!({"repo": "r2"}));
        assert!(is_error(&res), "{res}");
        let d = doc(&res);
        assert_eq!(d["kind"], "not_registered");
        assert!(
            d["message"]
                .as_str()
                .unwrap()
                .contains("moved or retargeted"),
            "{d}"
        );
    } else {
        eprintln!("SKIP retargeted-root case: symlink creation unavailable");
    }
    // Fresh registration without an index: no-index document, no creation.
    let r3 = home("rpstat-fresh");
    init_repo(&h, &r3, "r3");
    let before = listing(&h);
    let res = call_tool(&mut s, 5, "riltra_repo_status", &json!({"repo": "r3"}));
    assert!(!is_error(&res), "{res}");
    let d = doc(&res);
    let mut without_index = d.clone();
    without_index
        .as_object_mut()
        .unwrap()
        .remove("index_status")
        .unwrap();
    assert_eq!(
        without_index,
        cli_json(&h, &["repo", "status", "--repo", "r3"])
    );
    assert_eq!(d["index_status"]["freshness"], "unknown");
    assert_eq!(d["index_status"]["reasons"], json!(["no-index"]));
    assert_eq!(d["index_status"]["fts"], false);
    assert!(d["index_status"]["snapshot"].is_null());
    let (code, _) = s.finish();
    assert_eq!(code, 0);
    assert_eq!(listing(&h), before, "repo_status must not create files");
}

#[test]
fn t2_repo_status_composes_repo_and_index_documents_like_cli() {
    let h = home("rpcompose");
    let r = home("rpcompose-repo");
    write_search_fixture(&r);
    indexed_repo(&h, &r, "r");
    let mut s = ready(&h);
    let res = call_tool(&mut s, 2, "riltra_repo_status", &json!({"repo": "r"}));
    assert!(!is_error(&res), "{res}");
    let d = doc(&res);
    let mut without_index = d.clone();
    without_index
        .as_object_mut()
        .unwrap()
        .remove("index_status")
        .unwrap();
    assert_eq!(
        without_index,
        cli_json(&h, &["repo", "status", "--repo", "r"])
    );
    let cli_status = cli_json(&h, &["index", "status", "--repo", "r"]);
    assert_eq!(d["index_status"], cli_status);
    assert_eq!(d["index_status"]["freshness"], "fresh");
    assert_eq!(d["index_status"]["fts"], true);
    let (code, _) = s.finish();
    assert_eq!(code, 0);
}

// --- T3: code_search --------------------------------------------------------

#[test]
fn t3_code_search_matches_cli_including_paging_and_escaping() {
    let h = home("csrch");
    let r = home("csrch-repo");
    write_search_fixture(&r);
    indexed_repo(&h, &r, "r");
    let mut s = ready(&h);
    let res = call_tool(
        &mut s,
        2,
        "riltra_code_search",
        &json!({"repo": "r", "query": "alpha"}),
    );
    assert!(!is_error(&res), "{res}");
    let d = doc(&res);
    assert_eq!(
        d,
        cli_json(&h, &["search", "--repo", "r", "--symbol", "alpha"])
    );
    assert_eq!(d["total"], 3, "fixture sanity");
    assert_eq!(d["freshness"], "fresh");
    // Paging: first page truncated, last page complete.
    let page1 = doc(&call_tool(
        &mut s,
        3,
        "riltra_code_search",
        &json!({"repo": "r", "query": "alpha", "limit": 2}),
    ));
    assert_eq!(
        page1,
        cli_json(
            &h,
            &["search", "--repo", "r", "--symbol", "alpha", "--limit", "2"]
        )
    );
    assert_eq!(page1["truncated"], true);
    assert_eq!(page1["results"].as_array().unwrap().len(), 2);
    let page2 = doc(&call_tool(
        &mut s,
        4,
        "riltra_code_search",
        &json!({"repo": "r", "query": "alpha", "limit": 2, "offset": 2}),
    ));
    assert_eq!(
        page2,
        cli_json(
            &h,
            &["search", "--repo", "r", "--symbol", "alpha", "--limit", "2", "--offset", "2"]
        )
    );
    assert_eq!(page2["truncated"], false);
    assert_eq!(page2["results"].as_array().unwrap().len(), 1);
    // Empty result set.
    let none = doc(&call_tool(
        &mut s,
        5,
        "riltra_code_search",
        &json!({"repo": "r", "query": "zzz_absent"}),
    ));
    assert_eq!(
        none,
        cli_json(&h, &["search", "--repo", "r", "--symbol", "zzz_absent"])
    );
    assert_eq!(none["total"], 0);
    // Quote/backslash escaping parity of the echoed query.
    let escaped = "a\"b\\c";
    let esc = doc(&call_tool(
        &mut s,
        6,
        "riltra_code_search",
        &json!({"repo": "r", "query": escaped}),
    ));
    assert_eq!(
        esc,
        cli_json(&h, &["search", "--repo", "r", "--symbol", escaped])
    );
    assert_eq!(esc["query"]["symbol"], escaped);
    // Limit 0 mirrors the CLI (accepted; empty page over the real total).
    let zero = doc(&call_tool(
        &mut s,
        7,
        "riltra_code_search",
        &json!({"repo": "r", "query": "alpha", "limit": 0}),
    ));
    assert_eq!(
        zero,
        cli_json(
            &h,
            &["search", "--repo", "r", "--symbol", "alpha", "--limit", "0"]
        )
    );
    let (code, _) = s.finish();
    assert_eq!(code, 0);
}

#[test]
fn t3_code_search_stale_and_root_unreadable_match_cli() {
    let h = home("csstale");
    let r = home("csstale-repo");
    write_search_fixture(&r);
    indexed_repo(&h, &r, "r");
    std::fs::write(r.join("d.py"), "def alpha_four():\n    pass\n").unwrap();
    let mut s = ready(&h);
    let d = doc(&call_tool(
        &mut s,
        2,
        "riltra_code_search",
        &json!({"repo": "r", "query": "alpha"}),
    ));
    assert_eq!(
        d,
        cli_json(&h, &["search", "--repo", "r", "--symbol", "alpha"])
    );
    assert_eq!(d["freshness"], "stale");
    assert!(!d["reasons"].as_array().unwrap().is_empty());
    // Root gone: both sides report the snapshot with `root-unreadable`.
    let moved = home("csstale-target");
    std::fs::remove_dir(&moved).unwrap();
    std::fs::rename(&r, &moved).unwrap();
    let gone = doc(&call_tool(
        &mut s,
        3,
        "riltra_code_search",
        &json!({"repo": "r", "query": "alpha"}),
    ));
    assert_eq!(
        gone,
        cli_json(&h, &["search", "--repo", "r", "--symbol", "alpha"])
    );
    assert_eq!(gone["freshness"], "unknown");
    assert_eq!(gone["reasons"], json!(["root-unreadable"]));
    assert!(!gone["snapshot"].is_null());
    let (code, _) = s.finish();
    assert_eq!(code, 0);
}

#[test]
fn t3_code_search_missing_index_matches_cli_without_creating_storage() {
    let h = home("csnone");
    let r = home("csnone-repo");
    init_repo(&h, &r, "r");
    let before = listing(&h);
    let mut s = ready(&h);
    let d = doc(&call_tool(
        &mut s,
        2,
        "riltra_code_search",
        &json!({"repo": "r", "query": "alpha"}),
    ));
    assert_eq!(d["freshness"], "unknown");
    assert_eq!(d["reasons"], json!(["no-index"]));
    assert!(d["snapshot"].is_null());
    assert_eq!(d["total"], 0);
    assert_eq!(d["results"], json!([]));
    assert_eq!(
        d,
        cli_json(&h, &["search", "--repo", "r", "--symbol", "alpha"])
    );
    let (code, _) = s.finish();
    assert_eq!(code, 0);
    assert_eq!(
        listing(&h),
        before,
        "missing index must not create a database"
    );
}

#[test]
fn t3_code_search_invalid_limit_and_unknown_repo_are_classified() {
    let h = home("csbad");
    let r = home("csbad-repo");
    init_repo(&h, &r, "r");
    let mut s = ready(&h);
    let res = call_tool(
        &mut s,
        2,
        "riltra_code_search",
        &json!({"repo": "r", "query": "x", "limit": 1001}),
    );
    assert!(is_error(&res), "{res}");
    assert_eq!(
        doc(&res),
        json!({"kind": "invalid_input", "message": "--limit exceeds maximum 1000"})
    );
    // CLI parity: same message text, exit code 2.
    let (code, _out, err) = run_cli(
        &h,
        &["search", "--repo", "r", "--symbol", "x", "--limit", "1001"],
    );
    assert_eq!(code, 2);
    assert!(err.contains("--limit exceeds maximum 1000"), "{err}");
    let res = call_tool(
        &mut s,
        3,
        "riltra_code_search",
        &json!({"repo": "nope", "query": "x"}),
    );
    assert!(is_error(&res), "{res}");
    assert_eq!(
        doc(&res),
        json!({"kind": "not_registered", "message": "unknown repository: 'nope'"})
    );
    let (code, _) = s.finish();
    assert_eq!(code, 0);
}

// --- T4 (F1): storage-safety regressions through the real MCP path ----------

#[test]
fn f1_v4_reads_leave_database_unmodified() {
    let h = home("f1-v4");
    let r = home("f1-v4-repo");
    write_search_fixture(&r);
    indexed_repo(&h, &r, "r");
    let db = db_path(&h, "r");
    let version_before = user_version(&db);
    let schema_before = schema_dump(&db);
    let mut s = ready(&h);
    let d = doc(&call_tool(
        &mut s,
        2,
        "riltra_code_search",
        &json!({"repo": "r", "query": "alpha"}),
    ));
    assert_eq!(d["freshness"], "fresh");
    let status = doc(&call_tool(
        &mut s,
        3,
        "riltra_repo_status",
        &json!({"repo": "r"}),
    ));
    assert_eq!(status["index_status"]["freshness"], "fresh");
    let (code, _) = s.finish();
    assert_eq!(code, 0);
    assert_eq!(user_version(&db), version_before);
    assert_eq!(schema_dump(&db), schema_before, "schema must be untouched");
}

#[test]
fn f1_legacy_v1_refused_without_migration_and_still_cli_migratable() {
    let h = home("f1-legacy");
    let r = home("f1-legacy-repo");
    init_repo(&h, &r, "v1repo");
    let db = db_path(&h, "v1repo");
    std::fs::create_dir_all(db.parent().unwrap()).unwrap();
    {
        // Exact M3-era v1 shape (handwritten, never SCHEMA_SQL).
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(V1_TABLES).unwrap();
    }
    let mut s = ready(&h);
    for (id, tool, args) in [
        (
            2,
            "riltra_code_search",
            json!({"repo": "v1repo", "query": "x"}),
        ),
        (3, "riltra_repo_status", json!({"repo": "v1repo"})),
    ] {
        let res = call_tool(&mut s, id, tool, &args);
        assert!(is_error(&res), "{tool}: {res}");
        let d = doc(&res);
        assert_eq!(d["kind"], "schema_outdated", "{tool}: {d}");
        assert!(
            d["message"].as_str().unwrap().contains("migrate"),
            "{tool}: {d}"
        );
    }
    assert_eq!(user_version(&db), 1, "MCP must not migrate");
    // The CLI migration path still upgrades the same file afterwards.
    let (code, out, err) = run_cli(&h, &["index", "status", "--repo", "v1repo"]);
    assert_eq!(code, 0, "{out} {err}");
    assert_eq!(user_version(&db), 4, "CLI migration path intact");
    // And the MCP read path works again on the migrated database.
    let d = doc(&call_tool(
        &mut s,
        4,
        "riltra_code_search",
        &json!({"repo": "v1repo", "query": "x"}),
    ));
    assert_eq!(d["total"], 0);
    assert_eq!(d["freshness"], "stale");
    assert!(
        d["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r == "root-moved"),
        "{d}"
    );
    let (code, _) = s.finish();
    assert_eq!(code, 0);
}

#[test]
fn f1_flipped_v2_v3_refused_unchanged() {
    let h = home("f1-flip23");
    let r = home("f1-flip23-repo");
    write_search_fixture(&r);
    indexed_repo(&h, &r, "r");
    let db = db_path(&h, "r");
    let schema_before = schema_dump(&db);
    let mut s = ready(&h);
    for (id, version) in (2..).zip([2u32, 3]) {
        set_user_version(&db, version);
        let res = call_tool(
            &mut s,
            id,
            "riltra_code_search",
            &json!({"repo": "r", "query": "alpha"}),
        );
        assert!(is_error(&res), "v{version}: {res}");
        assert_eq!(doc(&res)["kind"], "schema_outdated", "v{version}");
        assert_eq!(
            user_version(&db),
            version,
            "MCP must not migrate v{version}"
        );
    }
    assert_eq!(schema_dump(&db), schema_before);
    let (code, _) = s.finish();
    assert_eq!(code, 0);
}

#[test]
fn f1_future_schema_is_refused_as_schema_ahead() {
    let h = home("f1-ahead");
    let r = home("f1-ahead-repo");
    write_search_fixture(&r);
    indexed_repo(&h, &r, "r");
    let db = db_path(&h, "r");
    set_user_version(&db, 99);
    let mut s = ready(&h);
    for (id, tool, args) in [
        (
            2,
            "riltra_code_search",
            json!({"repo": "r", "query": "alpha"}),
        ),
        (3, "riltra_repo_status", json!({"repo": "r"})),
    ] {
        let res = call_tool(&mut s, id, tool, &args);
        assert!(is_error(&res), "{tool}: {res}");
        let d = doc(&res);
        assert_eq!(d["kind"], "schema_ahead", "{tool}: {d}");
        assert!(
            d["message"]
                .as_str()
                .unwrap()
                .contains("newer than supported v4"),
            "{tool}: {d}"
        );
    }
    assert_eq!(user_version(&db), 99, "refused, never guessed");
    let (code, _) = s.finish();
    assert_eq!(code, 0);
}

#[test]
fn f1_corrupt_database_is_store_error_and_byte_untouched() {
    let h = home("f1-corrupt");
    let r = home("f1-corrupt-repo");
    write_search_fixture(&r);
    indexed_repo(&h, &r, "r");
    let db = db_path(&h, "r");
    for sidecar in ["-wal", "-shm"] {
        let _ = std::fs::remove_file(PathBuf::from(format!("{}{sidecar}", db.display())));
    }
    std::fs::write(&db, b"definitely not a sqlite database").unwrap();
    let mut s = ready(&h);
    let res = call_tool(
        &mut s,
        2,
        "riltra_code_search",
        &json!({"repo": "r", "query": "alpha"}),
    );
    assert!(is_error(&res), "{res}");
    let d = doc(&res);
    assert_eq!(d["kind"], "store_error");
    let mcp_message = d["message"].as_str().unwrap();
    assert!(mcp_message.contains("not a readable database"), "{d}");
    let res = call_tool(&mut s, 3, "riltra_repo_status", &json!({"repo": "r"}));
    assert_eq!(doc(&res)["kind"], "store_error");
    // CLI parity of the classification text.
    let (code, _out, err) = run_cli(&h, &["search", "--repo", "r", "--symbol", "alpha"]);
    assert_eq!(code, 3);
    let cli_message = err.trim().strip_prefix("error: ").unwrap();
    assert_eq!(mcp_message, cli_message);
    let (code, _) = s.finish();
    assert_eq!(code, 0);
    assert_eq!(
        std::fs::read(&db).unwrap(),
        b"definitely not a sqlite database",
        "the corrupt file must be left untouched"
    );
}

#[test]
fn f1_directory_at_db_path_fails_closed() {
    let h = home("f1-dir");
    let r = home("f1-dir-repo");
    init_repo(&h, &r, "r");
    let db = db_path(&h, "r");
    std::fs::create_dir_all(&db).unwrap();
    let mut s = ready(&h);
    let res = call_tool(
        &mut s,
        2,
        "riltra_code_search",
        &json!({"repo": "r", "query": "x"}),
    );
    assert!(is_error(&res), "{res}");
    let d = doc(&res);
    assert_eq!(d["kind"], "store_error");
    assert!(
        d["message"]
            .as_str()
            .unwrap()
            .contains("not a regular file"),
        "{d}"
    );
    let res = call_tool(&mut s, 3, "riltra_repo_status", &json!({"repo": "r"}));
    assert_eq!(doc(&res)["kind"], "store_error");
    let (code, _) = s.finish();
    assert_eq!(code, 0);
    assert!(db.is_dir(), "fail-closed must not replace the directory");
}

#[test]
fn f1_symlinked_db_path_fails_closed() {
    let h = home("f1-symlink");
    let r = home("f1-symlink-repo");
    init_repo(&h, &r, "r");
    let db = db_path(&h, "r");
    std::fs::create_dir_all(db.parent().unwrap()).unwrap();
    let target = db.with_extension("real");
    std::fs::write(&target, b"not sqlite either").unwrap();
    if !try_file_symlink(&target, &db) {
        eprintln!("SKIP symlinked-db case: symlink creation unavailable");
        return;
    }
    let mut s = ready(&h);
    let res = call_tool(
        &mut s,
        2,
        "riltra_code_search",
        &json!({"repo": "r", "query": "x"}),
    );
    assert!(is_error(&res), "{res}");
    let d = doc(&res);
    assert_eq!(d["kind"], "store_error");
    assert!(d["message"].as_str().unwrap().contains("symlink"), "{d}");
    let (code, _) = s.finish();
    assert_eq!(code, 0);
    assert_eq!(std::fs::read(&target).unwrap(), b"not sqlite either");
}

#[test]
fn f1_concurrent_version_flip_is_classified_and_never_migrates() {
    let h = home("f1-flip");
    let r = home("f1-flip-repo");
    write_search_fixture(&r);
    indexed_repo(&h, &r, "r");
    let db = db_path(&h, "r");
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flipper = {
        let db = db.clone();
        let stop = stop.clone();
        std::thread::spawn(move || {
            let conn = rusqlite::Connection::open(&db).unwrap();
            conn.execute_batch("PRAGMA busy_timeout = 3000;").unwrap();
            let mut version = 1u32;
            while !stop.load(Ordering::SeqCst) {
                conn.execute(&format!("PRAGMA user_version = {version}"), [])
                    .unwrap();
                version = if version == 1 { 4 } else { 1 };
                std::thread::sleep(Duration::from_millis(5));
            }
            conn.execute("PRAGMA user_version = 4", []).unwrap();
        })
    };
    let mut s = ready(&h);
    for id in 2..=9 {
        let res = call_tool(
            &mut s,
            id,
            "riltra_code_search",
            &json!({"repo": "r", "query": "alpha"}),
        );
        if is_error(&res) {
            let d = doc(&res);
            let kind = d["kind"].as_str().unwrap();
            assert!(
                kind == "schema_outdated" || kind == "store_error",
                "unexpected classification under churn: {d}"
            );
        } else {
            assert_eq!(doc(&res)["repo"], "r");
        }
    }
    stop.store(true, Ordering::SeqCst);
    flipper.join().unwrap();
    let (code, _) = s.finish();
    assert_eq!(code, 0);
    assert_eq!(
        user_version(&db),
        4,
        "the flipper's last write stands; MCP never migrated"
    );
}

#[test]
fn f1_concurrent_cli_rebuild_does_not_disturb_mcp_reads() {
    let h = home("f1-rebuild");
    let r = home("f1-rebuild-repo");
    write_search_fixture(&r);
    indexed_repo(&h, &r, "r");
    let rebuild = {
        let h = h.clone();
        std::thread::spawn(move || {
            for _ in 0..3 {
                let (code, _out, err) = run_cli(&h, &["index", "--repo", "r"]);
                assert_eq!(code, 0, "concurrent rebuild failed: {err}");
            }
        })
    };
    let mut s = ready(&h);
    for id in 2..=7 {
        let res = call_tool(
            &mut s,
            id,
            "riltra_code_search",
            &json!({"repo": "r", "query": "alpha"}),
        );
        if is_error(&res) {
            let d = doc(&res);
            assert_eq!(
                d["kind"], "store_error",
                "unexpected classification during rebuild: {d}"
            );
        } else {
            let d = doc(&res);
            assert_eq!(d["repo"], "r");
            assert!(d["freshness"].as_str().is_some(), "{d}");
        }
    }
    rebuild.join().unwrap();
    let (code, _) = s.finish();
    assert_eq!(code, 0);
    assert_eq!(user_version(&db_path(&h, "r")), 4);
}
