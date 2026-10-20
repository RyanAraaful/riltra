// SPDX-License-Identifier: Apache-2.0
//! Test-only mock language server for M5 LSP adapter tests. Speaks minimal
//! Content-Length JSON-RPC with hand-rolled std-only framing (deliberately
//! independent of `lsp.rs` so framing bugs cannot hide in both).
//!
//! Mode from `RILTRA_MOCK_LSP`; event log (cancel/shutdown/exit receipts)
//! appended to `RILTRA_MOCK_LOG`; served URIs built from `RILTRA_MOCK_ROOT`.

use std::io::{Read, Write};

fn read_message(stdin: &mut impl Read) -> Option<String> {
    let mut header = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        if stdin.read_exact(&mut byte).is_err() {
            return None;
        }
        header.push(byte[0]);
        if header.len() > 8192 {
            return None;
        }
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let text = String::from_utf8(header).ok()?;
    let mut length = None;
    for line in text.split("\r\n") {
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                length = value.trim().parse::<usize>().ok();
            }
        }
    }
    let mut body = vec![0u8; length?];
    stdin.read_exact(&mut body).ok()?;
    String::from_utf8(body).ok()
}

fn send(body: &str) {
    let out = std::io::stdout();
    let mut lock = out.lock();
    let _ = write!(lock, "Content-Length: {}\r\n\r\n{body}", body.len());
    let _ = lock.flush();
}

fn respond(id: &str, result: &str) {
    send(&format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{result}}}"
    ));
}

fn notify(method: &str, params: &str) {
    send(&format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"{method}\",\"params\":{params}}}"
    ));
}

fn log_event(msg: &str) {
    if let Ok(path) = std::env::var("RILTRA_MOCK_LOG") {
        use std::fmt::Write as _;
        let mut text = String::new();
        if let Ok(prev) = std::fs::read_to_string(&path) {
            text.push_str(&prev);
        }
        let _ = writeln!(text, "{msg}");
        let _ = std::fs::write(&path, text);
    }
}

fn file_uri(rel: &str) -> String {
    let root = std::env::var("RILTRA_MOCK_ROOT").unwrap_or_default();
    format!("file://{root}/{rel}")
}

fn id_of(body: &str) -> String {
    // Minimal id extraction: "id":<num> or "id":"s".
    if let Some(rest) = body.split("\"id\":").nth(1) {
        let rest = rest.trim_start();
        if let Some(quoted) = rest.strip_prefix('"') {
            let end = quoted.find('"').unwrap_or(0);
            return format!("\"{}\"", &quoted[..end]);
        }
        let end = rest
            .find(|c: char| !(c.is_ascii_digit() || c == '-'))
            .unwrap_or(rest.len());
        return rest[..end].to_string();
    }
    "0".to_string()
}

/// The URI of the document the client opened, captured from didOpen.
fn opened_uri(body: &str) -> String {
    if let Some(at) = body.find("\"uri\":\"") {
        let rest = &body[at + 7..];
        if let Some(end) = rest.find('"') {
            return rest[..end].to_string();
        }
    }
    String::new()
}

/// A `file://` URI as a local path, for the mock's own file writes. The
/// Windows form the adapter sends is `file:///C:/...`: the third slash
/// would make the drive letter a path component.
fn uri_to_path(uri: &str) -> String {
    let mut path = uri.strip_prefix("file://").unwrap_or(uri).to_string();
    if path.len() > 2 && path.as_bytes()[0] == b'/' && path.as_bytes()[2] == b':' {
        path.remove(0);
    }
    path
}

/// A file OUTSIDE the repository, really created: the adapter has to reject
/// its diagnostics by containment, not by "the file does not exist". The
/// name carries the mock's own process id, so parallel tests never share
/// it.
fn outside_file() -> String {
    let root = std::env::var("RILTRA_MOCK_ROOT").unwrap_or_default();
    let dir = format!("{root}/../outside-{}", std::process::id());
    let _ = std::fs::create_dir_all(&dir);
    let path = format!("{dir}/main.rs");
    let _ = std::fs::write(&path, "fn outside() {}\n");
    format!("file://{path}")
}

/// Publish diagnostics for the opened document: one ERROR (the shape a
/// missing compilation database produces), one WARNING on another line, one
/// diagnostic with a very long message that carries an absolute path, and a
/// batch of extra diagnostics. A second notification names a file OUTSIDE the
/// repository, so the adapter's path confinement has something to reject.
fn publish_diagnostics(uri: &str) {
    publish_diagnostics_for(uri, &outside_file());
}

