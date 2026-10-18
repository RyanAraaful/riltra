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
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
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
    spawn_with(home, |_| {})
}

fn spawn_with<F: FnOnce(&mut Command)>(home: &Path, configure: F) -> Session {
    let mut child = Command::new(env!("CARGO_BIN_EXE_riltra"));
    child
        .arg("mcp")
        .env("RILTRA_HOME", home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure(&mut child);
    let mut child = child.spawn().expect("spawn riltra mcp");
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
    ready_with(cfg, |_| {})
}

fn ready_with<F: FnOnce(&mut Command)>(cfg: &Path, configure: F) -> Session {
    let mut s = spawn_with(cfg, configure);
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
fn mcp_tools_list_advertises_exactly_eleven_readonly_tools() {
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
            "riltra_code_callees",
            "riltra_code_callers",
            "riltra_code_impact",
            "riltra_code_search",
            "riltra_code_trace",
            "riltra_health",
            "riltra_knowledge_related",
            "riltra_knowledge_search",
            "riltra_knowledge_show",
            "riltra_repo_list",
            "riltra_repo_status"
        ],
        "CP6 advertises exactly the eleven read-only tools"
    );
    assert!(
        !names.iter().any(|n| n.contains("write")),
        "no write tool may be advertised: {names:?}"
    );
    assert_eq!(
        names.iter().filter(|n| *n == &"riltra_health").count(),
        1,
        "riltra_health must exist exactly once"
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
    // CP6: riltra_health takes no arguments and rejects any custom field.
    let health_schema = &find("riltra_health")["inputSchema"];
    assert!(
        health_schema["properties"]
            .as_object()
            .is_none_or(|p| p.is_empty()),
        "riltra_health takes no inputs: {health_schema}"
    );
    assert!(
        health_schema["required"].is_null()
            || health_schema["required"]
                .as_array()
                .is_some_and(|r| r.is_empty()),
        "riltra_health requires nothing: {health_schema}"
    );
    assert_eq!(
        health_schema["additionalProperties"], false,
        "riltra_health must reject unsupported fields: {health_schema}"
    );
    // Every tool with inputs rejects unsupported fields.
    for name in [
        "riltra_code_callees",
        "riltra_code_callers",
        "riltra_code_impact",
        "riltra_code_search",
        "riltra_code_trace",
        "riltra_knowledge_related",
        "riltra_knowledge_search",
        "riltra_knowledge_show",
        "riltra_repo_status",
    ] {
        let schema = &find(name)["inputSchema"];
        assert_eq!(
            schema["additionalProperties"], false,
            "{name} must reject unsupported fields: {schema}"
        );
    }
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
    // CP4 graph tools mirror their CLI counterparts: a required path+line+col
    // anchor with optional symbol/limit/offset; trace adds `to`+`depth`,
    // impact adds `depth` and must not accept `to`. Unknown parameters are
    // rejected (`additionalProperties: false`, CLI unknown-flag strictness).
    for tool in [
        "riltra_code_callers",
        "riltra_code_callees",
        "riltra_code_trace",
        "riltra_code_impact",
    ] {
        let schema = &find(tool)["inputSchema"];
        let mut required: Vec<&str> = schema["required"]
            .as_array()
            .unwrap_or_else(|| panic!("{tool}: required array: {schema}"))
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        required.sort_unstable();
        assert_eq!(
            required,
            ["col", "line", "path", "repo"],
            "{tool}: {schema}"
        );
        let mut keys: Vec<&str> = schema["properties"]
            .as_object()
            .unwrap_or_else(|| panic!("{tool}: properties: {schema}"))
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        let expected: &[&str] = match tool {
            "riltra_code_trace" => &[
                "col", "depth", "limit", "line", "offset", "path", "repo", "symbol", "to",
            ],
            "riltra_code_impact" => &[
                "col", "depth", "limit", "line", "offset", "path", "repo", "symbol",
            ],
            _ => &["col", "limit", "line", "offset", "path", "repo", "symbol"],
        };
        assert_eq!(keys, expected, "{tool}: properties: {schema}");
        assert_eq!(schema["additionalProperties"], false, "{tool}: {schema}");
    }
    // CP5 knowledge tools: search requires repo+query, show/related require
    // repo+id; show takes no generation, related defaults direction to
    // outbound; unknown parameters are rejected everywhere.
    let search_schema = &find("riltra_knowledge_search")["inputSchema"];
    let mut required: Vec<&str> = search_schema["required"]
        .as_array()
        .unwrap_or_else(|| panic!("knowledge_search: required array: {search_schema}"))
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    required.sort_unstable();
    assert_eq!(required, ["query", "repo"], "{search_schema}");
    let mut keys: Vec<&str> = search_schema["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "generation",
            "include_archived",
            "include_retracted",
            "include_superseded",
            "kind",
            "limit",
            "offset",
            "query",
            "repo"
        ],
        "{search_schema}"
    );
    assert_eq!(
        search_schema["additionalProperties"], false,
        "{search_schema}"
    );
    let show_schema = &find("riltra_knowledge_show")["inputSchema"];
    let mut required: Vec<&str> = show_schema["required"]
        .as_array()
        .unwrap_or_else(|| panic!("knowledge_show: required array: {show_schema}"))
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    required.sort_unstable();
    assert_eq!(required, ["id", "repo"], "{show_schema}");
    let mut keys: Vec<&str> = show_schema["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["id", "repo", "revision"], "{show_schema}");
    assert_eq!(show_schema["additionalProperties"], false, "{show_schema}");
    let related_schema = &find("riltra_knowledge_related")["inputSchema"];
    let mut required: Vec<&str> = related_schema["required"]
        .as_array()
        .unwrap_or_else(|| panic!("knowledge_related: required array: {related_schema}"))
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    required.sort_unstable();
    assert_eq!(required, ["id", "repo"], "{related_schema}");
    let mut keys: Vec<&str> = related_schema["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "direction",
            "generation",
            "id",
            "include_retracted",
            "limit",
            "offset",
            "repo",
            "revision"
        ],
        "{related_schema}"
    );
    assert_eq!(
        related_schema["additionalProperties"], false,
        "{related_schema}"
    );
    let (code, _) = s.finish();
    assert_eq!(code, 0, "clean disconnect must exit 0");
    assert_home_untouched(&h);
}

// --- CP6: health composition -------------------------------------------------

#[test]
fn t1_health_composition_matches_cli_and_creates_nothing() {
    // Multiple registered repositories: deterministic count, CLI parity for
    // status and environment doctor, source-of-truth versions and limits.
    let h = home("cp6-health");
    let r1 = home("cp6-health-r1");
    let r2 = home("cp6-health-r2");
    init_repo(&h, &r1, "alpha");
    init_repo(&h, &r2, "beta");
    let before = listing(&h);
    let mut s = ready(&h);
    let res = call_tool(&mut s, 2, "riltra_health", &json!({}));
    assert!(!is_error(&res), "{res}");
    let d = doc(&res);
    assert_eq!(d["operation"], json!("health"));
    assert_eq!(d["status"], cli_json(&h, &["status"]));
    assert_eq!(d["doctor"], cli_json(&h, &["doctor"]));
    assert_eq!(d["doctor"]["scope"], json!("environment"));
    assert_eq!(d["doctor"]["pass"], json!(true));
    assert_eq!(d["versions"]["riltra"], json!(env!("CARGO_PKG_VERSION")));
    assert_eq!(d["versions"]["code_index_schema"], json!(4));
    assert_eq!(d["versions"]["knowledge_schema"], json!(2));
    assert_eq!(d["limits"]["code_search"]["default_limit"], json!(50));
    assert_eq!(d["limits"]["code_search"]["max_limit"], json!(1000));
    assert_eq!(d["limits"]["knowledge_search"]["default_limit"], json!(50));
    assert_eq!(d["limits"]["knowledge_search"]["max_limit"], json!(1000));
    assert_eq!(d["limits"]["graph_traversal"]["depth_default"], json!(3));
    assert_eq!(d["limits"]["graph_traversal"]["depth_max"], json!(6));
    assert_eq!(d["limits"]["graph_traversal"]["default_limit"], json!(50));
    assert_eq!(d["limits"]["graph_traversal"]["max_limit"], json!(1000));
    assert_eq!(d["capabilities"]["transport"], json!("stdio"));
    assert_eq!(d["capabilities"]["read_only"], json!(true));
    assert_eq!(d["capabilities"]["mutations"], json!(false));
    assert_eq!(d["capabilities"]["resources"], json!(false));
    assert_eq!(d["capabilities"]["prompts"], json!(false));
    assert_eq!(d["capabilities"]["network"], json!(false));
    assert_eq!(d["capabilities"]["tools"].as_array().unwrap().len(), 11);
    assert_eq!(d["repo_count"], json!(2));
    let (code, _) = s.finish();
    assert_eq!(code, 0);
    assert_eq!(
        listing(&h),
        before,
        "health must not create or modify any file"
    );

    // Missing registry: empty-registry semantics, count 0.
    let h2 = home("cp6-health-empty");
    let before2 = listing(&h2);
    let mut s2 = ready(&h2);
    let d = doc(&call_tool(&mut s2, 2, "riltra_health", &json!({})));
    assert_eq!(d["repo_count"], json!(0));
    assert_eq!(d["status"], cli_json(&h2, &["status"]));
    assert_eq!(d["doctor"], cli_json(&h2, &["doctor"]));
    let (code, _) = s2.finish();
    assert_eq!(code, 0);
    assert_eq!(listing(&h2), before2);

    // Corrupt registry: classified store_error, never a fabricated count.
    let h3 = home("cp6-health-corrupt");
    init_repo(&h3, &home("cp6-health-corrupt-repo"), "r");
    std::fs::write(h3.join("registry.json"), "{not json").unwrap();
    let mut s3 = ready(&h3);
    let res = call_tool(&mut s3, 2, "riltra_health", &json!({}));
    assert!(is_error(&res), "{res}");
    let d = doc(&res);
    assert_eq!(d["kind"], "store_error");
    assert!(d.get("repo_count").is_none(), "no fabricated count: {d}");
    let res = call_tool(&mut s3, 3, "riltra_repo_list", &json!({}));
    assert!(is_error(&res));
    assert_eq!(doc(&res)["kind"], "store_error");
    let (code, _) = s3.finish();
    assert_eq!(code, 0);

    // Failed doctor check stays visible: with git unavailable the environment
    // doctor reports pass:false inside the health document (registry still
    // readable, count still 0).
    let h4 = home("cp6-health-nogit");
    let empty_bin = home("cp6-health-nogit-bin");
    let mut s4 = ready_with(&h4, |cmd| {
        cmd.env("PATH", &empty_bin);
    });
    let d = doc(&call_tool(&mut s4, 2, "riltra_health", &json!({})));
    assert_eq!(d["doctor"]["pass"], json!(false), "{d}");
    assert!(
        d["doctor"]["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["name"] == "git-available" && c["pass"] == json!(false)),
        "{d}"
    );
    assert_eq!(d["repo_count"], json!(0));
    let (code, _) = s4.finish();
    assert_eq!(code, 0);
}

// --- CP6: G4 hard error paths -------------------------------------------------

#[test]
fn g4_unknown_input_fields_are_rejected_not_ignored() {
    let h = home("cp6-schema");
    let r = home("cp6-schema-repo");
    write_search_fixture(&r);
    indexed_repo(&h, &r, "r");
    let mut s = ready(&h);
    // Warm up the index store so WAL sidecar recreation (accepted L2
    // read-only behavior) is not misread as a mutation below.
    let _ = call_tool(
        &mut s,
        1,
        "riltra_code_search",
        &json!({"repo": "r", "query": "alpha"}),
    );
    let before = listing(&h);
    // Unsupported properties are protocol-level schema refusals on every tool,
    // including the older CP3 inputs that previously ignored them.
    for (id, tool, args) in [
        (2i64, "riltra_repo_status", json!({"repo": "r", "bogus": 1})),
        (
            3,
            "riltra_code_search",
            json!({"repo": "r", "query": "alpha", "bogus": 1}),
        ),
        (
            4,
            "riltra_code_callers",
            json!({"repo": "r", "path": "a.py", "line": 1, "col": 6, "bogus": 1}),
        ),
        (
            5,
            "riltra_code_callees",
            json!({"repo": "r", "path": "a.py", "line": 1, "col": 6, "bogus": 1}),
        ),
        (
            6,
            "riltra_code_trace",
            json!({"repo": "r", "path": "a.py", "line": 1, "col": 6, "bogus": 1}),
        ),
        (
            7,
            "riltra_code_impact",
            json!({"repo": "r", "path": "a.py", "line": 1, "col": 6, "bogus": 1}),
        ),
        (
            8,
            "riltra_knowledge_search",
            json!({"repo": "r", "query": "x", "bogus": 1}),
        ),
        (
            9,
            "riltra_knowledge_show",
            json!({"repo": "r", "id": "d1", "bogus": 1}),
        ),
        (
            10,
            "riltra_knowledge_related",
            json!({"repo": "r", "id": "d1", "bogus": 1}),
        ),
        (11, "riltra_health", json!({"bogus": 1})),
    ] {
        s.send(
            &json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "tools/call",
                "params": {"name": tool, "arguments": args},
            })
            .to_string(),
        );
        let v = s.recv();
        assert_eq!(v["id"], id);
        assert!(
            v.get("error").is_none(),
            "{tool} schema refusal must not become a JSON-RPC error: {v}"
        );
        assert!(
            v["result"]["isError"] == json!(true),
            "{tool} must reject unknown fields at the schema layer: {v}"
        );
        let message = v["result"]["content"][0]["text"].as_str().unwrap();
        assert!(
            message.starts_with("failed to deserialize parameters"),
            "{tool}: SDK-owned refusal expected: {v}"
        );
        assert!(
            message.contains("bogus"),
            "{tool}: refusal must name the offending field: {v}"
        );
    }
    // Non-integer and negative numerics stay schema-level refusals.
    for (id, tool, args) in [
        (
            12,
            "riltra_code_callers",
            json!({"repo": "r", "path": "a.py", "line": -1, "col": 6}),
        ),
        (
            13,
            "riltra_code_callers",
            json!({"repo": "r", "path": "a.py", "line": 1.5, "col": 6}),
        ),
        (
            14,
            "riltra_code_search",
            json!({"repo": "r", "query": "alpha", "limit": -5}),
        ),
        (
            15,
            "riltra_knowledge_search",
            json!({"repo": "r", "query": "x", "generation": -1}),
        ),
    ] {
        s.send(
            &json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "tools/call",
                "params": {"name": tool, "arguments": args},
            })
            .to_string(),
        );
        let v = s.recv();
        assert_eq!(v["id"], id);
        assert!(
            v.get("error").is_none(),
            "{tool} numeric refusal must not become a JSON-RPC error: {v}"
        );
        assert!(
            v["result"]["isError"] == json!(true),
            "{tool} must refuse non-domain numerics at the schema layer: {v}"
        );
        let message = v["result"]["content"][0]["text"].as_str().unwrap();
        assert!(
            message.starts_with("failed to deserialize parameters"),
            "{tool}: SDK-owned refusal expected: {v}"
        );
    }
    // Valid calls still work after every refusal.
    let d = doc(&call_tool(
        &mut s,
        20,
        "riltra_code_search",
        &json!({"repo": "r", "query": "alpha"}),
    ));
    assert_eq!(d["total"], 3);
    let (code, _) = s.finish();
    assert_eq!(code, 0);
    assert_eq!(listing(&h), before, "schema refusals must not create files");
}

