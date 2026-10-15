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

fn main() {
    let mode = std::env::var("RILTRA_MOCK_LSP").unwrap_or_default();
    let stdin = std::io::stdin();
    let mut lock = stdin.lock();
    let mut initialized = false;
    let mut hang = false;
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
        if body.contains("\"method\":\"$/cancelRequest\"") {
            log_event("cancel");
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
                    println!("[mock] a helpful log line on stdout");
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
                let _ = lock.flush();
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