/// The same, with the outside file already prepared: the caller can create
/// it BEFORE it answers, so the notification follows the response at once.
fn publish_diagnostics_for(uri: &str, outside: &str) {
    let root = std::env::var("RILTRA_MOCK_ROOT").unwrap_or_default();
    let long = format!(
        "a very long diagnostic message that repeats itself {} and names the absolute path {root}/include/geom/point.h so that a sanitizer has something to shorten",
        "again ".repeat(40)
    );
    let mut batch = String::new();
    for i in 0..20 {
        if i > 0 {
            batch.push(',');
        }
        batch.push_str(&format!(
            "{{\"range\":{{\"start\":{{\"line\":{i},\"character\":0}},\"end\":{{\"line\":{i},\"character\":1}}}},\"severity\":2,\"code\":\"filler-{i}\",\"message\":\"filler {i}\"}}"
        ));
    }
    notify(
        "textDocument/publishDiagnostics",
        &format!(
            "{{\"uri\":\"{uri}\",\"diagnostics\":[{{\"range\":{{\"start\":{{\"line\":3,\"character\":9}},\"end\":{{\"line\":3,\"character\":23}}}},\"severity\":1,\"code\":\"pp_file_not_found\",\"source\":\"clang\",\"message\":\"'geom/point.h' file not found in {root}/include\"}},{{\"range\":{{\"start\":{{\"line\":1,\"character\":4}},\"end\":{{\"line\":1,\"character\":9}}}},\"severity\":2,\"code\":\"unused-variable\",\"source\":\"clang\",\"message\":\"unused variable 'x'\"}},{{\"range\":{{\"start\":{{\"line\":0,\"character\":0}},\"end\":{{\"line\":0,\"character\":1}}}},\"severity\":1,\"code\":\"long-message\",\"message\":\"{long}\"}}]}}"
        ),
    );
    notify(
        "textDocument/publishDiagnostics",
        &format!(
            "{{\"uri\":\"{outside}\",\"diagnostics\":[{{\"range\":{{\"start\":{{\"line\":0,\"character\":0}},\"end\":{{\"line\":0,\"character\":1}}}},\"severity\":1,\"code\":\"outside-the-repository\",\"message\":\"this file is outside the repository\"}}]}}"
        ),
    );
    notify(
        "textDocument/publishDiagnostics",
        &format!("{{\"uri\":\"{uri}\",\"diagnostics\":[{batch}]}}"),
    );
}

/// A server→client request: the client has to answer it, or a server that
/// waits for the answer would stall. The reply is logged, so the test can
/// see that it arrived.
fn ask_the_client(id: &str) {
    send(&format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":\"{id}\",\"method\":\"workspace/configuration\",\"params\":{{\"items\":[]}}}}"
    ));
}

/// Append a line to the opened document: the client asked about the file as
/// it was, and the file changes underneath it while the server answers.
fn mutate_opened_file(uri: &str) {
    if uri.is_empty() {
        return;
    }
    let path = uri_to_path(uri);
    if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(&path) {
        let _ = writeln!(file, "// the source changed while the server was answering");
    }
}