#[test]
fn g4_protocol_level_rejections_stay_protocol_level() {
    let h = home("cp6-proto");
    let mut s = ready(&h);
    // Unknown tool: a JSON-RPC method error, never a domain document.
    s.send(
        &json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": {"name": "riltra_nope", "arguments": {}},
        })
        .to_string(),
    );
    let v = s.recv();
    assert_eq!(v["id"], 2);
    assert!(v["error"].as_object().is_some(), "{v}");
    assert!(v.get("result").is_none(), "{v}");
    // Unsupported tool parameters: the SDK refuses with its own deserialization
    // error result, never a domain error invented by the tool.
    s.send(
        &json!({
            "jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": {"name": "riltra_health", "arguments": {"surprise": 1}},
        })
        .to_string(),
    );
    let v = s.recv();
    assert_eq!(v["id"], 3);
    assert!(
        v.get("error").is_none(),
        "schema failures stay SDK-owned, not JSON-RPC errors: {v}"
    );
    assert_eq!(v["result"]["isError"], json!(true), "{v}");
    let message = v["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        message.starts_with("failed to deserialize parameters"),
        "SDK-owned refusal expected: {v}"
    );
    assert!(
        v["result"].get("structuredContent").is_none()
            || v["result"]["structuredContent"]["kind"].is_null(),
        "no domain error kind may be manufactured: {v}"
    );
    // Unparsable JSON (serde_json Category::Syntax) is ignored by rmcp by
    // design: no response must arrive, the stream must stay connected, and the
    // next valid request must succeed. This is a positive contract check, not
    // an optional branch. Upstream #1353 concerns only the -32600 reply for
    // structurally invalid frames on the async_rw error path, not this silence.
    s.send("{not json");
    match s.lines.recv_timeout(Duration::from_secs(3)) {
        Err(RecvTimeoutError::Timeout) => {}
        Err(RecvTimeoutError::Disconnected) => {
            panic!("server closed stdout after unparsable JSON instead of ignoring it")
        }
        Ok(line) => panic!("unparsable JSON must be ignored without a response, got: {line}"),
    }
    // Valid requests still work after protocol-level refusals.
    let res = call_tool(&mut s, 4, "riltra_health", &json!({}));
    assert!(!is_error(&res), "{res}");
    assert_eq!(doc(&res)["repo_count"], json!(0));
    let (code, _) = s.finish();
    assert_eq!(code, 0);
}

#[test]
fn g4_structurally_invalid_jsonrpc_gets_invalid_request() {
    // Well-formed JSON that is not a JSON-RPC message (serde_json
    // Category::Data) gets the transport's -32600 reply. Exercise it on a fresh
    // process before initialize, with no outstanding responses: rmcp 3.5.1 can
    // lose this reply when it competes with a response-send task because
    // AsyncRwTransport::receive is not cancel-safe on this path
    // (modelcontextprotocol/rust-sdk#1353). That upstream race is separate from
    // Riltra's domain-error classification and is deliberately not worked
    // around here; CP7 interoperability evaluation owns it.
    let h = home("cp6-data-error");
    let mut s = spawn(&h);
    s.send(r#"{"foo":"bar"}"#);
    let v = s.recv();
    assert_eq!(v["error"]["code"], json!(-32600), "{v}");
    assert_eq!(v["error"]["message"], json!("Invalid request"), "{v}");
    assert!(v.get("id").is_none(), "protocol error carries no id: {v}");
    assert!(v.get("result").is_none(), "no successful result: {v}");
    assert!(v["error"].get("kind").is_none(), "no domain kind: {v}");
    assert!(
        v["error"]
            .get("data")
            .is_none_or(|d| d.get("kind").is_none()),
        "no domain error classification: {v}"
    );
    // The same session still initializes and serves a valid request.
    s.send(initialize_request());
    let v = s.recv();
    assert_eq!(v["id"], 1, "initialize after protocol error: {v}");
    s.send(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
    let res = call_tool(&mut s, 2, "riltra_repo_list", &json!({}));
    assert!(!is_error(&res), "{res}");
    assert_eq!(doc(&res)["repos"].as_array().unwrap().len(), 0);
    let (code, err) = s.finish();
    assert_eq!(code, 0);
    assert!(!err.contains("panicked"), "server panicked: {err}");
    assert_home_untouched(&h);
}

#[test]
fn g4_invalid_requests_do_not_kill_the_server() {
    let h = home("cp6-resil");
    let r = home("cp6-resil-repo");
    write_search_fixture(&r);
    indexed_repo(&h, &r, "r");
    let mut s = ready(&h);
    // Warm up the index store so WAL sidecar recreation (accepted L2
    // read-only behavior) is not misread as a mutation below.
    let _ = call_tool(
        &mut s,
        1,
        "riltra_code_search",
        &json!({"repo": "r", "query": "alpha"}),
    );
    let before = listing(&h);
    // Valid, invalid domain, valid again.
    let d = doc(&call_tool(
        &mut s,
        2,
        "riltra_code_search",
        &json!({"repo": "r", "query": "alpha"}),
    ));
    assert_eq!(d["total"], 3);
    let res = call_tool(
        &mut s,
        3,
        "riltra_code_search",
        &json!({"repo": "nope", "query": "x"}),
    );
    assert!(is_error(&res));
    let d = doc(&call_tool(
        &mut s,
        4,
        "riltra_code_search",
        &json!({"repo": "r", "query": "alpha"}),
    ));
    assert_eq!(d["total"], 3);
    // Unknown tool.
    s.send(
        &json!({
            "jsonrpc": "2.0", "id": 5, "method": "tools/call",
            "params": {"name": "riltra_nope", "arguments": {}},
        })
        .to_string(),
    );
    let v = s.recv();
    assert!(v["error"].as_object().is_some(), "{v}");
    // Unparsable JSON is ignored by rmcp: no response may arrive, and the
    // session must remain usable. An unexpected response or a disconnect fails.
    s.send("{not json");
    match s.lines.recv_timeout(Duration::from_secs(3)) {
        Err(RecvTimeoutError::Timeout) => {}
        Err(RecvTimeoutError::Disconnected) => {
            panic!("server closed stdout after unparsable JSON instead of ignoring it")
        }
        Ok(line) => panic!("unparsable JSON must be ignored without a response, got: {line}"),
    }
    // Valid after malformed: clean EOF afterwards.
    let d = doc(&call_tool(&mut s, 6, "riltra_repo_list", &json!({})));
    assert_eq!(d["repos"].as_array().unwrap().len(), 1);
    let (code, err) = s.finish();
    assert_eq!(code, 0);
    assert!(!err.contains("panicked"), "server panicked: {err}");
    assert_eq!(
        listing(&h),
        before,
        "invalid requests must not mutate storage"
    );
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
    // The one exact, copy-runnable command the refusals must hand the user.
    let command = "riltra index status --repo v1repo";
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
            d["message"].as_str().unwrap().contains(command),
            "{tool}: refusal must name the exact CLI command `{command}`: {d}"
        );
    }
    assert_eq!(user_version(&db), 1, "MCP must not migrate");
    // Executing exactly that command migrates the file through the CLI.
    let args: Vec<&str> = command.split_whitespace().skip(1).collect();
    let (code, out, err) = run_cli(&h, &args);
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

// --- CP4 helpers: SCIP-backed graph fixture ---------------------------------

// Minimal proto3 writer for exactly the fields the importer reads (field
// numbers pinned by proto/scip.proto; same shape as tests/edges.rs).
fn varint(mut v: u64, out: &mut Vec<u8>) {
    loop {
        let mut b = (v & 0x7f) as u8;
        v >>= 7;
        if v != 0 {
            b |= 0x80;
        }
        out.push(b);
        if v == 0 {
            break;
        }
    }
}

fn tag(field: u32, wire: u32, out: &mut Vec<u8>) {
    varint(((field << 3) | wire) as u64, out);
}

fn len_field(field: u32, bytes: &[u8], out: &mut Vec<u8>) {
    tag(field, 2, out);
    varint(bytes.len() as u64, out);
    out.extend_from_slice(bytes);
}

fn str_field(field: u32, value: &str, out: &mut Vec<u8>) {
    len_field(field, value.as_bytes(), out);
}

fn varint_field(field: u32, value: u64, out: &mut Vec<u8>) {
    tag(field, 0, out);
    varint(value, out);
}

fn occurrence(symbol: &str, roles: u64, line: i64, start: i64, end: i64) -> Vec<u8> {
    let mut r = Vec::new();
    varint_field(1, line as u64, &mut r);
    varint_field(2, start as u64, &mut r);
    varint_field(3, end as u64, &mut r);
    let mut o = Vec::new();
    str_field(2, symbol, &mut o);
    varint_field(3, roles, &mut o);
    len_field(8, &r, &mut o);
    o
}

fn test_document(path: &str, occurrences: &[Vec<u8>], symbols: &[(&str, &str)]) -> Vec<u8> {
    let mut doc = Vec::new();
    str_field(1, path, &mut doc);
    for o in occurrences {
        len_field(2, o, &mut doc);
    }
    for (uri, name) in symbols {
        let mut info = Vec::new();
        str_field(1, uri, &mut info);
        str_field(6, name, &mut info);
        len_field(3, &info, &mut doc);
    }
    varint_field(6, 1, &mut doc);
    doc
}

fn graph_uri(name: &str) -> String {
    format!("scip-test test test-pkg 1.0.0 graph.py/{name}().")
}

/// Index a one-file Python repo whose fN functions call the listed targets,
/// then import matching SCIP facts (provider `cp4`): Exact CALLS edges and
/// semantic anchors. Returns the 1-based line of each fN definition; col 6 is
/// inside every `fN` name.
fn graph_repo(cfg: &Path, dir: &Path, name: &str, adjacency: &[Vec<usize>]) -> Vec<usize> {
    let names: Vec<String> = (0..adjacency.len()).map(|i| format!("f{i}")).collect();
    let uris: Vec<String> = names.iter().map(|n| graph_uri(n)).collect();
    let mut src = String::new();
    let mut occurrences = Vec::new();
    let mut lines = Vec::new();
    let mut line = 0usize;
    for (i, targets) in adjacency.iter().enumerate() {
        lines.push(line + 1);
        src.push_str(&format!("def {}():\n", names[i]));
        occurrences.push(occurrence(
            &uris[i],
            1,
            line as i64,
            4,
            (4 + names[i].len()) as i64,
        ));
        line += 1;
        if targets.is_empty() {
            src.push_str("    pass\n");
            line += 1;
        }
        for target in targets {
            src.push_str(&format!("    {}()\n", names[*target]));
            occurrences.push(occurrence(
                &uris[*target],
                0,
                line as i64,
                4,
                (4 + names[*target].len()) as i64,
            ));
            line += 1;
        }
        src.push('\n');
        line += 1;
    }
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("graph.py"), &src).unwrap();
    init_repo(cfg, dir, name);
    let (code, _out, err) = run_cli(cfg, &["index", "--repo", name]);
    assert_eq!(code, 0, "index {name}: {err}");
    let symbols: Vec<(&str, &str)> = uris
        .iter()
        .zip(names.iter())
        .map(|(u, n)| (u.as_str(), n.as_str()))
        .collect();
    let mut tool = Vec::new();
    str_field(1, "cp4", &mut tool);
    str_field(2, "1.0", &mut tool);
    let mut md = Vec::new();
    len_field(2, &tool, &mut md);
    str_field(
        3,
        &format!("file://{}", dir.to_string_lossy().replace('\\', "/")),
        &mut md,
    );
    varint_field(4, 1, &mut md);
    let mut payload = Vec::new();
    len_field(1, &md, &mut payload);
    len_field(
        2,
        &test_document("graph.py", &occurrences, &symbols),
        &mut payload,
    );
    let scip = cfg.join("index.scip");
    std::fs::write(&scip, &payload).unwrap();
    let (code, _out, err) = run_cli(
        cfg,
        &[
            "semantic",
            "import",
            "--repo",
            name,
            "--scip",
            &scip.to_string_lossy(),
        ],
    );
    assert_eq!(code, 0, "semantic import {name}: {err}");
    lines
}

