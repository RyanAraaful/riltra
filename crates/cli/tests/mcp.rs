// SPDX-License-Identifier: Apache-2.0
//! M8 CP1: `riltra mcp` stdio server skeleton. Each test spawns the real
//! binary with an isolated `RILTRA_HOME` and speaks newline-delimited
//! JSON-RPC 2.0 over stdin/stdout (the MCP stdio transport framing).

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

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
fn mcp_tools_list_is_empty() {
    let h = home("tools");
    let mut s = spawn(&h);
    s.send(initialize_request());
    s.recv();
    s.send(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
    s.send(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#);
    let v = s.recv();
    assert_eq!(v["id"], 2);
    let tools = v["result"]["tools"].as_array().expect("tools array");
    assert!(tools.is_empty(), "CP1 advertises no tools, got {tools:?}");
    let (code, _) = s.finish();
    assert_eq!(code, 0);
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