fn main() {
    let mode = std::env::var("RILTRA_MOCK_LSP").unwrap_or_default();
    let stdin = std::io::stdin();
    let mut lock = stdin.lock();
    let mut initialized = false;
    let mut hang = false;
    let mut opened = String::new();
    while let Some(body) = read_message(&mut lock) {
        if body.contains("\"method\":\"initialized\"") {
            initialized = true;
            if mode == "progress" {
                notify(
                    "$/progress",
                    "{\"token\":\"t\",\"value\":{\"kind\":\"begin\"}}",
                );
                notify("$/weird", "{}");
            }
            continue;
        }
        if body.contains("\"method\":\"textDocument/didOpen\"") {
            opened = opened_uri(&body);
            if mode == "diagnostics" {
                publish_diagnostics(&opened);
            }
            continue;
        }
        if body.contains("\"method\":\"$/cancelRequest\"") {
            log_event("cancel");
            continue;
        }
        if body.contains("client-request-1") {
            // The client's reply to the server's own request.
            log_event("client-request-answered");
            continue;
        }
        if body.contains("\"method\":\"shutdown\"") {
            log_event("shutdown");
            respond(&id_of(&body), "null");
            continue;
        }
        if body.contains("\"method\":\"exit\"") {
            log_event("exit");
            break;
        }
        if !initialized && !body.contains("\"method\":\"initialize\"") {
            continue;
        }
        if body.contains("\"method\":\"initialize\"") {
            let id = id_of(&body);
            match mode.as_str() {
                "crash" => std::process::exit(1),
                "hang" => {
                    // Answer initialize so the session forms, then hang on
                    // every later request: the harness must time out, cancel,
                    // kill, and reap.
                    respond(
                        &id,
                        "{\"capabilities\":{\"referencesProvider\":true},\"serverInfo\":{\"version\":\"0.0-mock\"}}",
                    );
                    hang = true;
                    continue;
                }
                "malformed" => {
                    let out = std::io::stdout();
                    let mut lock = out.lock();
                    let body = "{oops!";
                    let _ = write!(lock, "Content-Length: {0}\r\n\r\n{body}", body.len());
                    let _ = lock.flush();
                    continue;
                }
                "pollution" => {
                    // A path in the server's stdout must not reach the caller.
                    println!("[mock] reading /opt/build/private/x");
                    respond(&id, "{\"capabilities\":{}}");
                    continue;
                }
                "no-refs" => {
                    respond(
                        &id,
                        "{\"capabilities\":{},\"serverInfo\":{\"version\":\"0.0-mock\"}}",
                    );
                    continue;
                }
                _ => {
                    respond(
                        &id,
                        "{\"capabilities\":{\"referencesProvider\":true,\"positionEncodingKind\":\"utf-16\"},\"serverInfo\":{\"version\":\"0.0-mock\"}}",
                    );
                    continue;
                }
            }
        }
        if hang {
            // Hang after a formed session: sleep past every harness deadline.
            loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
            }
        }
        if body.contains("\"method\":\"textDocument/definition\"") {
            if mode == "stderr-flood" {
                let err = std::io::stderr();
                let mut lock = err.lock();
                for _ in 0..2000 {
                    let _ = lock.write_all(&[b'x'; 1024]);
                }
                // A path in the server's stderr must not reach the caller.
                // It is written last, so it is in the tail the adapter keeps.
                let _ = writeln!(lock, "indexing C:\\Windows\\System32\\private");
                let _ = lock.flush();
            }
            if mode == "stale-input" {
                // The file the client asked about changes while the server is
                // answering: the answer is about a version of the source that
                // is no longer on disk.
                mutate_opened_file(&opened);
            }
            if mode == "late-diagnostics" {
                // The server answers first and publishes afterwards: the
                // adapter must not lose the warning that follows the answer.
                // The file work happens BEFORE the answer, so the
                // notification follows the response immediately.
                let outside = outside_file();
                let uri = file_uri("main.rs");
                respond(
                    &id_of(&body),
                    &format!(
                        "{{\"uri\":\"{uri}\",\"range\":{{\"start\":{{\"line\":1,\"character\":4}},\"end\":{{\"line\":1,\"character\":7}}}}}}"
                    ),
                );
                publish_diagnostics_for(&opened, &outside);
                continue;
            }
            if mode == "empty" {
                // An empty answer with an error diagnostic: the answer stays
                // empty, and the diagnostic says why.
                publish_diagnostics(&opened);
                respond(&id_of(&body), "[]");
                continue;
            }
            if mode == "fail-after-diagnostics" {
                // The server explains itself and then fails: the diagnostics
                // must travel with the failure.
                publish_diagnostics(&opened);
                send(&format!(
                    "{{\"jsonrpc\":\"2.0\",\"id\":{},\"error\":{{\"code\":-32000,\"message\":\"the server refused\"}}}}",
                    id_of(&body)
                ));
                continue;
            }
            if mode == "client-request" {
                // The server asks the client something after it answered: the
                // client has to reply, or the server waits.
                let uri = file_uri("main.rs");
                respond(
                    &id_of(&body),
                    &format!(
                        "{{\"uri\":\"{uri}\",\"range\":{{\"start\":{{\"line\":1,\"character\":4}},\"end\":{{\"line\":1,\"character\":7}}}}}}"
                    ),
                );
                ask_the_client("client-request-1");
                continue;
            }
            if mode == "stray-response" {
                // A response nobody asked for: it must be recorded as a
                // bounded notice.
                let uri = file_uri("main.rs");
                respond(
                    &id_of(&body),
                    &format!(
                        "{{\"uri\":\"{uri}\",\"range\":{{\"start\":{{\"line\":1,\"character\":4}},\"end\":{{\"line\":1,\"character\":7}}}}}}"
                    ),
                );
                send("{\"jsonrpc\":\"2.0\",\"id\":9999,\"result\":{\"path\":\"C:/Windows/System32/private\"}}");
                continue;
            }
            let uri = file_uri("main.rs");
            respond(
                &id_of(&body),
                &format!(
                    "{{\"uri\":\"{uri}\",\"range\":{{\"start\":{{\"line\":1,\"character\":4}},\"end\":{{\"line\":1,\"character\":7}}}}}}"
                ),
            );
            continue;
        }
        if body.contains("\"method\":\"textDocument/references\"") {
            let uri = file_uri("main.rs");
            respond(
                &id_of(&body),
                &format!(
                    "[{{\"uri\":\"{uri}\",\"range\":{{\"start\":{{\"line\":4,\"character\":0}},\"end\":{{\"line\":4,\"character\":3}}}}}},{{\"uri\":\"{uri}\",\"range\":{{\"start\":{{\"line\":7,\"character\":2}},\"end\":{{\"line\":7,\"character\":5}}}}}}]"
                ),
            );
            continue;
        }
        // Unknown method: protocol-correct MethodNotFound.
        if body.contains("\"id\"") {
            send(&format!(
                "{{\"jsonrpc\":\"2.0\",\"id\":{},\"error\":{{\"code\":-32601,\"message\":\"nope\"}}}}",
                id_of(&body)
            ));
        }
    }
}
