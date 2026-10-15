// SPDX-License-Identifier: Apache-2.0
//! M5 LSP subprocess adapter: explicit opt-in, std-only JSON-RPC over
//! Content-Length framing. No shell, no auto-install, no async runtime.
//! The child is untrusted (servers may run hooks/builds/network): absolute
//! executable path only, bounded messages/timeouts, stderr kept off the
//! protocol stream, kill + reap on every failure path.

// Allow dead code until Task 7 wires the CLI (staged plan, then remove).
#![allow(dead_code)]

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;

use crate::semantic::{self, Evidence};

/// Explicit server launch description. The executable path must be absolute;
/// argument strings are passed as argv, never through a shell.
pub struct ServerSpec {
    pub exe: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub root: PathBuf,
    pub timeout: Duration,
    pub max_msg_bytes: usize,
    pub max_messages: usize,
}

impl ServerSpec {
    pub fn new(exe: &str, root: &Path) -> Result<ServerSpec, String> {
        let path = Path::new(exe);
        if !path.is_absolute() {
            return Err("language server path must be absolute".to_string());
        }
        Ok(ServerSpec {
            exe: exe.to_string(),
            args: Vec::new(),
            env: Vec::new(),
            root: root.to_path_buf(),
            timeout: Duration::from_secs(30),
            max_msg_bytes: 8 * 1024 * 1024,
            max_messages: 10_000,
        })
    }
}

/// One live answer set with the negotiated encoding and server diagnostics.
#[derive(Debug)]
pub struct LspEvidence {
    pub facts: Vec<Evidence>,
    pub server_version: String,
    pub position_encoding: String,
    pub diagnostics: Vec<String>,
}

/// Digest binding server identity to a repo revision (one third of the
/// Task 6 tripartite binding; a claim, never proof by itself).
pub fn source_binding(server: &ServerSpec, head: &str) -> String {
    let mut material = Vec::new();
    material.extend_from_slice(server.exe.as_bytes());
    material.push(0);
    for arg in &server.args {
        material.extend_from_slice(arg.as_bytes());
        material.push(0);
    }
    material.extend_from_slice(head.as_bytes());
    crate::store::content_digest(&material)
}

/// Encode one JSON-RPC message with Content-Length framing.
pub fn encode_message(body: &Value) -> Result<Vec<u8>, String> {
    let text = serde_json::to_string(body).map_err(|e| format!("cannot encode request: {e}"))?;
    let mut out = format!("Content-Length: {}\r\n\r\n", text.len()).into_bytes();
    out.extend_from_slice(text.as_bytes());
    Ok(out)
}

/// Read one framed message. Any non-framing bytes are a protocol error,
/// never skipped: a chatty server must not desynchronize the stream.
pub fn read_message<R: Read>(mut reader: R, max_bytes: usize) -> Result<Value, String> {
    let mut header = Vec::new();
    let mut byte = [0u8; 1];
    // Scan headers byte-at-a-time so a pollution prefix fails loudly at
    // the first line instead of being buffered past.
    loop {
        reader
            .read_exact(&mut byte)
            .map_err(|_| "unexpected EOF in headers".to_string())?;
        header.push(byte[0]);
        if header.len() > 8192 {
            return Err("headers too large".to_string());
        }
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
        // A bare LF line or garbage before any colon: not framing.
        if byte[0] == b'\n' && !header.ends_with(b"\r\n\r\n") {
            let text = String::from_utf8_lossy(&header);
            if !text.lines().next().is_some_and(|l| l.contains(':')) {
                return Err(format!("non-framing stdout: '{text}'"));
            }
        }
    }
    let text = String::from_utf8(header).map_err(|_| "non-UTF8 headers".to_string())?;
    let mut length: Option<usize> = None;
    for line in text.split("\r\n") {
        if line.is_empty() {
            continue;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| format!("bad header line: '{line}'"))?;
        if name.trim().eq_ignore_ascii_case("content-length") {
            length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| "bad Content-Length".to_string())?,
            );
        }
    }
    let length = length.ok_or_else(|| "missing Content-Length".to_string())?;
    if length > max_bytes {
        return Err("message exceeds max_msg_bytes".to_string());
    }
    let mut body = vec![0u8; length];
    reader
        .read_exact(&mut body)
        .map_err(|_| "unexpected EOF in body".to_string())?;
    serde_json::from_slice(&body).map_err(|e| format!("malformed JSON: {e}"))
}