/// Syntax-only graph fixture (no SCIP facts): Heuristic POTENTIAL_CALL edges.
fn syntax_graph_repo(cfg: &Path, dir: &Path, name: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join("a.py"),
        "def target():\n    pass\n\ndef caller():\n    target()\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("b.py"),
        "from a import target\n\ndef second():\n    target()\n",
    )
    .unwrap();
    init_repo(cfg, dir, name);
    let (code, _out, err) = run_cli(cfg, &["index", "--repo", name]);
    assert_eq!(code, 0, "index {name}: {err}");
}

fn graph_doc(s: &mut Session, id: i64, tool: &str, args: &Value) -> Value {
    let res = call_tool(s, id, tool, args);
    assert!(!is_error(&res), "{tool} failed: {res}");
    doc(&res)
}

/// MCP graph document must equal the parsed CLI document for the same input
/// and repository state (design gate §3 parity rule).
fn assert_graph_parity(cfg: &Path, got: &Value, cli: &[&str]) {
    let expected = cli_json(cfg, cli);
    assert_eq!(got, &expected, "MCP/CLI parity for {cli:?}");
}

// --- T2: callers / callees ---------------------------------------------------

#[test]
fn t2_callers_callees_match_cli_with_semantic_identity_and_paging() {
    let h = home("cp4-t2");
    let r = home("cp4-t2-repo");
    // f0 calls f1,f2; f1,f2,f3 call f0.
    let lines = graph_repo(&h, &r, "g", &[vec![1, 2], vec![0], vec![0], vec![0]]);
    let mut s = ready(&h);
    let line0 = lines[0].to_string();

    // Callers of f0 = f1,f2,f3: semantic Exact CALLS with provenance.
    let d = graph_doc(
        &mut s,
        2,
        "riltra_code_callers",
        &json!({"repo": "g", "path": "graph.py", "line": lines[0], "col": 6}),
    );
    assert_graph_parity(
        &h,
        &d,
        &[
            "callers", "--repo", "g", "--path", "graph.py", "--line", &line0, "--col", "6",
        ],
    );
    assert_eq!(d["completeness"]["total"], 3, "{d}");
    assert!(d["anchor"]["anchor_uncertainty"].is_null(), "{d}");
    assert_eq!(
        d["anchor"]["endpoint"]["symbol_uri"],
        graph_uri("f0"),
        "{d}"
    );
    for edge in d["edges"].as_array().unwrap() {
        assert_eq!(edge["kind"], "CALLS", "{edge}");
        assert_eq!(edge["precision"], "Exact", "{edge}");
        assert_eq!(edge["evidence"]["imports"][0]["provider"], "cp4", "{edge}");
    }

    // Callees of f0 = f1,f2.
    let d = graph_doc(
        &mut s,
        3,
        "riltra_code_callees",
        &json!({"repo": "g", "path": "graph.py", "line": lines[0], "col": 6}),
    );
    assert_graph_parity(
        &h,
        &d,
        &[
            "callees", "--repo", "g", "--path", "graph.py", "--line", &line0, "--col", "6",
        ],
    );
    assert_eq!(d["direction"], "callees", "{d}");
    assert_eq!(d["completeness"]["total"], 2, "{d}");

    // Exact symbol identity: the agreeing URI passes, a contradicting one
    // refuses instead of silently switching.
    let d = graph_doc(
        &mut s,
        4,
        "riltra_code_callers",
        &json!({"repo": "g", "path": "graph.py", "line": lines[0], "col": 6, "symbol": graph_uri("f0")}),
    );
    assert_graph_parity(
        &h,
        &d,
        &[
            "callers",
            "--repo",
            "g",
            "--path",
            "graph.py",
            "--line",
            &line0,
            "--col",
            "6",
            "--symbol",
            &graph_uri("f0"),
        ],
    );
    let res = call_tool(
        &mut s,
        5,
        "riltra_code_callers",
        &json!({"repo": "g", "path": "graph.py", "line": lines[0], "col": 6, "symbol": graph_uri("f1")}),
    );
    assert!(is_error(&res), "{res}");
    let d = doc(&res);
    assert_eq!(d["kind"], "invalid_input", "{d}");
    assert!(
        d["message"]
            .as_str()
            .unwrap()
            .contains("--symbol disagrees"),
        "{d}"
    );

    // Pagination: page 1 truncates over the real total, page 2 completes.
    let p0 = graph_doc(
        &mut s,
        6,
        "riltra_code_callers",
        &json!({"repo": "g", "path": "graph.py", "line": lines[0], "col": 6, "limit": 2}),
    );
    assert_graph_parity(
        &h,
        &p0,
        &[
            "callers", "--repo", "g", "--path", "graph.py", "--line", &line0, "--col", "6",
            "--limit", "2",
        ],
    );
    assert_eq!(p0["truncated"], true, "{p0}");
    assert_eq!(p0["limits"]["next_offset"], 2, "{p0}");
    assert_eq!(p0["edges"].as_array().unwrap().len(), 2, "{p0}");
    let p1 = graph_doc(
        &mut s,
        7,
        "riltra_code_callers",
        &json!({"repo": "g", "path": "graph.py", "line": lines[0], "col": 6, "limit": 2, "offset": 2}),
    );
    assert_graph_parity(
        &h,
        &p1,
        &[
            "callers", "--repo", "g", "--path", "graph.py", "--line", &line0, "--col", "6",
            "--limit", "2", "--offset", "2",
        ],
    );
    assert_eq!(p1["truncated"], false, "{p1}");
    assert_eq!(p1["limits"]["next_offset"], Value::Null, "{p1}");

    // Empty result: f3 has no callers; coverage is never claimed complete.
    let line3 = lines[3].to_string();
    let d = graph_doc(
        &mut s,
        8,
        "riltra_code_callers",
        &json!({"repo": "g", "path": "graph.py", "line": lines[3], "col": 6}),
    );
    assert_graph_parity(
        &h,
        &d,
        &[
            "callers", "--repo", "g", "--path", "graph.py", "--line", &line3, "--col", "6",
        ],
    );
    assert_eq!(d["completeness"]["total"], 0, "{d}");
    assert_eq!(d["edges"], json!([]), "{d}");
    assert_eq!(d["completeness"]["status"], "partial", "{d}");
    assert!(
        d["completeness"]["unresolved"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r == "empty-result-not-coverage-proof"),
        "{d}"
    );

    let (code, _) = s.finish();
    assert_eq!(code, 0);
}

#[test]
fn t2_syntax_only_callers_keep_heuristic_uncertainty_and_match_cli() {
    let h = home("cp4-t2s");
    let r = home("cp4-t2s-repo");
    syntax_graph_repo(&h, &r, "s");
    let mut s = ready(&h);

    let d = graph_doc(
        &mut s,
        2,
        "riltra_code_callers",
        &json!({"repo": "s", "path": "a.py", "line": 1, "col": 6}),
    );
    assert_graph_parity(
        &h,
        &d,
        &[
            "callers", "--repo", "s", "--path", "a.py", "--line", "1", "--col", "6",
        ],
    );
    assert_eq!(d["completeness"]["total"], 2, "{d}");
    assert_eq!(d["anchor"]["kind"], "syntax-fallback", "{d}");
    assert!(d["anchor"]["anchor_uncertainty"].is_string(), "{d}");
    for edge in d["edges"].as_array().unwrap() {
        assert_eq!(
            edge["kind"], "POTENTIAL_CALL",
            "heuristic edges are never promoted to CALLS: {edge}"
        );
        assert_eq!(edge["precision"], "Heuristic", "{edge}");
        assert_eq!(edge["verification"], "Unverified", "{edge}");
        assert!(
            edge["callee"].is_null(),
            "a syntax callee is not identity proof: {edge}"
        );
    }
    assert_eq!(d["verification"], "Unverified", "{d}");
    assert_eq!(d["freshness"]["status"], "fresh", "{d}");
    assert!(
        d["completeness"]["unresolved"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r == "unresolved_target"),
        "{d}"
    );

    let d = graph_doc(
        &mut s,
        3,
        "riltra_code_callees",
        &json!({"repo": "s", "path": "a.py", "line": 5, "col": 5}),
    );
    assert_graph_parity(
        &h,
        &d,
        &[
            "callees", "--repo", "s", "--path", "a.py", "--line", "5", "--col", "5",
        ],
    );
    assert_eq!(d["completeness"]["total"], 1, "{d}");
    assert_eq!(d["edges"][0]["kind"], "POTENTIAL_CALL", "{d}");

    let (code, _) = s.finish();
    assert_eq!(code, 0);
}

// --- T3: trace / impact ------------------------------------------------------

#[test]
fn t3_trace_matches_cli_direct_transitive_and_destination() {
    let h = home("cp4-t3");
    let r = home("cp4-t3-repo");
    // f0 -> f1 -> f2
    let lines = graph_repo(&h, &r, "g", &[vec![1], vec![2], vec![]]);
    let mut s = ready(&h);
    let line0 = lines[0].to_string();

    // Direct + transitive enumeration to the chain end.
    let d = graph_doc(
        &mut s,
        2,
        "riltra_code_trace",
        &json!({"repo": "g", "path": "graph.py", "line": lines[0], "col": 6, "depth": 3}),
    );
    assert_graph_parity(
        &h,
        &d,
        &[
            "trace", "--repo", "g", "--path", "graph.py", "--line", &line0, "--col", "6",
            "--depth", "3",
        ],
    );
    assert_eq!(d["disclaimer"], "plausible-path-not-execution", "{d}");
    assert_eq!(d["budgets"]["depth"], 3, "{d}");
    assert_eq!(d["budgets"]["max_depth"], 6, "{d}");
    assert_eq!(d["paths"].as_array().unwrap().len(), 1, "{d}");
    assert_eq!(d["paths"][0]["edges"].as_array().unwrap().len(), 2, "{d}");
    assert_eq!(d["paths"][0]["edges"][0]["kind"], "CALLS", "{d}");
    assert_eq!(d["cycle"], false, "{d}");
    assert_eq!(d["truncated"], false, "{d}");

    // Destination-constrained trace: PATH:LINE:COL, same interpretation as CLI.
    let to = format!("graph.py:{}:6", lines[2]);
    let d = graph_doc(
        &mut s,
        3,
        "riltra_code_trace",
        &json!({"repo": "g", "path": "graph.py", "line": lines[0], "col": 6, "to": to.clone(), "depth": 6}),
    );
    assert_graph_parity(
        &h,
        &d,
        &[
            "trace", "--repo", "g", "--path", "graph.py", "--line", &line0, "--col", "6", "--to",
            &to, "--depth", "6",
        ],
    );
    assert_eq!(d["destination"]["path"], "graph.py", "{d}");
    assert_eq!(d["destination"]["line"], lines[2], "{d}");
    assert_eq!(d["paths"].as_array().unwrap().len(), 1, "{d}");

    // Depth boundary 1 is valid and echoed in the budgets.
    let d = graph_doc(
        &mut s,
        4,
        "riltra_code_trace",
        &json!({"repo": "g", "path": "graph.py", "line": lines[0], "col": 6, "depth": 1}),
    );
    assert_graph_parity(
        &h,
        &d,
        &[
            "trace", "--repo", "g", "--path", "graph.py", "--line", &line0, "--col", "6",
            "--depth", "1",
        ],
    );
    assert_eq!(d["budgets"]["depth"], 1, "{d}");

    // Unreachable destination: explicit empty result, never fabricated.
    let to = format!("graph.py:{}:6", lines[0]);
    let line2 = lines[2].to_string();
    let d = graph_doc(
        &mut s,
        5,
        "riltra_code_trace",
        &json!({"repo": "g", "path": "graph.py", "line": lines[2], "col": 6, "to": to.clone()}),
    );
    assert_graph_parity(
        &h,
        &d,
        &[
            "trace", "--repo", "g", "--path", "graph.py", "--line", &line2, "--col", "6", "--to",
            &to,
        ],
    );
    assert_eq!(d["paths"], json!([]), "{d}");
    assert!(
        d["unresolved"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r == "target-unreachable"),
        "{d}"
    );

    let (code, _) = s.finish();
    assert_eq!(code, 0);
}

