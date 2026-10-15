// SPDX-License-Identifier: Apache-2.0
//! M5 LSP subprocess adapter: explicit opt-in, std-only JSON-RPC over
//! Content-Length framing. No shell, no auto-install, no async runtime.
//! The child is untrusted (servers may run hooks/builds/network): absolute
//! executable path only, bounded messages/timeouts, stderr kept off the
//! protocol stream, kill + reap on every failure path.

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
    /// Negotiated `positionEncodingKind` ("utf-16" unless the server picks
    /// otherwise). Read by Tier B validation; queries already carry bytes.
    #[allow(dead_code)]
    pub position_encoding: String,
    pub diagnostics: Vec<String>,
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

/// LSP error code servers use for "state changed under you, retry".
/// rust-analyzer answers it while a freshly opened file is still loading:
/// retryable like an empty answer, never evidence, never silently fatal.
const RETRYABLE_CODE: &str = "\"code\":-32801";

/// True when a request error only reports unreadiness (empty answer or a
/// retryable code) rather than a real failure.
fn is_unready(result: &Result<Value, String>) -> bool {
    match result {
        Ok(v) => v.is_null() || v.as_array().is_some_and(Vec::is_empty),
        Err(e) => e.contains(RETRYABLE_CODE),
    }
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
    if !uri.starts_with("file://") {
        return Err(format!("non-file URI: '{uri}'"));
    }
    let candidate = semantic::uri_path(uri)?;
    let canonical_root = root
        .canonicalize()
        .map_err(|e| format!("repository root unavailable: {e}"))?;
    if !crate::paths::contains(&canonical_root, &candidate) {
        return Err(format!("URI escapes repository: '{uri}'"));
    }
    Ok(candidate)
}