/// The exact `$​/cancelRequest` payload emitted on every timeout.
/// Unit-tested here; wire emission on the timeout branch is reviewed, since
/// a hung server cannot acknowledge receipt by construction.
pub fn cancel_payload(id: i64) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0", "method": "$/cancelRequest",
        "params": {"id": id}
    })
}

enum Incoming {
    Response { id: String, body: Value },
    Notify { method: String, params: Value },
    Broken(String),
}

/// Convert an LSP (line, character) pair to a byte offset within one line
/// under the negotiated encoding. Never rounds, never clamps.
pub fn lsp_pos_to_bytes(line: &str, character: usize, encoding: &str) -> Result<usize, String> {
    match encoding {
        "utf-8" => {
            if character > line.len() {
                return Err("character past line end".to_string());
            }
            if !line.is_char_boundary(character) {
                return Err("character lands mid-char".to_string());
            }
            Ok(character)
        }
        "utf-16" => {
            let mut units = 0usize;
            for (i, ch) in line.char_indices() {
                if units == character {
                    return Ok(i);
                }
                units += ch.len_utf16();
                if units > character {
                    return Err("character lands mid-surrogate-pair".to_string());
                }
            }
            if units == character {
                return Ok(line.len());
            }
            Err("character past line end".to_string())
        }
        "utf-32" => {
            let mut count = 0usize;
            for (i, _) in line.char_indices() {
                if count == character {
                    return Ok(i);
                }
                count += 1;
            }
            if count == character {
                return Ok(line.len());
            }
            Err("character past line end".to_string())
        }
        other => Err(format!("unsupported position encoding: '{other}'")),
    }
}

fn id_key(id: &Value) -> String {
    match id {
        Value::Number(n) => n.to_string(),
        Value::String(s) => format!("\"{s}\""),
        _ => String::new(),
    }
}

/// Parse a `file://` URI into a repo-contained path. Anything else —
/// `untitled:`, traversal, missing file — is rejected with a reason.
pub fn uri_to_path(root: &Path, uri: &str) -> Result<PathBuf, String> {
    let path_part = uri
        .strip_prefix("file://")
        .ok_or_else(|| format!("non-file URI: '{uri}'"))?;
    let bytes = path_part.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err("bad percent-encoding in URI".to_string());
            }
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3])
                .map_err(|_| "bad percent-encoding in URI".to_string())?;
            out.push(
                u8::from_str_radix(hex, 16)
                    .map_err(|_| "bad percent-encoding in URI".to_string())?,
            );
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    let decoded = String::from_utf8(out).map_err(|_| "non-UTF8 URI".to_string())?;
    let candidate = PathBuf::from(decoded);
    let canonical_root = root
        .canonicalize()
        .map_err(|e| format!("repository root unavailable: {e}"))?;
    if !crate::paths::contains(&canonical_root, &candidate) {
        return Err(format!("URI escapes repository: '{uri}'"));
    }
    Ok(candidate)
}

fn file_uri(path: &Path) -> String {
    format!("file://{}", path.to_string_lossy().replace('\\', "/"))
}

/// A live, opted-in language server subprocess. One reader thread pumps
/// stdout into a mailbox; one thread drains stderr off-stream. Every wait
/// is bounded by the spec timeout; every failure kills and reaps the child.
#[derive(Debug)]
pub struct Session {
    stdin: std::sync::Arc<std::sync::Mutex<std::process::ChildStdin>>,
    inbox: std::sync::mpsc::Receiver<Incoming>,
    child: Option<std::process::Child>,
    stderr_log: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    next_id: i64,
    sent: usize,
    max_messages: usize,
    timeout: Duration,
    max_msg_bytes: usize,
    root: PathBuf,
    canonical_root: PathBuf,
    opened: std::collections::HashSet<String>,
    refs_supported: bool,
    server_version: String,
    pub position_encoding: String,
    pub diagnostics: Vec<String>,
    dead: Option<String>,
}