#[test]
fn t3_trace_cycle_truncation_and_budgets_match_cli() {
    let h = home("cp4-t3c");
    let r = home("cp4-t3c-repo");
    // f0 <-> f1: a cycle.
    let lines = graph_repo(&h, &r, "g", &[vec![1], vec![0]]);
    let mut s = ready(&h);
    let line0 = lines[0].to_string();
    let d = graph_doc(
        &mut s,
        2,
        "riltra_code_trace",
        &json!({"repo": "g", "path": "graph.py", "line": lines[0], "col": 6, "depth": 6}),
    );
    assert_graph_parity(
        &h,
        &d,
        &[
            "trace", "--repo", "g", "--path", "graph.py", "--line", &line0, "--col", "6",
            "--depth", "6",
        ],
    );
    assert_eq!(d["cycle"], true, "{d}");
    assert_eq!(d["paths"][0]["cycle"], true, "{d}");
    assert!(d["visits"]["nodes"].as_u64().unwrap() <= 500, "{d}");
    let (code, _) = s.finish();
    assert_eq!(code, 0);

    // f0 -> f1, f2: two sibling paths, paginated and truncated.
    let h = home("cp4-t3t");
    let r = home("cp4-t3t-repo");
    let lines = graph_repo(&h, &r, "g", &[vec![1, 2], vec![], vec![]]);
    let mut s = ready(&h);
    let line0 = lines[0].to_string();
    let p0 = graph_doc(
        &mut s,
        2,
        "riltra_code_trace",
        &json!({"repo": "g", "path": "graph.py", "line": lines[0], "col": 6, "depth": 2, "limit": 1}),
    );
    assert_graph_parity(
        &h,
        &p0,
        &[
            "trace", "--repo", "g", "--path", "graph.py", "--line", &line0, "--col", "6",
            "--depth", "2", "--limit", "1",
        ],
    );
    assert_eq!(p0["completeness"]["total"], 2, "{p0}");
    assert_eq!(p0["paths"].as_array().unwrap().len(), 1, "{p0}");
    assert_eq!(p0["truncated"], true, "{p0}");
    assert_eq!(p0["limits"]["next_offset"], 1, "{p0}");
    assert_eq!(
        p0["completeness"]["denominator"]["truncated_branches"], 1,
        "{p0}"
    );
    let p1 = graph_doc(
        &mut s,
        3,
        "riltra_code_trace",
        &json!({"repo": "g", "path": "graph.py", "line": lines[0], "col": 6, "depth": 2, "limit": 1, "offset": 1}),
    );
    assert_graph_parity(
        &h,
        &p1,
        &[
            "trace", "--repo", "g", "--path", "graph.py", "--line", &line0, "--col", "6",
            "--depth", "2", "--limit", "1", "--offset", "1",
        ],
    );
    assert_eq!(p1["truncated"], false, "{p1}");
    assert_eq!(p1["limits"]["next_offset"], Value::Null, "{p1}");

    // No outgoing edges: an empty enumeration, with the empty-result reason.
    let line1 = lines[1].to_string();
    let d = graph_doc(
        &mut s,
        4,
        "riltra_code_trace",
        &json!({"repo": "g", "path": "graph.py", "line": lines[1], "col": 6}),
    );
    assert_graph_parity(
        &h,
        &d,
        &[
            "trace", "--repo", "g", "--path", "graph.py", "--line", &line1, "--col", "6",
        ],
    );
    assert_eq!(d["paths"], json!([]), "{d}");
    assert!(
        d["completeness"]["unresolved"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r == "empty-result-not-coverage-proof"),
        "{d}"
    );
    let (code, _) = s.finish();
    assert_eq!(code, 0);
}

#[test]
fn t3_impact_matches_cli_verdicts_cycles_and_paging() {
    let h = home("cp4-t3i");
    let r = home("cp4-t3i-repo");
    let lines = graph_repo(&h, &r, "g", &[vec![1], vec![2], vec![]]);
    let mut s = ready(&h);
    let last = lines[2].to_string();
    let d = graph_doc(
        &mut s,
        2,
        "riltra_code_impact",
        &json!({"repo": "g", "path": "graph.py", "line": lines[2], "col": 6, "depth": 3}),
    );
    assert_graph_parity(
        &h,
        &d,
        &[
            "impact", "--repo", "g", "--path", "graph.py", "--line", &last, "--col", "6",
            "--depth", "3",
        ],
    );
    assert_eq!(d["direction"], "impact", "{d}");
    assert_eq!(d["verdict"], "HIGH", "{d}");
    assert_eq!(d["disclaimer"], "plausible-impact-not-execution", "{d}");
    assert_eq!(d["paths"].as_array().unwrap().len(), 2, "{d}");
    assert_eq!(d["paths"][0]["scope"], "direct", "{d}");
    assert_eq!(d["paths"][1]["scope"], "transitive", "{d}");
    assert_eq!(d["paths"][1]["distance"], 2, "{d}");
    assert_eq!(d["destination"], Value::Null, "{d}");

    // Pagination over impact paths.
    let p0 = graph_doc(
        &mut s,
        3,
        "riltra_code_impact",
        &json!({"repo": "g", "path": "graph.py", "line": lines[2], "col": 6, "limit": 1}),
    );
    assert_graph_parity(
        &h,
        &p0,
        &[
            "impact", "--repo", "g", "--path", "graph.py", "--line", &last, "--col", "6",
            "--limit", "1",
        ],
    );
    assert_eq!(p0["truncated"], true, "{p0}");
    assert_eq!(p0["limits"]["next_offset"], 1, "{p0}");

    // A cycle stays explicit in impact too.
    let h2 = home("cp4-t3ic");
    let r2 = home("cp4-t3ic-repo");
    let lines2 = graph_repo(&h2, &r2, "g", &[vec![1], vec![0]]);
    let mut s2 = ready(&h2);
    let line0 = lines2[0].to_string();
    let d = graph_doc(
        &mut s2,
        2,
        "riltra_code_impact",
        &json!({"repo": "g", "path": "graph.py", "line": lines2[0], "col": 6, "depth": 6}),
    );
    assert_graph_parity(
        &h2,
        &d,
        &[
            "impact", "--repo", "g", "--path", "graph.py", "--line", &line0, "--col", "6",
            "--depth", "6",
        ],
    );
    assert_eq!(d["cycle"], true, "{d}");
    assert_eq!(d["verdict"], "HIGH", "{d}");
    let (code, _) = s2.finish();
    assert_eq!(code, 0);
    let (code, _) = s.finish();
    assert_eq!(code, 0);
}

// --- T4: error classification ------------------------------------------------

#[test]
fn t4_graph_unknown_repo_and_missing_index_are_classified() {
    let h = home("cp4-t4m");
    let r = home("cp4-t4m-repo");
    std::fs::create_dir_all(&r).unwrap();
    std::fs::write(r.join("a.py"), "def target():\n    pass\n").unwrap();
    init_repo(&h, &r, "s");
    let mut s = ready(&h);
    let before = listing(&h);

    let res = call_tool(
        &mut s,
        2,
        "riltra_code_callers",
        &json!({"repo": "nope", "path": "a.py", "line": 1, "col": 1}),
    );
    assert!(is_error(&res), "{res}");
    let d = doc(&res);
    assert_eq!(d["kind"], "not_registered", "{d}");
    assert_eq!(d["message"], "unknown repository: 'nope'", "{d}");

    // A registered repo with no index: the CLI's no-snapshot text, no_index.
    for (id, tool) in [
        (3, "riltra_code_callers"),
        (4, "riltra_code_callees"),
        (5, "riltra_code_trace"),
        (6, "riltra_code_impact"),
    ] {
        let res = call_tool(
            &mut s,
            id,
            tool,
            &json!({"repo": "s", "path": "a.py", "line": 1, "col": 6}),
        );
        assert!(is_error(&res), "{tool}: {res}");
        let d = doc(&res);
        assert_eq!(d["kind"], "no_index", "{tool}: {d}");
        assert_eq!(
            d["message"], "no snapshot: index the repository first",
            "{tool}: {d}"
        );
    }
    // Missing stores stay missing: the graph tools created nothing.
    assert_eq!(listing(&h), before, "graph tools must not create storage");

    // Exit-3 CLI parity: with the index directory present the CLI creates the
    // empty database itself and reports the same missing-snapshot text.
    std::fs::create_dir_all(h.join("index")).unwrap();
    let (code, _out, err) = run_cli(
        &h,
        &[
            "callers", "--repo", "s", "--path", "a.py", "--line", "1", "--col", "6",
        ],
    );
    assert_eq!(code, 3);
    assert!(
        err.contains("no snapshot: index the repository first"),
        "{err}"
    );
    // The CLI-created empty v4 database has no snapshot: still no_index.
    let db = db_path(&h, "s");
    assert_eq!(user_version(&db), 4);
    let res = call_tool(
        &mut s,
        7,
        "riltra_code_callers",
        &json!({"repo": "s", "path": "a.py", "line": 1, "col": 6}),
    );
    let d = doc(&res);
    assert_eq!(d["kind"], "no_index", "{d}");
    assert_eq!(user_version(&db), 4, "classification must not migrate");

    let (code, _) = s.finish();
    assert_eq!(code, 0);
}

#[test]
fn t4_graph_schema_guard_outdated_ahead_and_corrupt() {
    // v1 fixture: refused without migration, naming the runnable CLI command.
    let h = home("cp4-t4o");
    let r = home("cp4-t4o-repo");
    std::fs::create_dir_all(&r).unwrap();
    std::fs::write(r.join("a.py"), "def target():\n    pass\n").unwrap();
    init_repo(&h, &r, "v1repo");
    let db = db_path(&h, "v1repo");
    std::fs::create_dir_all(db.parent().unwrap()).unwrap();
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(V1_TABLES).unwrap();
    }
    let mut s = ready(&h);
    for (id, tool) in [(2, "riltra_code_callers"), (3, "riltra_code_trace")] {
        let res = call_tool(
            &mut s,
            id,
            tool,
            &json!({"repo": "v1repo", "path": "a.py", "line": 1, "col": 1}),
        );
        assert!(is_error(&res), "{tool}: {res}");
        let d = doc(&res);
        assert_eq!(d["kind"], "schema_outdated", "{tool}: {d}");
        assert!(
            d["message"]
                .as_str()
                .unwrap()
                .contains("riltra index status --repo v1repo"),
            "{tool}: {d}"
        );
    }
    assert_eq!(user_version(&db), 1, "MCP must not migrate graph stores");
    let (code, _) = s.finish();
    assert_eq!(code, 0);

    // v5: schema_ahead, untouched.
    let h2 = home("cp4-t4a");
    let r2 = home("cp4-t4a-repo");
    syntax_graph_repo(&h2, &r2, "s");
    let db2 = db_path(&h2, "s");
    set_user_version(&db2, 5);
    let mut s2 = ready(&h2);
    let res = call_tool(
        &mut s2,
        2,
        "riltra_code_impact",
        &json!({"repo": "s", "path": "a.py", "line": 1, "col": 6}),
    );
    let d = doc(&res);
    assert_eq!(d["kind"], "schema_ahead", "{d}");
    assert!(
        d["message"]
            .as_str()
            .unwrap()
            .contains("newer than supported"),
        "{d}"
    );
    assert_eq!(user_version(&db2), 5, "ahead stores are never touched");
    let (code, _) = s2.finish();
    assert_eq!(code, 0);

    // Corrupt file: store_error, byte-identical afterwards.
    let h3 = home("cp4-t4c");
    let r3 = home("cp4-t4c-repo");
    std::fs::create_dir_all(&r3).unwrap();
    std::fs::write(r3.join("a.py"), "def target():\n    pass\n").unwrap();
    init_repo(&h3, &r3, "s");
    let db3 = db_path(&h3, "s");
    std::fs::create_dir_all(db3.parent().unwrap()).unwrap();
    std::fs::write(&db3, b"definitely not a sqlite database").unwrap();
    let bytes_before = std::fs::read(&db3).unwrap();
    let mut s3 = ready(&h3);
    let res = call_tool(
        &mut s3,
        2,
        "riltra_code_callees",
        &json!({"repo": "s", "path": "a.py", "line": 1, "col": 6}),
    );
    assert!(is_error(&res), "{res}");
    let d = doc(&res);
    assert_eq!(d["kind"], "store_error", "{d}");
    assert_eq!(
        std::fs::read(&db3).unwrap(),
        bytes_before,
        "corrupt store bytes untouched"
    );
    let (code, _) = s3.finish();
    assert_eq!(code, 0);
}