/// Build a `file://` URI for a local path. Strips Windows extended-length
/// (`\\?\`) and UNC (`\\?\UNC\`) prefixes first: servers reject URIs built
/// from them (proven by rust-analyzer Tier B: "url is not a file").
fn file_uri(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let text = text
        .strip_prefix("//?/UNC/")
        .map(|rest| format!("//{rest}"))
        .unwrap_or(text);
    let text = text.strip_prefix("//?/").unwrap_or(&text).to_string();
    if text.starts_with("//") {
        format!("file:{text}")
    } else {
        format!(
            "file:///{text}",
            text = text.strip_prefix('/').unwrap_or(&text)
        )
    }
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
    root: PathBuf,
    canonical_root: PathBuf,
    opened: std::collections::HashSet<String>,
    refs_supported: bool,
    diagnostic_provider: bool,
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
                        if tx.send(classify(&body)).is_err() {
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
        root: spec.root.clone(),
        canonical_root,
        opened: std::collections::HashSet::new(),
        refs_supported: false,
        diagnostic_provider: false,
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
                        if std::env::var("RILTRA_LSP_DEBUG").is_ok() {
                            eprintln!("LSP <- {body}");
                        }
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
        // rust-analyzer reports its encoding as `positionEncoding`
        // (singular); accept that alias too, never assume.
        self.refs_supported = caps
            .get("referencesProvider")
            .is_some_and(|v| v.as_bool().unwrap_or(true) && !v.is_null());
        self.diagnostic_provider = caps.get("diagnosticProvider").is_some();
        if let Some(enc) = result
            .pointer("/capabilities/positionEncodingKind")
            .or_else(|| result.pointer("/capabilities/positionEncoding"))
            .and_then(Value::as_str)
        {
            self.position_encoding = enc.to_string();
        }
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
            // Real languageId: servers index "plaintext" as nothing (proven
            // by an empty rust-analyzer answer in Tier B).
            let language_id = match crate::syntax::classify(&disk) {
                crate::syntax::FileKind::Supported(lang) => match lang {
                    crate::syntax::Language::Tsx => "typescriptreact",
                    other => crate::syntax::language_name(other),
                },
                _ => "plaintext",
            };
            self.send_value(&serde_json::json!({
                "jsonrpc": "2.0", "method": "textDocument/didOpen",
                "params": {"textDocument": {
                    "uri": file_uri(&disk), "languageId": language_id,
                    "version": 1, "text": text
                }}
            }))?;
            self.await_analysis(&validated)?;
        }
        Ok((disk, text))
    }

    /// Warmup: when the server advertises push diagnostics, wait (bounded)
    /// for the first `publishDiagnostics` of the opened file before any
    /// position query. Pull-model servers never push: callers additionally
    /// retry empty answers (see `definition`). Servers without the
    /// capability skip the wait entirely.
    fn await_analysis(&mut self, rel: &str) -> Result<(), String> {
        if !self.diagnostic_provider {
            return Ok(());
        }
        let deadline = std::time::Instant::now() + self.timeout.min(Duration::from_secs(15));
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            if left.is_zero() {
                self.diagnostics
                    .push("analysis wait timed out; answers may be empty".to_string());
                return Ok(());
            }
            match self.inbox.recv_timeout(left.min(Duration::from_secs(2))) {
                Ok(Incoming::Notify { method, params }) => {
                    if method == "textDocument/publishDiagnostics" {
                        self.note_notification(&method, &params);
                        let uri = params.get("uri").and_then(Value::as_str).unwrap_or("");
                        if let Ok(path) = uri_to_path(&self.root, uri) {
                            let mut check = self.root.clone();
                            for part in rel.split('/') {
                                check.push(part);
                            }
                            if path == check {
                                return Ok(());
                            }
                        }
                    } else {
                        self.note_notification(&method, &params);
                    }
                }
                Ok(Incoming::Response { id, .. }) => {
                    self.diagnostics
                        .push(format!("stray response id {id} during warmup"));
                }
                Ok(Incoming::Broken(e)) => {
                    return Err(self.fail(format!("server broken: {e}{}", self.stderr_tail())));
                }
                Err(_) => continue,
            }
        }
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
    /// Cold servers answer empty while loading: empty answers are retried
    /// on a bounded schedule (immediate + 2s x5); a still-empty answer is
    /// reported as-is with the retry count, never upgraded to evidence.
    pub fn definition(&mut self, rel: &str, line1: u32, col1: u32) -> Result<LspEvidence, String> {
        let (disk, _) = self.ensure_open(rel)?;
        let mut facts = Vec::new();
        let mut tries = 0;
        for attempt in 0..6 {
            tries = attempt;
            let result = self.request(
                "textDocument/definition",
                serde_json::json!({"textDocument": {"uri": file_uri(&disk)},
                    "position": {"line": line1.saturating_sub(1), "character": col1.saturating_sub(1)}}),
            );
            if !is_unready(&result) || attempt == 5 {
                facts = self.locations_to_evidence(&result?, true)?;
                break;
            }
            std::thread::sleep(Duration::from_secs(2));
        }
        if tries > 0 && !facts.is_empty() {
            self.diagnostics
                .push(format!("definition converged after {tries} retries"));
        }
        Ok(LspEvidence {
            facts,
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
        let mut facts = Vec::new();
        for attempt in 0..6 {
            let result = self.request(
                "textDocument/references",
                serde_json::json!({"textDocument": {"uri": file_uri(&disk)},
                    "position": {"line": line1.saturating_sub(1), "character": col1.saturating_sub(1)},
                    "context": {"includeDeclaration": true}}),
            );
            if !is_unready(&result) || attempt == 5 {
                facts = self.locations_to_evidence(&result?, false)?;
                break;
            }
            std::thread::sleep(Duration::from_secs(2));
        }
        Ok(LspEvidence {
            facts,
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
        // Lowercase drive letter (rust-analyzer normalization): same file.
        let lower = slash
            .chars()
            .enumerate()
            .map(|(i, c)| if i == 0 { c.to_ascii_lowercase() } else { c })
            .collect::<String>();
        assert!(uri_to_path(&dir, &format!("file:///{lower}/main.rs")).is_ok());
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