/// Spawn the server and complete `initialize`/`initialized`. Only an
/// absolute, existing executable is launched, via argv, never a shell.
pub fn spawn(spec: ServerSpec) -> Result<Session, String> {
    if !Path::new(&spec.exe).is_file() {
        return Err(format!("language server not found: '{}'", spec.exe));
    }
    let canonical_root = spec
        .root
        .canonicalize()
        .map_err(|e| format!("repository root unavailable: {e}"))?;
    let mut command = std::process::Command::new(&spec.exe);
    command
        .args(&spec.args)
        .envs(spec.env.iter().map(|(k, v)| (k, v)))
        .current_dir(&spec.root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|e| format!("cannot spawn language server: {e}"))?;
    let stdin = child.stdin.take().ok_or("no stdin")?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let mut stderr = child.stderr.take().ok_or("no stderr")?;
    let stdin = std::sync::Arc::new(std::sync::Mutex::new(stdin));
    let stderr_log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let (tx, inbox) = std::sync::mpsc::channel::<Incoming>();
    // Stderr drain: bounded tail, never on the protocol stream, never a
    // pipe-fill deadlock even under megabyte floods.
    {
        let stderr_log = stderr_log.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match stderr.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let mut log = stderr_log.lock().unwrap();
                        log.extend_from_slice(&buf[..n]);
                        const CAP: usize = 262_144;
                        if log.len() > CAP {
                            let drop = log.len() - CAP;
                            log.drain(..drop);
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    }
    // Stdout pump: exactly one framed message per iteration; any framing
    // or JSON error terminates the session loudly (never skip/resync).
    {
        let max_msg_bytes = spec.max_msg_bytes;
        std::thread::spawn(move || {
            let mut stdout = stdout;
            loop {
                match read_message(&mut stdout, max_msg_bytes) {
                    Ok(body) => {
                        let item = classify(&body);
                        if tx.send(item).is_err() {
                            break;
                        }
                        if matches_body_end(&body) {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Incoming::Broken(e));
                        break;
                    }
                }
            }
        });
    }
    let mut session = Session {
        stdin,
        inbox,
        child: Some(child),
        stderr_log,
        next_id: 1,
        sent: 0,
        max_messages: spec.max_messages,
        timeout: spec.timeout,
        max_msg_bytes: spec.max_msg_bytes,
        root: spec.root.clone(),
        canonical_root,
        opened: std::collections::HashSet::new(),
        refs_supported: false,
        server_version: String::new(),
        position_encoding: "utf-16".to_string(),
        diagnostics: Vec::new(),
        dead: None,
    };
    session.initialize(&spec)?;
    Ok(session)
}

/// Route one parsed body: response by id, server request, or notification.
fn classify(body: &Value) -> Incoming {
    if let Some(id) = body.get("id") {
        if body.get("method").is_some() {
            return Incoming::Notify {
                method: format!("request:{}", body["method"].as_str().unwrap_or("?")),
                params: body.clone(),
            };
        }
        return Incoming::Response {
            id: id_key(id),
            body: body.clone(),
        };
    }
    Incoming::Notify {
        method: body
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        params: body.get("params").cloned().unwrap_or(Value::Null),
    }
}

fn matches_body_end(_body: &Value) -> bool {
    false
}

impl Session {
    fn fail(&mut self, reason: String) -> String {
        self.kill_reap();
        self.dead = Some(reason.clone());
        reason
    }

    /// Best-effort kill + bounded reap. Never blocks forever: a hung child
    /// is left for `Drop` after a bounded wait, never awaited inline.
    fn kill_reap(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let start = std::time::Instant::now();
            while start.elapsed() < Duration::from_secs(5) {
                match child.try_wait() {
                    Ok(Some(_)) => return,
                    Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                    Err(_) => return,
                }
            }
            // Still alive: park it back so Drop retries; the session is
            // dead regardless and no caller blocks on it.
            self.child = Some(child);
        }
    }

    fn send_value(&mut self, body: &Value) -> Result<(), String> {
        if self.sent >= self.max_messages {
            return Err("message budget exceeded".to_string());
        }
        if let Some(dead) = &self.dead {
            return Err(format!("session dead: {dead}"));
        }
        let bytes = encode_message(body)?;
        let write_result = self
            .stdin
            .lock()
            .unwrap()
            .write_all(&bytes)
            .map_err(|e| format!("stdin write failed: {e}"));
        if let Err(e) = write_result {
            return Err(self.fail(e));
        }
        self.sent += 1;
        Ok(())
    }

    /// Send a request and wait for the matching id. Server→client requests
    /// get a protocol-correct MethodNotFound; known notifications are
    /// consumed into diagnostics; unknown `$/` notifications are recorded
    /// explicitly (never silently dropped, never fatal).
    fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        self.send_value(&serde_json::json!({
            "jsonrpc": "2.0", "id": id, "method": method, "params": params
        }))?;
        let want = id.to_string();
        let deadline = self.timeout;
        loop {
            match self.inbox.recv_timeout(deadline) {
                Ok(Incoming::Response { id: got, body }) => {
                    if got == want {
                        if let Some(err) = body.get("error") {
                            return Err(format!("server error: {err}"));
                        }
                        return Ok(body.get("result").cloned().unwrap_or(Value::Null));
                    }
                    self.diagnostics
                        .push(format!("stray response id {got} during {method}"));
                }
                Ok(Incoming::Notify {
                    method: m,
                    params: p,
                }) => {
                    if let Some(req) = m.strip_prefix("request:") {
                        let _ = req;
                        let rid = p.get("id").cloned().unwrap_or(Value::Null);
                        let _ = self.send_value(&serde_json::json!({
                            "jsonrpc": "2.0", "id": rid,
                            "error": {"code": -32601, "message": "not implemented"}
                        }));
                    } else {
                        self.note_notification(&m, &p);
                    }
                }
                Ok(Incoming::Broken(e)) => {
                    return Err(self.fail(format!("server broken: {e}{}", self.stderr_tail())));
                }
                Err(_) => {
                    // Timeout: cancel, then kill + reap, then report.
                    let _ = self.send_value(&cancel_payload(id));
                    return Err(self.fail(format!("request '{method}' timed out")));
                }
            }
        }
    }

    fn stderr_tail(&self) -> String {
        let log = self.stderr_log.lock().unwrap();
        if log.is_empty() {
            return String::new();
        }
        let tail = &log[log.len().saturating_sub(512)..];
        format!("; stderr tail: {}", String::from_utf8_lossy(tail))
    }

    fn note_notification(&mut self, method: &str, params: &Value) {
        match method {
            "$/progress" | "window/logMessage" | "window/showMessage" | "telemetry/event" => {
                self.diagnostics.push(format!("{method}: {params}"));
            }
            _ if method.starts_with("$/") => {
                self.diagnostics
                    .push(format!("unknown-notification: {method}"));
            }
            _ => {
                self.diagnostics.push(format!("notification: {method}"));
            }
        }
        if self.diagnostics.len() > 256 {
            self.diagnostics.remove(0);
        }
    }

    fn initialize(&mut self, spec: &ServerSpec) -> Result<(), String> {
        let result = self.request(
            "initialize",
            serde_json::json!({
                "processId": std::process::id(),
                "rootUri": file_uri(&spec.root),
                "capabilities": {},
                "workspaceFolders": [{"uri": file_uri(&spec.root), "name": "root"}],
            }),
        )?;
        let caps = result
            .get("capabilities")
            .ok_or_else(|| self.fail("initialize: no capabilities".to_string()))?;
        // Advertised-capability honesty: only what the server declares.
        self.refs_supported = caps
            .get("referencesProvider")
            .is_some_and(|v| v.as_bool().unwrap_or(true) && !v.is_null());
        if let Some(enc) = result
            .pointer("/capabilities/positionEncodingKind")
            .and_then(Value::as_str)
        {
            self.position_encoding = enc.to_string();
        }
        if let Some(info) = result.get("serverInfo") {
            self.server_version = info
                .get("version")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
        }
        self.send_value(
            &serde_json::json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}),
        )?;
        Ok(())
    }

    fn ensure_open(&mut self, rel: &str) -> Result<(PathBuf, String), String> {
        if let Some(dead) = &self.dead {
            return Err(format!("session dead: {dead}"));
        }
        let validated = semantic::validate_doc_path(&self.root, rel)?;
        let mut disk = self.root.clone();
        for part in validated.split('/') {
            disk.push(part);
        }
        let text =
            std::fs::read_to_string(&disk).map_err(|e| format!("cannot read '{rel}': {e}"))?;
        if self.opened.insert(validated.clone()) {
            self.send_value(&serde_json::json!({
                "jsonrpc": "2.0", "method": "textDocument/didOpen",
                "params": {"textDocument": {
                    "uri": file_uri(&disk), "languageId": "plaintext",
                    "version": 1, "text": text
                }}
            }))?;
        }
        Ok((disk, text))
    }

    fn locations_to_evidence(
        &self,
        value: &Value,
        definition: bool,
    ) -> Result<Vec<Evidence>, String> {
        let items: Vec<&Value> = match value {
            Value::Null => Vec::new(),
            Value::Array(items) => items.iter().collect(),
            obj if obj.is_object() => vec![obj],
            _ => return Err("bad location payload".to_string()),
        };
        let mut out = Vec::new();
        for item in items {
            // LocationLink uses targetUri/targetRange; plain Location uses uri/range.
            let uri = item
                .get("targetUri")
                .or_else(|| item.get("uri"))
                .and_then(Value::as_str)
                .ok_or_else(|| "location without URI".to_string())?;
            let range = item
                .get("targetRange")
                .or_else(|| item.get("range"))
                .ok_or_else(|| "location without range".to_string())?;
            let disk = uri_to_path(&self.root, uri)?;
            let text =
                std::fs::read_to_string(&disk).map_err(|e| format!("cannot read target: {e}"))?;
            let digest = crate::store::content_digest(text.as_bytes());
            let lines: Vec<&str> = text.split('\n').collect();
            let get = |key: &str| -> Result<(usize, usize), String> {
                let pos = range
                    .get(key)
                    .ok_or_else(|| "range without position".to_string())?;
                Ok((
                    pos.get("line")
                        .and_then(Value::as_u64)
                        .ok_or_else(|| "bad line".to_string())? as usize,
                    pos.get("character")
                        .and_then(Value::as_u64)
                        .ok_or_else(|| "bad character".to_string())? as usize,
                ))
            };
            let (sl, sc) = get("start")?;
            let (el, ec) = get("end")?;
            if sl >= lines.len() || el >= lines.len() {
                return Err("location past end of file".to_string());
            }
            let byte_start = lsp_pos_to_bytes(lines[sl], sc, &self.position_encoding)?;
            let byte_end = lsp_pos_to_bytes(lines[el], ec, &self.position_encoding)?;
            let mut line_starts = Vec::with_capacity(lines.len());
            let mut offset = 0usize;
            for line in &lines {
                line_starts.push(offset);
                offset += line.len() + 1;
            }
            let mut evidence = Evidence::lsp_location(
                disk.canonicalize()
                    .ok()
                    .and_then(|c| {
                        c.strip_prefix(&self.canonical_root)
                            .map(|p| p.to_string_lossy().replace('\\', "/"))
                            .ok()
                    })
                    .unwrap_or_else(|| disk.to_string_lossy().to_string())
                    .as_str(),
                &digest,
                semantic::SpanBytes {
                    start_line: sl as u32,
                    start_col: sc as u32,
                    end_line: el as u32,
                    end_col: ec as u32,
                    byte_start: line_starts[sl] + byte_start,
                    byte_end: line_starts[el] + byte_end,
                },
            );
            evidence.roles = semantic::Roles::from_lsp_kind(definition);
            evidence.provider.version = self.server_version.clone();
            out.push(evidence);
        }
        Ok(out)
    }

    /// Go to definition at a 1-based (line, col). Single hop, no traversal.
    pub fn definition(&mut self, rel: &str, line1: u32, col1: u32) -> Result<LspEvidence, String> {
        let (disk, _) = self.ensure_open(rel)?;
        let result = self.request(
            "textDocument/definition",
            serde_json::json!({"textDocument": {"uri": file_uri(&disk)},
                "position": {"line": line1.saturating_sub(1), "character": col1.saturating_sub(1)}}),
        )?;
        Ok(LspEvidence {
            facts: self.locations_to_evidence(&result, true)?,
            server_version: self.server_version.clone(),
            position_encoding: self.position_encoding.clone(),
            diagnostics: self.diagnostics.clone(),
        })
    }

    /// Find references at a 1-based (line, col). Advertised capability only:
    /// a server without `referencesProvider` yields explicit unsupported.
    pub fn references(&mut self, rel: &str, line1: u32, col1: u32) -> Result<LspEvidence, String> {
        if !self.refs_supported {
            return Err("unsupported: server lacks referencesProvider".to_string());
        }
        let (disk, _) = self.ensure_open(rel)?;
        let result = self.request(
            "textDocument/references",
            serde_json::json!({"textDocument": {"uri": file_uri(&disk)},
                "position": {"line": line1.saturating_sub(1), "character": col1.saturating_sub(1)},
                "context": {"includeDeclaration": true}}),
        )?;
        Ok(LspEvidence {
            facts: self.locations_to_evidence(&result, false)?,
            server_version: self.server_version.clone(),
            position_encoding: self.position_encoding.clone(),
            diagnostics: self.diagnostics.clone(),
        })
    }

    /// `shutdown` request + `exit` notification + bounded reap. The server
    /// gets a short grace period to exit on its own (so `exit` is observed);
    /// only a lingerer is killed.
    pub fn shutdown(mut self) -> Vec<String> {
        if self.dead.is_none() {
            let _ = self.request("shutdown", Value::Null);
            let _ = self.send_value(&serde_json::json!({"jsonrpc": "2.0", "method": "exit"}));
            if let Some(child) = self.child.as_mut() {
                let start = std::time::Instant::now();
                while start.elapsed() < Duration::from_secs(2) {
                    match child.try_wait() {
                        Ok(Some(_)) => break,
                        Ok(None) => std::thread::sleep(Duration::from_millis(25)),
                        Err(_) => break,
                    }
                }
            }
        }
        self.kill_reap();
        // Drain stderr tail into diagnostics for post-mortem.
        let tail = self.stderr_tail();
        if !tail.is_empty() {
            self.diagnostics.push(tail);
        }
        std::mem::take(&mut self.diagnostics)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.kill_reap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framing_round_trips() {
        let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "x"});
        let bytes = encode_message(&body).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.starts_with("Content-Length: "));
        assert!(text.contains("\r\n\r\n"));
        let back = read_message(text.as_bytes(), 1024).unwrap();
        assert_eq!(back, body);
    }

    #[test]
    fn stdout_pollution_is_rejected_not_skipped() {
        let polluted = b"[server] starting up\nContent-Length: 2\r\n\r\n{}";
        assert!(read_message(&polluted[..], 1024).is_err());
    }

    #[test]
    fn malformed_json_and_oversize_are_rejected() {
        let bad = b"Content-Length: 5\r\n\r\n{oops";
        assert!(read_message(&bad[..], 1024).is_err());
        let big = b"Content-Length: 2000\r\n\r\n{}";
        assert!(read_message(&big[..], 1024).is_err());
    }

    #[test]
    fn truncated_stream_errors() {
        let cut = b"Content-Length: 100\r\n\r\n{\"a\":";
        assert!(read_message(&cut[..], 1024).is_err());
        assert!(read_message(&b""[..], 1024).is_err());
    }

    #[test]
    fn relative_server_path_is_rejected() {
        assert!(ServerSpec::new("rust-analyzer", Path::new("C:\\r")).is_err());
    }

    #[test]
    fn utf16_surrogate_span_math_is_exact() {
        // "🚀 x": rocket = 2 UTF-16 units; x at units 3..4, bytes 5..6.
        let line = "\u{1F680} x";
        assert_eq!(lsp_pos_to_bytes(line, 3, "utf-16").unwrap(), 5);
        assert_eq!(lsp_pos_to_bytes(line, 4, "utf-16").unwrap(), 6);
        assert!(lsp_pos_to_bytes(line, 1, "utf-16").is_err()); // mid-pair
        assert_eq!(lsp_pos_to_bytes(line, 2, "utf-32").unwrap(), 5);
        assert_eq!(lsp_pos_to_bytes(line, 5, "utf-8").unwrap(), 5);
        assert!(lsp_pos_to_bytes(line, 0, "utf-7").is_err());
    }

    #[test]
    fn cancel_payload_shape_is_exact() {
        assert_eq!(
            cancel_payload(7),
            serde_json::json!({
                "jsonrpc": "2.0", "method": "$/cancelRequest",
                "params": {"id": 7}
            })
        );
    }

    #[test]
    fn uri_validation_rejects_escape_and_non_file() {
        let dir = crate::testutil::tmpdir("lsp-uri");
        std::fs::write(dir.join("main.rs"), "fn main() {}\n").unwrap();
        let slash = dir.to_string_lossy().replace('\\', "/");
        assert!(uri_to_path(&dir, &format!("file://{slash}/main.rs")).is_ok());
        assert!(uri_to_path(&dir, "untitled:Untitled-1").is_err());
        assert!(uri_to_path(&dir, &format!("file://{slash}/../evil.rs")).is_err());
        assert!(uri_to_path(&dir, &format!("file://{slash}/nope.rs")).is_err());
    }

    /// Locate the mock server built from `examples/mock_lsp.rs`, building it
    /// once per test process so the binary can never go stale. A missing
    /// binary fails loudly, never skips.
    fn mock_bin() -> std::path::PathBuf {
        static BUILD: std::sync::Once = std::sync::Once::new();
        BUILD.call_once(|| {
            let status = std::process::Command::new("cargo")
                .args(["build", "--offline", "--example", "mock_lsp"])
                .status()
                .expect("cargo build mock server");
            assert!(status.success(), "mock server must build");
        });
        let mut p = std::env::current_exe().unwrap();
        p.pop(); // test binary
        p.pop(); // deps
        p.push("examples");
        p.push(format!("mock_lsp{}", std::env::consts::EXE_SUFFIX));
        assert!(p.is_file(), "mock server missing: {}", p.display());
        p
    }

    const MOCK_MAIN: &str = "fn main() {\n    let x = 1;\n    println!(\"{x}\");\n}\nfn helper() {}\nfn a() {}\nfn b() {}\nfn c() {}\nfn d() {}\n";

    fn mock_spec(mode: &str, root: &std::path::Path, log: &std::path::Path) -> ServerSpec {
        let exe = mock_bin().to_string_lossy().to_string();
        let mut spec = ServerSpec::new(&exe, root).unwrap();
        spec.env = vec![
            ("RILTRA_MOCK_LSP".to_string(), mode.to_string()),
            (
                "RILTRA_MOCK_ROOT".to_string(),
                root.to_string_lossy().replace('\\', "/"),
            ),
            (
                "RILTRA_MOCK_LOG".to_string(),
                log.to_string_lossy().to_string(),
            ),
        ];
        spec.timeout = std::time::Duration::from_secs(10);
        spec
    }

    fn mock_repo() -> (std::path::PathBuf, std::path::PathBuf) {
        let dir = crate::testutil::tmpdir("lsp-mock");
        std::fs::write(dir.join("main.rs"), MOCK_MAIN).unwrap();
        let log = dir.join("mock.log");
        (dir, log)
    }

    #[test]
    fn mock_success_definition_and_references() {
        let (dir, log) = mock_repo();
        let mut session = spawn(mock_spec("success", &dir, &log)).unwrap();
        assert_eq!(session.position_encoding, "utf-16");
        let def = session.definition("main.rs", 2, 5).unwrap();
        assert_eq!(def.facts.len(), 1);
        assert_eq!(def.facts[0].location.path, "main.rs");
        assert!(def.facts[0].symbol.is_none());
        assert_eq!(
            def.facts[0].verification,
            crate::semantic::Verification::Unverified
        );
        let refs = session.references("main.rs", 2, 5).unwrap();
        assert_eq!(refs.facts.len(), 2);
        session.shutdown();
        let logged = std::fs::read_to_string(&log).unwrap();
        assert!(logged.contains("shutdown"), "shutdown logged:\n{logged}");
        assert!(logged.contains("exit"), "exit logged:\n{logged}");
    }

    #[test]
    fn mock_hang_times_out_kills_and_reaps() {
        let (dir, log) = mock_repo();
        let mut spec = mock_spec("hang", &dir, &log);
        spec.timeout = std::time::Duration::from_millis(700);
        let mut session = spawn(spec).unwrap();
        let start = std::time::Instant::now();
        let err = session.definition("main.rs", 1, 1).unwrap_err();
        assert!(err.contains("timed out"), "unexpected: {err}");
        assert!(
            start.elapsed() < std::time::Duration::from_secs(15),
            "reap must stay bounded"
        );
        assert!(session.definition("main.rs", 1, 1).is_err());
    }

    #[test]
    fn mock_crash_reports_death() {
        let (dir, log) = mock_repo();
        let err = spawn(mock_spec("crash", &dir, &log)).unwrap_err();
        assert!(err.contains("broken"), "unexpected: {err}");
    }

    #[test]
    fn mock_malformed_response_is_rejected() {
        let (dir, log) = mock_repo();
        let err = spawn(mock_spec("malformed", &dir, &log)).unwrap_err();
        assert!(err.contains("broken"), "unexpected: {err}");
    }

    #[test]
    fn mock_stdout_pollution_is_rejected() {
        let (dir, log) = mock_repo();
        let err = spawn(mock_spec("pollution", &dir, &log)).unwrap_err();
        assert!(err.contains("broken"), "unexpected: {err}");
    }

    #[test]
    fn mock_stderr_flood_never_deadlocks() {
        let (dir, log) = mock_repo();
        let mut session = spawn(mock_spec("stderr-flood", &dir, &log)).unwrap();
        let def = session.definition("main.rs", 2, 5).unwrap();
        assert_eq!(def.facts.len(), 1);
        let diags = session.shutdown();
        assert!(
            diags.iter().any(|d| d.contains("stderr tail")),
            "stderr captured: {diags:?}"
        );
    }

    #[test]
    fn mock_progress_consumed_and_unknown_recorded() {
        let (dir, log) = mock_repo();
        let mut session = spawn(mock_spec("progress", &dir, &log)).unwrap();
        let def = session.definition("main.rs", 2, 5).unwrap();
        assert_eq!(def.facts.len(), 1);
        // Both notifications precede the response on the pipe: deterministic.
        assert!(
            def.diagnostics.iter().any(|d| d.starts_with("$/progress")),
            "progress consumed: {:?}",
            def.diagnostics
        );
        assert!(
            def.diagnostics
                .iter()
                .any(|d| d.contains("unknown-notification: $/weird")),
            "unknown recorded: {:?}",
            def.diagnostics
        );
        session.shutdown();
    }

    #[test]
    fn mock_server_without_refs_capability_is_unsupported() {
        let (dir, log) = mock_repo();
        let mut session = spawn(mock_spec("no-refs", &dir, &log)).unwrap();
        assert!(session.definition("main.rs", 2, 5).is_ok());
        let err = session.references("main.rs", 2, 5).unwrap_err();
        assert!(err.contains("unsupported"), "unexpected: {err}");
        session.shutdown();
    }
}