#[test]
fn t4_graph_invalid_inputs_are_classified_with_cli_messages() {
    let h = home("cp4-t4i");
    let r = home("cp4-t4i-repo");
    let _lines = graph_repo(&h, &r, "g", &[vec![1], vec![]]);
    let mut s = ready(&h);
    let refused = |s: &mut Session, id: i64, tool: &str, args: Value| -> Value {
        let res = call_tool(s, id, tool, &args);
        assert!(is_error(&res), "{tool} {args}: {res}");
        doc(&res)
    };

    // Zero line/column: exact CLI exit-2 texts.
    for (id, args, text) in [
        (
            2,
            json!({"repo": "g", "path": "graph.py", "line": 0, "col": 6}),
            "invalid --line value: '0'",
        ),
        (
            3,
            json!({"repo": "g", "path": "graph.py", "line": 1, "col": 0}),
            "invalid --col value: '0'",
        ),
    ] {
        let d = refused(&mut s, id, "riltra_code_callers", args);
        assert_eq!(d["kind"], "invalid_input", "{d}");
        assert_eq!(d["message"], text, "{d}");
    }
    let (code, _out, err) = run_cli(
        &h,
        &[
            "callers", "--repo", "g", "--path", "graph.py", "--line", "0", "--col", "6",
        ],
    );
    assert_eq!(code, 2);
    assert!(err.contains("invalid --line value: '0'"), "{err}");

    // Escaping path: the index-level refusal text, preserved.
    let d = refused(
        &mut s,
        4,
        "riltra_code_callers",
        json!({"repo": "g", "path": "../x", "line": 1, "col": 1}),
    );
    assert_eq!(d["kind"], "invalid_input", "{d}");
    assert_eq!(d["message"], "non-canonical document path: '../x'", "{d}");

    // Limit/offset/depth ranges: CLI flag texts verbatim.
    for (id, tool, args, text) in [
        (
            5,
            "riltra_code_callers",
            json!({"repo": "g", "path": "graph.py", "line": 1, "col": 6, "limit": 0}),
            "--limit must be between 1 and 1000",
        ),
        (
            6,
            "riltra_code_callees",
            json!({"repo": "g", "path": "graph.py", "line": 1, "col": 6, "limit": 1001}),
            "--limit must be between 1 and 1000",
        ),
        (
            7,
            "riltra_code_callers",
            json!({"repo": "g", "path": "graph.py", "line": 1, "col": 6, "offset": 9223372036854775807u64}),
            "--offset exceeds supported range",
        ),
        (
            8,
            "riltra_code_trace",
            json!({"repo": "g", "path": "graph.py", "line": 1, "col": 6, "depth": 0}),
            "--depth must be between 1 and 6",
        ),
        (
            9,
            "riltra_code_impact",
            json!({"repo": "g", "path": "graph.py", "line": 1, "col": 6, "depth": 7}),
            "--depth must be between 1 and 6",
        ),
    ] {
        let d = refused(&mut s, id, tool, args);
        assert_eq!(d["kind"], "invalid_input", "{tool}: {d}");
        assert_eq!(d["message"], text, "{tool}: {d}");
    }

    // Malformed destination: the CLI `--to` grammar, verbatim.
    for (id, to, text) in [
        (10, "nope", "--to must be PATH:LINE:COL"),
        (11, ":1:1", "--to requires a path"),
        (12, "graph.py:0:1", "invalid --to line value: '0'"),
    ] {
        let d = refused(
            &mut s,
            id,
            "riltra_code_trace",
            json!({"repo": "g", "path": "graph.py", "line": 1, "col": 6, "to": to}),
        );
        assert_eq!(d["kind"], "invalid_input", "{d}");
        assert_eq!(d["message"], text, "{d}");
    }

    // Anchorless position (no occurrence, no enclosing definition).
    let h2 = home("cp4-t4x");
    let r2 = home("cp4-t4x-repo");
    std::fs::create_dir_all(&r2).unwrap();
    std::fs::write(r2.join("empty.py"), "\n\n\n").unwrap();
    init_repo(&h2, &r2, "e");
    let (code, _out, err) = run_cli(&h2, &["index", "--repo", "e"]);
    assert_eq!(code, 0, "{err}");
    let mut s2 = ready(&h2);
    let d = refused(
        &mut s2,
        2,
        "riltra_code_callers",
        json!({"repo": "e", "path": "empty.py", "line": 2, "col": 1}),
    );
    assert_eq!(d["kind"], "invalid_input", "{d}");
    assert_eq!(d["message"], "no anchor here", "{d}");
    let (code, _out, err) = run_cli(
        &h2,
        &[
            "callers", "--repo", "e", "--path", "empty.py", "--line", "2", "--col", "1",
        ],
    );
    assert_eq!(code, 3);
    assert!(err.contains("no anchor here"), "{err}");
    let (code, _) = s2.finish();
    assert_eq!(code, 0);
    let (code, _) = s.finish();
    assert_eq!(code, 0);
}

#[test]
fn t4_impact_rejects_a_destination_parameter() {
    let h = home("cp4-t4d");
    let r = home("cp4-t4d-repo");
    let lines = graph_repo(&h, &r, "g", &[vec![1], vec![]]);
    let mut s = ready(&h);
    s.send(
        &json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": {"name": "riltra_code_impact", "arguments":
                {"repo": "g", "path": "graph.py", "line": lines[0], "col": 6, "to": "graph.py:1:6"}}
        })
        .to_string(),
    );
    let v = s.recv();
    assert_eq!(v["id"], 2);
    let rejected = v.get("error").is_some() || v.get("result").is_some_and(is_error);
    assert!(
        rejected,
        "impact with `to` must be rejected, not silently ignored: {v}"
    );
    // The supported call still works: impact takes no destination.
    let d = graph_doc(
        &mut s,
        3,
        "riltra_code_impact",
        &json!({"repo": "g", "path": "graph.py", "line": lines[0], "col": 6}),
    );
    assert_eq!(d["direction"], "impact", "{d}");
    assert_eq!(d["destination"], Value::Null, "{d}");
    let (code, _) = s.finish();
    assert_eq!(code, 0);
}

// --- T5: F1 graph read safety -------------------------------------------------

#[test]
fn t5_graph_legacy_v1_refused_without_migration_and_still_cli_migratable() {
    let h = home("cp4-t5l");
    let r = home("cp4-t5l-repo");
    std::fs::create_dir_all(&r).unwrap();
    std::fs::write(r.join("a.py"), "def target():\n    pass\n").unwrap();
    init_repo(&h, &r, "v1repo");
    let db = db_path(&h, "v1repo");
    std::fs::create_dir_all(db.parent().unwrap()).unwrap();
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(V1_TABLES).unwrap();
    }
    let mut s = ready(&h);
    let res = call_tool(
        &mut s,
        2,
        "riltra_code_callers",
        &json!({"repo": "v1repo", "path": "a.py", "line": 1, "col": 1}),
    );
    assert!(is_error(&res), "{res}");
    let d = doc(&res);
    assert_eq!(d["kind"], "schema_outdated", "{d}");
    let command = "riltra index status --repo v1repo";
    assert!(d["message"].as_str().unwrap().contains(command), "{d}");
    assert_eq!(user_version(&db), 1, "MCP must not migrate");
    // Executing exactly that command migrates the file through the CLI.
    let args: Vec<&str> = command.split_whitespace().skip(1).collect();
    let (code, out, err) = run_cli(&h, &args);
    assert_eq!(code, 0, "{out} {err}");
    assert_eq!(user_version(&db), 4, "CLI migration path intact");
    // The v1 fixture has no graph proof: the migrated store is still refused
    // by the graph guard (classified, never fabricated).
    let res = call_tool(
        &mut s,
        3,
        "riltra_code_callers",
        &json!({"repo": "v1repo", "path": "a.py", "line": 1, "col": 1}),
    );
    assert!(is_error(&res), "{res}");
    let d = doc(&res);
    assert_eq!(d["kind"], "store_error", "{d}");
    assert!(
        d["message"]
            .as_str()
            .unwrap()
            .starts_with("graph-proof-incompatible"),
        "{d}"
    );
    let (code, _) = s.finish();
    assert_eq!(code, 0);
}

#[test]
fn t5_graph_reads_leave_v4_untouched_and_survive_concurrent_rebuild() {
    let h = home("cp4-t5v");
    let r = home("cp4-t5v-repo");
    let lines = graph_repo(&h, &r, "g", &[vec![1], vec![2], vec![]]);
    let db = db_path(&h, "g");
    let version_before = user_version(&db);
    let schema_before = schema_dump(&db);
    let mut s = ready(&h);
    graph_doc(
        &mut s,
        2,
        "riltra_code_callers",
        &json!({"repo": "g", "path": "graph.py", "line": lines[2], "col": 6}),
    );
    graph_doc(
        &mut s,
        3,
        "riltra_code_callees",
        &json!({"repo": "g", "path": "graph.py", "line": lines[0], "col": 6}),
    );
    graph_doc(
        &mut s,
        4,
        "riltra_code_trace",
        &json!({"repo": "g", "path": "graph.py", "line": lines[0], "col": 6, "to": format!("graph.py:{}:6", lines[2])}),
    );
    graph_doc(
        &mut s,
        5,
        "riltra_code_impact",
        &json!({"repo": "g", "path": "graph.py", "line": lines[2], "col": 6}),
    );
    assert_eq!(user_version(&db), version_before);
    assert_eq!(
        schema_dump(&db),
        schema_before,
        "graph reads must not change the schema"
    );

    // Concurrent CLI rebuild: every graph call either answers from a
    // consistent revision or refuses with store_error; nothing mixes.
    let rebuild = {
        let h = h.clone();
        std::thread::spawn(move || {
            for _ in 0..3 {
                let (code, _out, err) = run_cli(&h, &["index", "--repo", "g"]);
                assert_eq!(code, 0, "concurrent rebuild failed: {err}");
            }
        })
    };
    for id in 6..=11 {
        let res = call_tool(
            &mut s,
            id,
            "riltra_code_callers",
            &json!({"repo": "g", "path": "graph.py", "line": lines[2], "col": 6}),
        );
        let d = doc(&res);
        if is_error(&res) {
            assert_eq!(
                d["kind"], "store_error",
                "unexpected classification during rebuild: {d}"
            );
        } else {
            assert_eq!(d["repo"], "g", "{d}");
            assert_eq!(d["schema_version"], 1, "{d}");
            assert!(d["snapshot"]["revision"].as_str().is_some(), "{d}");
        }
    }
    rebuild.join().unwrap();
    let (code, _) = s.finish();
    assert_eq!(code, 0);
    assert_eq!(user_version(&db), 4);
}

// --- CP5: knowledge tools ---------------------------------------------------

fn run_cli_stdin(cfg: &Path, args: &[&str], input: &[u8]) -> (i32, String, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_riltra"))
        .args(args)
        .env("RILTRA_HOME", cfg)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn riltra");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(input)
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait riltra");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn knowledge_db(cfg: &Path, name: &str) -> PathBuf {
    cfg.join("knowledge").join(format!("{name}.db"))
}

fn kcreate(cfg: &Path, repo: &str, payload: &str) {
    let (code, _out, err) = run_cli_stdin(
        cfg,
        &["knowledge", "create", "--repo", repo, "--input", "-"],
        payload.as_bytes(),
    );
    assert_eq!(code, 0, "knowledge create {repo}: {err}");
}

fn kupdate(cfg: &Path, repo: &str, id: &str, expected: u64, payload: &str) {
    let (code, _out, err) = run_cli_stdin(
        cfg,
        &[
            "knowledge",
            "update",
            "--repo",
            repo,
            "--id",
            id,
            "--expected-revision",
            &expected.to_string(),
            "--input",
            "-",
        ],
        payload.as_bytes(),
    );
    assert_eq!(code, 0, "knowledge update {id}: {err}");
}

fn klink(cfg: &Path, repo: &str, payload: &str) {
    let (code, _out, err) = run_cli_stdin(
        cfg,
        &["knowledge", "link", "--repo", repo, "--input", "-"],
        payload.as_bytes(),
    );
    assert_eq!(code, 0, "knowledge link: {err}");
}

fn kunlink(cfg: &Path, repo: &str, relation: &str, payload: &str) {
    let (code, _out, err) = run_cli_stdin(
        cfg,
        &[
            "knowledge",
            "unlink",
            "--repo",
            repo,
            "--relation",
            relation,
            "--input",
            "-",
        ],
        payload.as_bytes(),
    );
    assert_eq!(code, 0, "knowledge unlink {relation}: {err}");
}

/// One knowledge tool document; the shared `doc` helper already proves the
/// text payload parses and equals `structuredContent`.
fn kjson(s: &mut Session, id: i64, tool: &str, args: &Value) -> Value {
    doc(&call_tool(s, id, tool, args))
}

/// The parsed MCP document must equal the parsed CLI document exactly.
fn kparity(cfg: &Path, got: &Value, cli_args: &[&str]) {
    let expected = cli_json(cfg, cli_args);
    assert_eq!(got, &expected, "MCP and CLI must agree for {cli_args:?}");
}

/// A refusal: `isError`, classified, carrying the CLI's exact message text.
fn kerr(
    cfg: &Path,
    s: &mut Session,
    id: i64,
    tool: &str,
    args: &Value,
    cli_args: &[&str],
) -> Value {
    let res = call_tool(s, id, tool, args);
    assert!(is_error(&res), "{tool} must refuse: {res}");
    let d = doc(&res);
    let (code, _out, err) = run_cli(cfg, cli_args);
    assert_ne!(code, 0, "CLI must refuse too: {cli_args:?}");
    let message = err
        .strip_prefix("error: ")
        .expect("CLI failure carries 'error: '")
        .trim_end();
    assert_eq!(d["message"], message, "CLI message must be preserved");
    d
}

const K_D1: &str = r#"{"id":"d1","kind":"decision","title":"First","body":"kjson alpha","status":"active","author":"a","evidence":[]}"#;

#[test]
fn t2_knowledge_search_matches_cli_missing_empty_populated_paging_and_escaping() {
    let h = home("cp5-search");
    let r = home("cp5-search-repo");
    init_repo(&h, &r, "krepo");
    let mut s = ready(&h);
    let mut id = 2;

    // Missing store: empty generation-0 success, and nothing is created.
    let before = listing(&h);
    let got = kjson(
        &mut s,
        id,
        "riltra_knowledge_search",
        &json!({"repo": "krepo", "query": "kjson"}),
    );
    assert_eq!(got["generation"], json!(0));
    assert_eq!(got["result"]["hits"], json!([]));
    kparity(
        &h,
        &got,
        &["knowledge", "search", "--repo", "krepo", "--query", "kjson"],
    );
    assert!(
        !knowledge_db(&h, "krepo").exists(),
        "search created a store"
    );
    assert_eq!(listing(&h), before, "search touched storage");
    id += 1;

    // Empty store: a zero-record import still answers a valid empty page.
    let payload = h.join("empty.jsonl");
    std::fs::write(
        &payload,
        r#"{"type":"meta","format":"riltra-knowledge","version":1,"repo":"krepo","records":0,"relations":0}"#,
    )
    .unwrap();
    let (code, _out, err) = run_cli(
        &h,
        &[
            "knowledge",
            "import",
            "--repo",
            "krepo",
            "--file",
            payload.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "empty import: {err}");
    let got = kjson(
        &mut s,
        id,
        "riltra_knowledge_search",
        &json!({"repo": "krepo", "query": "kjson"}),
    );
    assert_eq!(got["generation"], json!(0));
    assert_eq!(got["result"]["hits"], json!([]));
    kparity(
        &h,
        &got,
        &["knowledge", "search", "--repo", "krepo", "--query", "kjson"],
    );
    id += 1;

    // Populated store including a record whose fields need JSON escaping.
    kcreate(&h, "krepo", K_D1);
    kcreate(
        &h,
        "krepo",
        r#"{"id":"d2","kind":"decision","title":"Second","body":"kjson beta","status":"active","author":"a","evidence":[]}"#,
    );
    kcreate(
        &h,
        "krepo",
        r#"{"id":"b1","kind":"benchmark","title":"Perf","body":"kjson bench","status":"active","author":"a","evidence":[],"measurement":{"method":"m","environment":"e","sample_count":3,"metrics":{"latency":{"value":1.5,"unit":"ms"}}}}"#,
    );
    let tricky = json!({
        "id": "e1",
        "kind": "decision",
        "title": "Quote \"Q\" back\\slash",
        "body": "kjson line1\nline2\ttab",
        "status": "active",
        "author": "a",
        "evidence": []
    })
    .to_string();
    kcreate(&h, "krepo", &tricky);
    for (mcp, cli) in [
        (
            json!({"repo": "krepo", "query": "kjson"}),
            vec!["--query", "kjson"],
        ),
        (
            json!({"repo": "krepo", "query": "kjson", "limit": 2}),
            vec!["--query", "kjson", "--limit", "2"],
        ),
        (
            json!({"repo": "krepo", "query": "kjson", "limit": 2, "offset": 2}),
            vec!["--query", "kjson", "--limit", "2", "--offset", "2"],
        ),
        (
            json!({"repo": "krepo", "query": "kjson", "kind": "benchmark"}),
            vec!["--query", "kjson", "--kind", "benchmark"],
        ),
        (
            json!({"repo": "krepo", "query": "zzznope"}),
            vec!["--query", "zzznope"],
        ),
        (
            json!({"repo": "krepo", "query": "kjson", "include_archived": true}),
            vec!["--query", "kjson", "--include-archived"],
        ),
        (
            json!({"repo": "krepo", "query": "kjson", "generation": 4}),
            vec!["--query", "kjson", "--generation", "4"],
        ),
    ] {
        let got = kjson(&mut s, id, "riltra_knowledge_search", &mcp);
        let mut args = vec!["knowledge", "search", "--repo", "krepo"];
        args.extend(cli);
        kparity(&h, &got, &args);
        id += 1;
    }

    // Ordering is by record id and the page bounds are exact.
    let got = kjson(
        &mut s,
        id,
        "riltra_knowledge_search",
        &json!({"repo": "krepo", "query": "kjson", "limit": 2}),
    );
    assert_eq!(
        got["pagination"],
        json!({"limit": 2, "offset": 0, "total": 4, "truncated": true, "next_offset": 2})
    );
    let hits = got["result"]["hits"].as_array().unwrap();
    assert_eq!(hits[0]["id"], json!("b1"));
    assert_eq!(hits[1]["id"], json!("d1"));
    id += 1;

    // Escaped fields round-trip byte-exactly.
    let got = kjson(
        &mut s,
        id,
        "riltra_knowledge_search",
        &json!({"repo": "krepo", "query": "kjson"}),
    );
    assert_eq!(got["generation"], json!(4));
    let e1 = got["result"]["hits"]
        .as_array()
        .unwrap()
        .iter()
        .find(|hit| hit["id"] == json!("e1"))
        .unwrap();
    assert_eq!(e1["title"], json!("Quote \"Q\" back\\slash"));
    assert_eq!(e1["body"], json!("kjson line1\nline2\ttab"));
    id += 1;

    // Visibility flags: superseded and retracted records stay hidden unless
    // explicitly requested, with their lifecycle payloads.
    kupdate(
        &h,
        "krepo",
        "d2",
        1,
        r#"{"title":"Second v2","body":"kjson beta v2","status":"superseded","author":"a","evidence":[],"superseded_by":{"id":"d1","revision":1},"reason":"replaced by d1"}"#,
    );
    kupdate(
        &h,
        "krepo",
        "b1",
        1,
        r#"{"title":"Perf","body":"kjson bench","status":"retracted","author":"a","evidence":[],"reason":"wrong"}"#,
    );
    for (mcp, cli) in [
        (
            json!({"repo": "krepo", "query": "kjson", "include_superseded": true}),
            vec!["--query", "kjson", "--include-superseded"],
        ),
        (
            json!({"repo": "krepo", "query": "kjson", "include_retracted": true}),
            vec!["--query", "kjson", "--include-retracted"],
        ),
        (
            json!({"repo": "krepo", "query": "kjson", "include_archived": true, "include_superseded": true, "include_retracted": true}),
            vec![
                "--query",
                "kjson",
                "--include-archived",
                "--include-superseded",
                "--include-retracted",
            ],
        ),
        (
            json!({"repo": "krepo", "query": "kjson"}),
            vec!["--query", "kjson"],
        ),
    ] {
        let got = kjson(&mut s, id, "riltra_knowledge_search", &mcp);
        let mut args = vec!["knowledge", "search", "--repo", "krepo"];
        args.extend(cli);
        kparity(&h, &got, &args);
        id += 1;
    }
    let got = kjson(
        &mut s,
        id,
        "riltra_knowledge_search",
        &json!({"repo": "krepo", "query": "kjson"}),
    );
    let ids: Vec<&str> = got["result"]["hits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hit| hit["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["d1", "e1"]);
    id += 1;
    let got = kjson(
        &mut s,
        id,
        "riltra_knowledge_search",
        &json!({"repo": "krepo", "query": "kjson", "include_archived": true, "include_superseded": true, "include_retracted": true}),
    );
    let ids: Vec<&str> = got["result"]["hits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hit| hit["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["b1", "d1", "d2", "e1"]);
    id += 1;

    // Repository isolation: a second registered repo in the same home never
    // sees this store.
    let other = home("cp5-search-other");
    init_repo(&h, &other, "other");
    let got = kjson(
        &mut s,
        id,
        "riltra_knowledge_search",
        &json!({"repo": "other", "query": "kjson"}),
    );
    assert_eq!(got["generation"], json!(0));
    assert_eq!(got["result"]["hits"], json!([]));
    id += 1;
    let res = call_tool(
        &mut s,
        id,
        "riltra_knowledge_show",
        &json!({"repo": "other", "id": "d1"}),
    );
    assert!(is_error(&res), "{res}");
    assert_eq!(doc(&res)["kind"], json!("store_missing"));

    let (code, _) = s.finish();
    assert_eq!(code, 0);
}

#[test]
fn r1_missing_store_generation_guard_matches_cli() {
    let h = home("cp5-r1");
    let r = home("cp5-r1-repo");
    init_repo(&h, &r, "krepo");
    let db = knowledge_db(&h, "krepo");
    let before = listing(&h);

    let mut s = ready(&h);
    // Omitted and 0: valid generation-0 empty success.
    let got = kjson(
        &mut s,
        2,
        "riltra_knowledge_search",
        &json!({"repo": "krepo", "query": "x"}),
    );
    assert_eq!(got["generation"], json!(0));
    assert_eq!(got["result"]["hits"], json!([]));
    kparity(
        &h,
        &got,
        &["knowledge", "search", "--repo", "krepo", "--query", "x"],
    );
    let got = kjson(
        &mut s,
        3,
        "riltra_knowledge_search",
        &json!({"repo": "krepo", "query": "x", "generation": 0}),
    );
    assert_eq!(got["generation"], json!(0));
    kparity(
        &h,
        &got,
        &[
            "knowledge",
            "search",
            "--repo",
            "krepo",
            "--query",
            "x",
            "--generation",
            "0",
        ],
    );

    // Any nonzero generation conflicts against the absent store's generation 0.
    for (id, generation) in [(4, 1), (5, 7)] {
        let text = generation.to_string();
        let d = kerr(
            &h,
            &mut s,
            id,
            "riltra_knowledge_search",
            &json!({"repo": "krepo", "query": "x", "generation": generation}),
            &[
                "knowledge",
                "search",
                "--repo",
                "krepo",
                "--query",
                "x",
                "--generation",
                &text,
            ],
        );
        assert_eq!(d["kind"], "conflict", "{d}");
        assert_eq!(
            d["message"],
            json!(format!(
                "knowledge generation conflict: expected {generation} but store is at 0"
            )),
            "{d}"
        );
    }

    // An unregistered repository is still not_registered, never empty success.
    let d = kerr(
        &h,
        &mut s,
        6,
        "riltra_knowledge_search",
        &json!({"repo": "nope", "query": "x", "generation": 5}),
        &[
            "knowledge",
            "search",
            "--repo",
            "nope",
            "--query",
            "x",
            "--generation",
            "5",
        ],
    );
    assert_eq!(d["kind"], "not_registered", "{d}");

    let (code, _) = s.finish();
    assert_eq!(code, 0);
    assert!(!db.exists(), "no knowledge database may be created");
    assert_eq!(
        listing(&h),
        before,
        "missing-store reads must create nothing"
    );
}

#[test]
fn t3_knowledge_show_matches_cli_revisions_lifecycle_and_evidence() {
    let h = home("cp5-show");
    let r = home("cp5-show-repo");
    init_repo(&h, &r, "krepo");
    std::fs::write(r.join("a.py"), "alpha beta gamma").unwrap();
    kcreate(
        &h,
        "krepo",
        r#"{"id":"d1","kind":"decision","title":"First","body":"body one","status":"active","author":"a","evidence":[{"path":"a.py"}]}"#,
    );
    kcreate(
        &h,
        "krepo",
        r#"{"id":"d2","kind":"decision","title":"Second","body":"body two","status":"active","author":"a","evidence":[]}"#,
    );
    kupdate(
        &h,
        "krepo",
        "d1",
        1,
        r#"{"title":"First v2","body":"body one v2","status":"superseded","author":"a","evidence":[{"path":"a.py"}],"superseded_by":{"id":"d2","revision":1},"reason":"replaced by d2"}"#,
    );
    kupdate(
        &h,
        "krepo",
        "d2",
        1,
        r#"{"title":"Second v2","body":"body two v2","status":"active","author":"a","evidence":[],"confirm_state":"confirmed"}"#,
    );

    let mut s = ready(&h);
    let mut id = 2;
    for (mcp, cli) in [
        (json!({"repo": "krepo", "id": "d1"}), vec!["--id", "d1"]),
        (
            json!({"repo": "krepo", "id": "d1", "revision": 1}),
            vec!["--id", "d1", "--revision", "1"],
        ),
        (
            json!({"repo": "krepo", "id": "d1", "revision": 2}),
            vec!["--id", "d1", "--revision", "2"],
        ),
        (json!({"repo": "krepo", "id": "d2"}), vec!["--id", "d2"]),
    ] {
        let got = kjson(&mut s, id, "riltra_knowledge_show", &mcp);
        let mut args = vec!["knowledge", "show", "--repo", "krepo"];
        args.extend(cli);
        kparity(&h, &got, &args);
        id += 1;
    }

    // Supersession comes from the stored lifecycle events; the old revision
    // stays addressable and carries no full-stage fields.
    let got = kjson(
        &mut s,
        id,
        "riltra_knowledge_show",
        &json!({"repo": "krepo", "id": "d1"}),
    );
    assert_eq!(got["generation"], json!(4));
    assert_eq!(got["result"]["revision"], json!(2));
    assert_eq!(got["result"]["status"], json!("superseded"));
    assert_eq!(
        got["result"]["superseded_by"],
        json!({"id": "d2", "revision": 1})
    );
    assert_eq!(
        got["result"]["evidence"][0]["freshness"]["status"],
        json!("fresh")
    );
    assert_eq!(got["result"]["verification"], json!("Unverified"));
    assert_eq!(got["result"]["support"], json!("unverified"));
    id += 1;
    let got = kjson(
        &mut s,
        id,
        "riltra_knowledge_show",
        &json!({"repo": "krepo", "id": "d1", "revision": 1}),
    );
    assert!(got["result"].get("superseded_by").is_none(), "{got}");
    assert_eq!(got["result"]["confirm_state"], json!("candidate"));
    id += 1;

    // Confirmation is provenance, never promotion: d2 is confirmed on a v2
    // store and still reports Unverified.
    let got = kjson(
        &mut s,
        id,
        "riltra_knowledge_show",
        &json!({"repo": "krepo", "id": "d2"}),
    );
    assert_eq!(got["result"]["confirm_state"], json!("confirmed"));
    assert_eq!(got["result"]["verification"], json!("Unverified"));
    id += 1;

    // Edited bytes: freshness turns stale, the bound path is unchanged.
    std::fs::write(r.join("a.py"), "alpha beta gamma changed").unwrap();
    let got = kjson(
        &mut s,
        id,
        "riltra_knowledge_show",
        &json!({"repo": "krepo", "id": "d1", "revision": 1}),
    );
    assert_eq!(
        got["result"]["evidence"][0]["freshness"]["status"],
        json!("stale")
    );
    kparity(
        &h,
        &got,
        &[
            "knowledge",
            "show",
            "--repo",
            "krepo",
            "--id",
            "d1",
            "--revision",
            "1",
        ],
    );

    let (code, _) = s.finish();
    assert_eq!(code, 0);
}

#[test]
fn r2_knowledge_show_snapshot_on_v1_and_under_concurrent_writes() {
    // Foundation v1 store: latest and historical shows are unchanged and the
    // read never migrates.
    let h1 = home("cp5-r2-v1");
    let r1 = home("cp5-r2-v1-repo");
    init_repo(&h1, &r1, "v1repo");
    kcreate(&h1, "v1repo", K_D1);
    let db1 = knowledge_db(&h1, "v1repo");
    assert_eq!(user_version(&db1), 1);
    let mut s = ready(&h1);
    for (id, mcp, cli) in [
        (2, json!({"repo": "v1repo", "id": "d1"}), vec!["--id", "d1"]),
        (
            3,
            json!({"repo": "v1repo", "id": "d1", "revision": 1}),
            vec!["--id", "d1", "--revision", "1"],
        ),
    ] {
        let got = kjson(&mut s, id, "riltra_knowledge_show", &mcp);
        assert_eq!(got["generation"], json!(1), "{got}");
        let mut args = vec!["knowledge", "show", "--repo", "v1repo"];
        args.extend(cli);
        kparity(&h1, &got, &args);
    }
    let (code, _) = s.finish();
    assert_eq!(code, 0);
    assert_eq!(user_version(&db1), 1, "v1 show must not migrate");

    // Concurrent writer: with only d1 and only updates, the generation equals
    // d1's revision after every commit. Every envelope must therefore pair a
    // revision with the generation from the same snapshot.
    let h = home("cp5-r2-live");
    let r = home("cp5-r2-live-repo");
    init_repo(&h, &r, "krepo");
    kcreate(&h, "krepo", K_D1);
    let mut s = ready(&h);
    let writer = {
        let h = h.clone();
        std::thread::spawn(move || {
            for revision in 1..=8 {
                kupdate(
                    &h,
                    "krepo",
                    "d1",
                    revision,
                    &format!(
                        r#"{{"title":"Live {revision}","body":"body {revision}","status":"active","author":"a","evidence":[]}}"#
                    ),
                );
            }
        })
    };
    for id in 10..18 {
        let got = kjson(
            &mut s,
            id,
            "riltra_knowledge_show",
            &json!({"repo": "krepo", "id": "d1"}),
        );
        assert_eq!(got["result"]["id"], json!("d1"), "{got}");
        assert_eq!(
            got["generation"].as_u64(),
            got["result"]["revision"].as_u64(),
            "record and envelope must come from one snapshot: {got}"
        );
    }
    writer.join().unwrap();
    let (code, _) = s.finish();
    assert_eq!(code, 0);
}

#[test]
fn t4_knowledge_related_matches_cli_directions_retracted_and_paging() {
    let h = home("cp5-related");
    let r = home("cp5-related-repo");
    init_repo(&h, &r, "krepo");
    kcreate(&h, "krepo", K_D1);
    kcreate(
        &h,
        "krepo",
        r#"{"id":"d2","kind":"decision","title":"Second","body":"body two","status":"active","author":"a","evidence":[]}"#,
    );
    kcreate(
        &h,
        "krepo",
        r#"{"id":"d3","kind":"decision","title":"Third","body":"body three","status":"active","author":"a","evidence":[]}"#,
    );
    klink(
        &h,
        "krepo",
        r#"{"relation_id":"r1","src_id":"d2","src_revision":1,"dst_id":"d1","dst_revision":1,"kind":"supports","author":"a","rationale":"because","confirm_state":"confirmed"}"#,
    );
    klink(
        &h,
        "krepo",
        r#"{"relation_id":"r2","src_id":"d3","src_revision":1,"dst_id":"d1","dst_revision":1,"kind":"relates_to","author":"a","rationale":"maybe"}"#,
    );
    klink(
        &h,
        "krepo",
        r#"{"relation_id":"r3","src_id":"d3","src_revision":1,"dst_id":"d2","dst_revision":1,"kind":"relates_to","author":"a","rationale":"gone"}"#,
    );
    kunlink(&h, "krepo", "r3", r#"{"author":"a","reason":"mistake"}"#);
    // d3 moves to revision 2: the revision-1 bindings stay addressable.
    kupdate(
        &h,
        "krepo",
        "d3",
        1,
        r#"{"title":"Third v2","body":"body three v2","status":"active","author":"a","evidence":[]}"#,
    );

    let mut s = ready(&h);
    let generation = kjson(
        &mut s,
        2,
        "riltra_knowledge_show",
        &json!({"repo": "krepo", "id": "d1"}),
    )["generation"]
        .as_u64()
        .unwrap();
    let generation_text = generation.to_string();

    let mut id = 3;
    for (mcp, cli) in [
        // `direction` defaults to outbound.
        (json!({"repo": "krepo", "id": "d2"}), vec!["--id", "d2"]),
        (
            json!({"repo": "krepo", "id": "d1", "direction": "inbound"}),
            vec!["--id", "d1", "--direction", "inbound"],
        ),
        (
            json!({"repo": "krepo", "id": "d3", "revision": 1}),
            vec!["--id", "d3", "--revision", "1"],
        ),
        (json!({"repo": "krepo", "id": "d3"}), vec!["--id", "d3"]),
        (
            json!({"repo": "krepo", "id": "d2", "direction": "inbound"}),
            vec!["--id", "d2", "--direction", "inbound"],
        ),
        (
            json!({"repo": "krepo", "id": "d2", "direction": "inbound", "include_retracted": true}),
            vec![
                "--id",
                "d2",
                "--direction",
                "inbound",
                "--include-retracted",
            ],
        ),
        (
            json!({"repo": "krepo", "id": "d1", "direction": "inbound", "limit": 1}),
            vec!["--id", "d1", "--direction", "inbound", "--limit", "1"],
        ),
        (
            json!({"repo": "krepo", "id": "d1", "direction": "inbound", "limit": 1, "offset": 1}),
            vec![
                "--id",
                "d1",
                "--direction",
                "inbound",
                "--limit",
                "1",
                "--offset",
                "1",
            ],
        ),
        (
            json!({"repo": "krepo", "id": "d1", "direction": "inbound", "generation": generation}),
            vec![
                "--id",
                "d1",
                "--direction",
                "inbound",
                "--generation",
                generation_text.as_str(),
            ],
        ),
    ] {
        let got = kjson(&mut s, id, "riltra_knowledge_related", &mcp);
        let mut args = vec!["knowledge", "related", "--repo", "krepo"];
        args.extend(cli);
        kparity(&h, &got, &args);
        id += 1;
    }

    // Retracted relations: hidden by default, explicit opt-in, payload kept.
    let got = kjson(
        &mut s,
        id,
        "riltra_knowledge_related",
        &json!({"repo": "krepo", "id": "d2", "direction": "inbound", "include_retracted": true}),
    );
    let rel = &got["result"]["relations"][0];
    assert_eq!(rel["relation_id"], json!("r3"));
    assert_eq!(rel["retracted"], json!(true));
    assert_eq!(rel["retraction_author"], json!("a"));
    assert_eq!(rel["retraction_reason"], json!("mistake"));
    id += 1;
    let got = kjson(
        &mut s,
        id,
        "riltra_knowledge_related",
        &json!({"repo": "krepo", "id": "d2", "direction": "inbound"}),
    );
    assert_eq!(got["result"]["relations"], json!([]));
    id += 1;

    // Confirmation rides only where a stored event exists; head revision 2
    // of d3 has no pinned relations while revision 1 keeps its binding.
    let got = kjson(
        &mut s,
        id,
        "riltra_knowledge_related",
        &json!({"repo": "krepo", "id": "d2"}),
    );
    assert_eq!(
        got["result"]["relations"][0]["confirm_state"],
        json!("confirmed")
    );
    id += 1;
    let got = kjson(
        &mut s,
        id,
        "riltra_knowledge_related",
        &json!({"repo": "krepo", "id": "d3"}),
    );
    assert_eq!(got["result"]["relations"], json!([]));
    id += 1;
    let got = kjson(
        &mut s,
        id,
        "riltra_knowledge_related",
        &json!({"repo": "krepo", "id": "d3", "revision": 1}),
    );
    assert_eq!(got["result"]["relations"][0]["relation_id"], json!("r2"));
    id += 1;

    // Page bounds are exact.
    let got = kjson(
        &mut s,
        id,
        "riltra_knowledge_related",
        &json!({"repo": "krepo", "id": "d1", "direction": "inbound", "limit": 1}),
    );
    assert_eq!(
        got["pagination"],
        json!({"limit": 1, "offset": 0, "total": 2, "truncated": true, "next_offset": 1})
    );

    let (code, _) = s.finish();
    assert_eq!(code, 0);
}

#[test]
fn t4_knowledge_error_taxonomy_matches_cli_texts() {
    let h = home("cp5-errors");
    let r = home("cp5-errors-repo");
    init_repo(&h, &r, "krepo");
    let mut s = ready(&h);

    // Repository problems: not_registered with the CLI text.
    for (id, tool, mcp, cli) in [
        (
            2,
            "riltra_knowledge_search",
            json!({"repo": "ghost", "query": "x"}),
            vec!["knowledge", "search", "--repo", "ghost", "--query", "x"],
        ),
        (
            3,
            "riltra_knowledge_show",
            json!({"repo": "ghost", "id": "d1"}),
            vec!["knowledge", "show", "--repo", "ghost", "--id", "d1"],
        ),
        (
            4,
            "riltra_knowledge_related",
            json!({"repo": "ghost", "id": "d1"}),
            vec!["knowledge", "related", "--repo", "ghost", "--id", "d1"],
        ),
    ] {
        let d = kerr(&h, &mut s, id, tool, &mcp, &cli);
        assert_eq!(d["kind"], "not_registered", "{tool}: {d}");
    }

    // Missing store: show/related refuse as store_missing, nothing created.
    let before = listing(&h);
    let d = kerr(
        &h,
        &mut s,
        5,
        "riltra_knowledge_show",
        &json!({"repo": "krepo", "id": "d1"}),
        &["knowledge", "show", "--repo", "krepo", "--id", "d1"],
    );
    assert_eq!(d["kind"], "store_missing", "{d}");
    let d = kerr(
        &h,
        &mut s,
        6,
        "riltra_knowledge_related",
        &json!({"repo": "krepo", "id": "d1"}),
        &["knowledge", "related", "--repo", "krepo", "--id", "d1"],
    );
    assert_eq!(d["kind"], "store_missing", "{d}");
    assert_eq!(listing(&h), before, "refusals must not create storage");

    kcreate(&h, "krepo", K_D1);

    // Input problems: invalid_input with the CLI exit-2 texts.
    for (id, tool, mcp, cli) in [
        (
            7,
            "riltra_knowledge_search",
            json!({"repo": "krepo", "query": "body", "kind": "bogus"}),
            vec![
                "knowledge",
                "search",
                "--repo",
                "krepo",
                "--query",
                "body",
                "--kind",
                "bogus",
            ],
        ),
        (
            8,
            "riltra_knowledge_search",
            json!({"repo": "krepo", "query": "body", "limit": 0}),
            vec![
                "knowledge",
                "search",
                "--repo",
                "krepo",
                "--query",
                "body",
                "--limit",
                "0",
            ],
        ),
        (
            9,
            "riltra_knowledge_search",
            json!({"repo": "krepo", "query": "body", "limit": 1001}),
            vec![
                "knowledge",
                "search",
                "--repo",
                "krepo",
                "--query",
                "body",
                "--limit",
                "1001",
            ],
        ),
        (
            10,
            "riltra_knowledge_show",
            json!({"repo": "krepo", "id": "missing"}),
            vec!["knowledge", "show", "--repo", "krepo", "--id", "missing"],
        ),
        (
            11,
            "riltra_knowledge_show",
            json!({"repo": "krepo", "id": "d1", "revision": 99}),
            vec![
                "knowledge",
                "show",
                "--repo",
                "krepo",
                "--id",
                "d1",
                "--revision",
                "99",
            ],
        ),
        (
            12,
            "riltra_knowledge_related",
            json!({"repo": "krepo", "id": "missing"}),
            vec!["knowledge", "related", "--repo", "krepo", "--id", "missing"],
        ),
        (
            13,
            "riltra_knowledge_related",
            json!({"repo": "krepo", "id": "d1", "direction": "sideways"}),
            vec![
                "knowledge",
                "related",
                "--repo",
                "krepo",
                "--id",
                "d1",
                "--direction",
                "sideways",
            ],
        ),
    ] {
        let d = kerr(&h, &mut s, id, tool, &mcp, &cli);
        assert_eq!(d["kind"], "invalid_input", "{tool}: {d}");
    }

    // Stale generation: conflict with the CLI's message.
    let d = kerr(
        &h,
        &mut s,
        14,
        "riltra_knowledge_search",
        &json!({"repo": "krepo", "query": "body", "generation": 99}),
        &[
            "knowledge",
            "search",
            "--repo",
            "krepo",
            "--query",
            "body",
            "--generation",
            "99",
        ],
    );
    assert_eq!(d["kind"], "conflict", "{d}");
    let d = kerr(
        &h,
        &mut s,
        15,
        "riltra_knowledge_related",
        &json!({"repo": "krepo", "id": "d1", "generation": 99}),
        &[
            "knowledge",
            "related",
            "--repo",
            "krepo",
            "--id",
            "d1",
            "--generation",
            "99",
        ],
    );
    assert_eq!(d["kind"], "conflict", "{d}");

    let (code, _) = s.finish();
    assert_eq!(code, 0);
}

#[test]
fn t5_knowledge_reads_leave_stores_untouched_and_create_nothing() {
    let h = home("cp5-readonly");
    let r = home("cp5-readonly-repo");
    init_repo(&h, &r, "krepo");
    kcreate(&h, "krepo", K_D1);
    let db = knowledge_db(&h, "krepo");
    assert_eq!(user_version(&db), 1);
    let schema_v1 = schema_dump(&db);

    // Foundation v1 store: reads never migrate.
    let mut s = ready(&h);
    kjson(
        &mut s,
        2,
        "riltra_knowledge_search",
        &json!({"repo": "krepo", "query": "kjson"}),
    );
    kjson(
        &mut s,
        3,
        "riltra_knowledge_show",
        &json!({"repo": "krepo", "id": "d1"}),
    );
    kjson(
        &mut s,
        4,
        "riltra_knowledge_related",
        &json!({"repo": "krepo", "id": "d1"}),
    );
    let (code, _) = s.finish();
    assert_eq!(code, 0);
    assert_eq!(user_version(&db), 1, "v1 read must not migrate");
    assert_eq!(schema_dump(&db), schema_v1, "v1 schema must be untouched");

    // Full-stage v2 store: reads plus concurrent CLI writes stay consistent.
    kupdate(
        &h,
        "krepo",
        "d1",
        1,
        r#"{"title":"First v2","body":"kjson alpha v2","status":"active","author":"a","evidence":[],"confirm_state":"confirmed"}"#,
    );
    assert_eq!(user_version(&db), 2);
    let schema_v2 = schema_dump(&db);
    let mut s = ready(&h);
    let writer = {
        let h = h.clone();
        std::thread::spawn(move || {
            for i in 0..3 {
                kcreate(
                    &h,
                    "krepo",
                    &format!(
                        r#"{{"id":"w{i}","kind":"decision","title":"W{i}","body":"kjson w{i}","status":"active","author":"a","evidence":[]}}"#
                    ),
                );
            }
        })
    };
    for id in 10..16 {
        let got = kjson(
            &mut s,
            id,
            "riltra_knowledge_search",
            &json!({"repo": "krepo", "query": "kjson"}),
        );
        assert_eq!(got["repo"], json!("krepo"), "{got}");
        assert_eq!(got["operation"], json!("search"), "{got}");
        assert!(got["generation"].as_u64().is_some(), "{got}");
    }
    writer.join().unwrap();
    let (code, _) = s.finish();
    assert_eq!(code, 0);
    assert_eq!(user_version(&db), 2);
    assert_eq!(schema_dump(&db), schema_v2, "v2 schema must be untouched");

    // Missing store: the empty success and both refusals create nothing.
    let hm = home("cp5-readonly-missing");
    let rm = home("cp5-readonly-missing-repo");
    init_repo(&hm, &rm, "krepo");
    let before = listing(&hm);
    let mut s = ready(&hm);
    let got = kjson(
        &mut s,
        2,
        "riltra_knowledge_search",
        &json!({"repo": "krepo", "query": "x"}),
    );
    assert_eq!(got["generation"], json!(0));
    assert!(is_error(&call_tool(
        &mut s,
        3,
        "riltra_knowledge_show",
        &json!({"repo": "krepo", "id": "d1"})
    )));
    assert!(is_error(&call_tool(
        &mut s,
        4,
        "riltra_knowledge_related",
        &json!({"repo": "krepo", "id": "d1"})
    )));
    let (code, _) = s.finish();
    assert_eq!(code, 0);
    assert_eq!(listing(&hm), before, "nothing may be created");
    assert!(!knowledge_db(&hm, "krepo").exists());
}

#[test]
fn t5_knowledge_hostile_or_corrupt_store_fails_closed() {
    let h = home("cp5-hostile");
    let r = home("cp5-hostile-repo");
    init_repo(&h, &r, "krepo");
    let db = knowledge_db(&h, "krepo");
    std::fs::create_dir_all(db.parent().unwrap()).unwrap();

    // Directory at the store path: refuse, never read as empty.
    std::fs::create_dir_all(&db).unwrap();
    let mut s = ready(&h);
    let d = kerr(
        &h,
        &mut s,
        2,
        "riltra_knowledge_search",
        &json!({"repo": "krepo", "query": "x"}),
        &["knowledge", "search", "--repo", "krepo", "--query", "x"],
    );
    assert_eq!(d["kind"], "store_error", "{d}");
    let (code, _) = s.finish();
    assert_eq!(code, 0);
    std::fs::remove_dir_all(&db).unwrap();

    // Corrupt bytes: every tool refuses as store_error, bytes stay put.
    std::fs::write(&db, b"not a knowledge store").unwrap();
    let before = std::fs::read(&db).unwrap();
    let mut s = ready(&h);
    for (id, tool, mcp, cli) in [
        (
            2,
            "riltra_knowledge_search",
            json!({"repo": "krepo", "query": "x"}),
            vec!["knowledge", "search", "--repo", "krepo", "--query", "x"],
        ),
        (
            3,
            "riltra_knowledge_show",
            json!({"repo": "krepo", "id": "d1"}),
            vec!["knowledge", "show", "--repo", "krepo", "--id", "d1"],
        ),
        (
            4,
            "riltra_knowledge_related",
            json!({"repo": "krepo", "id": "d1"}),
            vec!["knowledge", "related", "--repo", "krepo", "--id", "d1"],
        ),
    ] {
        let d = kerr(&h, &mut s, id, tool, &mcp, &cli);
        assert_eq!(d["kind"], "store_error", "{tool}: {d}");
    }
    let (code, _) = s.finish();
    assert_eq!(code, 0);
    assert_eq!(
        std::fs::read(&db).unwrap(),
        before,
        "corrupt store must be byte-untouched"
    );
    std::fs::remove_file(&db).unwrap();

    // Symlinked store path: fail closed through the link guard.
    let elsewhere = h.join("elsewhere.db");
    std::fs::write(&elsewhere, b"not a store").unwrap();
    if !try_file_symlink(&elsewhere, &db) {
        return; // OS refused symlinks here; nothing to prove.
    }
    let mut s = ready(&h);
    for (id, tool, mcp, cli) in [
        (
            2,
            "riltra_knowledge_search",
            json!({"repo": "krepo", "query": "x"}),
            vec!["knowledge", "search", "--repo", "krepo", "--query", "x"],
        ),
        (
            3,
            "riltra_knowledge_show",
            json!({"repo": "krepo", "id": "d1"}),
            vec!["knowledge", "show", "--repo", "krepo", "--id", "d1"],
        ),
        (
            4,
            "riltra_knowledge_related",
            json!({"repo": "krepo", "id": "d1"}),
            vec!["knowledge", "related", "--repo", "krepo", "--id", "d1"],
        ),
    ] {
        let d = kerr(&h, &mut s, id, tool, &mcp, &cli);
        assert_eq!(d["kind"], "store_error", "{tool}: {d}");
    }
    let (code, _) = s.finish();
    assert_eq!(code, 0);
    assert!(
        std::fs::symlink_metadata(&db)
            .unwrap()
            .file_type()
            .is_symlink(),
        "the link must survive the refusal"
    );
    assert_eq!(std::fs::read(&elsewhere).unwrap(), b"not a store");
}
