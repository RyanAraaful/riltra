// SPDX-License-Identifier: Apache-2.0
//! M8 CP1/CP3: MCP stdio server. CP1 serves initialize; CP3 registers the
//! three read-only tools (design gate §3) through the rmcp tool macros.
//! Tool calls reuse the CLI's response formatters byte-for-byte, and the
//! index read path (`store::open_read`) never creates, migrates, or replaces
//! a database. stdout stays protocol-only; blocking work runs on the tokio
//! blocking pool.

use std::path::Path;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Implementation, ServerCapabilities, ServerConfig};
use rmcp::schemars::JsonSchema;
use rmcp::{serve_server, tool, tool_handler, tool_router, ServerHandler};
use serde::Deserialize;

use crate::{git, ignore, index, paths, registry, response, store};

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
struct RepoStatusInput {
    /// Registered repository name.
    repo: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
struct CodeSearchInput {
    /// Registered repository name.
    repo: String,
    /// Symbol text to search for (the CLI's `search <repo> <symbol>` argument).
    query: String,
    /// Maximum hits to return (CLI default 50, maximum 1000).
    limit: Option<usize>,
    /// Hits to skip before returning results (default 0).
    offset: Option<usize>,
}

struct RiltraMcp;

#[tool_router]
impl RiltraMcp {
    #[tool(
        name = "riltra_repo_list",
        description = "List registered repositories (same document as CLI `repo list`).",
        annotations(
            title = "List registered repositories",
            read_only_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn riltra_repo_list(&self) -> CallToolResult {
        blocking(|| match registry::load(&paths::registry_path()) {
            Ok(reg) => doc(&registry::list_json(&reg)),
            Err(e) => tool_error("store_error", &e),
        })
        .await
    }

    #[tool(
        name = "riltra_repo_status",
        description = "Repository status (same document as CLI `repo status`) plus a read-only `index_status` member (same document as CLI `index status`).",
        annotations(
            title = "Repository status",
            read_only_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn riltra_repo_status(
        &self,
        Parameters(input): Parameters<RepoStatusInput>,
    ) -> CallToolResult {
        blocking(move || repo_status_doc(&input)).await
    }

    #[tool(
        name = "riltra_code_search",
        description = "Search indexed symbols by name (same document as CLI `search`). A missing index reports `no-index` without creating storage.",
        annotations(
            title = "Search indexed symbols",
            read_only_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn riltra_code_search(
        &self,
        Parameters(input): Parameters<CodeSearchInput>,
    ) -> CallToolResult {
        blocking(move || code_search_doc(&input)).await
    }
}

#[tool_handler]
impl ServerHandler for RiltraMcp {
    fn get_info(&self) -> ServerConfig {
        // Override the SDK default (`Implementation::from_build_env` reads
        // rmcp's own crate name) with this binary's identity.
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(crate::NAME, env!("CARGO_PKG_VERSION")))
    }
}

/// Run synchronous domain work on the blocking pool; a panicked task is a
/// classified `store_error`, never a dropped connection.
async fn blocking(f: impl FnOnce() -> CallToolResult + Send + 'static) -> CallToolResult {
    match tokio::task::spawn_blocking(f).await {
        Ok(result) => result,
        Err(e) => tool_error("store_error", &format!("internal task failure: {e}")),
    }
}

/// A success result: the shared formatter's bytes, parsed back to JSON.
fn doc(json_text: &str) -> CallToolResult {
    match serde_json::from_str(json_text) {
        Ok(value) => CallToolResult::structured(value),
        Err(e) => tool_error(
            "store_error",
            &format!("internal response assembly error: {e}"),
        ),
    }
}

/// A classified refusal: `{"kind": ..., "message": ...}` with CLI message text.
fn tool_error(kind: &str, message: &str) -> CallToolResult {
    CallToolResult::structured_error(serde_json::json!({"kind": kind, "message": message}))
}

fn repo_status_doc(input: &RepoStatusInput) -> CallToolResult {
    let home = paths::config_dir();
    let reg = match registry::load(&paths::registry_path()) {
        Ok(reg) => reg,
        Err(e) => return tool_error("store_error", &e),
    };
    let entry = match registry::find(&reg, &input.repo) {
        Some(entry) => entry,
        None => {
            return tool_error(
                "not_registered",
                &format!("unknown repository: '{}'", input.repo),
            )
        }
    };
    let recorded = entry.root.clone();
    let canon = match paths::canonical_root(Path::new(&recorded)) {
        Ok(canon) => canon,
        Err(_) => {
            return tool_error(
                "not_registered",
                &format!("registered root is missing: '{recorded}'"),
            )
        }
    };
    let root = canon.to_string_lossy().to_string();
    // The record must still resolve to itself: a moved root or a
    // symlink/junction retarget is rejected instead of fingerprinted.
    if root != recorded {
        return tool_error(
            "not_registered",
            &format!("registered root moved or retargeted (record '{recorded}')"),
        );
    }
    let info = git::fingerprint(&canon);
    let (_, ignore) = ignore::load_rules(&canon);
    let index_status = match index_status_doc(&home, &input.repo, &recorded) {
        Ok(text) => text,
        Err((kind, message)) => return tool_error(kind, &message),
    };
    let mut value: serde_json::Value = match serde_json::from_str(&response::repo_status_json(
        &input.repo,
        &root,
        &info,
        &ignore,
    )) {
        Ok(value) => value,
        Err(e) => {
            return tool_error(
                "store_error",
                &format!("internal response assembly error: {e}"),
            )
        }
    };
    match serde_json::from_str(&index_status) {
        Ok(index_value) => value["index_status"] = index_value,
        Err(e) => {
            return tool_error(
                "store_error",
                &format!("internal response assembly error: {e}"),
            )
        }
    }
    CallToolResult::structured(value)
}

/// One migration pointer for outdated schemas: actionable CLI instruction,
/// identical for both index-reading tools.
fn outdated_message(repo: &str, version: u32) -> String {
    format!(
        "index schema v{version} is outdated; run `riltra index status {repo}` (or `riltra index {repo}`) with the CLI to migrate, then retry"
    )
}

/// `index status` through the read-only path; `Err` carries the classified
/// refusal so every index-reading tool reports the same guard.
fn index_status_doc(
    home: &Path,
    repo: &str,
    recorded: &str,
) -> Result<String, (&'static str, String)> {
    match store::open_read(&store::db_path(home, repo)) {
        store::IndexRead::Missing => Ok(response::index_status_json(
            repo,
            recorded,
            &index::Freshness {
                status: "unknown".to_string(),
                reasons: vec!["no-index".to_string()],
                snapshot: None,
            },
            false,
        )),
        store::IndexRead::Outdated(version) => {
            Err(("schema_outdated", outdated_message(repo, version)))
        }
        store::IndexRead::Ahead(version) => Err((
            "schema_ahead",
            format!(
                "database schema v{version} is newer than supported v{}",
                store::SCHEMA_VERSION
            ),
        )),
        store::IndexRead::Error(message) => Err(("store_error", message)),
        store::IndexRead::Ready(mut conn) => {
            // The FTS flag reflects build capability, not revision content.
            let fts = store::fts_available(&conn);
            let tx = match conn.transaction() {
                Ok(tx) => tx,
                Err(e) => return Err(("store_error", e.to_string())),
            };
            let freshness = index::evaluate_tx(&tx, Path::new(recorded));
            let _ = tx.commit();
            Ok(response::index_status_json(repo, recorded, &freshness, fts))
        }
    }
}

fn code_search_doc(input: &CodeSearchInput) -> CallToolResult {
    let limit = input.limit.unwrap_or(index::DEFAULT_SEARCH_LIMIT);
    let offset = input.offset.unwrap_or(0);
    if limit > index::MAX_SEARCH_LIMIT {
        return tool_error(
            "invalid_input",
            &format!("--limit exceeds maximum {}", index::MAX_SEARCH_LIMIT),
        );
    }
    let home = paths::config_dir();
    let reg = match registry::load(&paths::registry_path()) {
        Ok(reg) => reg,
        Err(e) => return tool_error("store_error", &e),
    };
    let entry = match registry::find(&reg, &input.repo) {
        Some(entry) => entry,
        None => {
            return tool_error(
                "not_registered",
                &format!("unknown repository: '{}'", input.repo),
            )
        }
    };
    let recorded = entry.root.clone();
    match store::open_read(&store::db_path(&home, &input.repo)) {
        store::IndexRead::Missing => search_doc(
            &input.repo,
            &recorded,
            None,
            "unknown",
            &["no-index".to_string()],
            input,
            limit,
            offset,
            0,
            &[],
        ),
        store::IndexRead::Outdated(version) => {
            tool_error("schema_outdated", &outdated_message(&input.repo, version))
        }
        store::IndexRead::Ahead(version) => tool_error(
            "schema_ahead",
            &format!(
                "database schema v{version} is newer than supported v{}",
                store::SCHEMA_VERSION
            ),
        ),
        store::IndexRead::Error(message) => tool_error("store_error", &message),
        store::IndexRead::Ready(mut conn) => {
            // One read transaction across snapshot read, verdict, and fact
            // rows: the response can never mix two revisions.
            let tx = match conn.transaction() {
                Ok(tx) => tx,
                Err(e) => return tool_error("store_error", &e.to_string()),
            };
            let snapshot = match store::latest_snapshot(&tx) {
                Ok(Some(snapshot)) => snapshot,
                _ => {
                    return tool_error(
                        "store_error",
                        "index database unreadable; rebuild with `index`",
                    )
                }
            };
            let canon_root = match Path::new(&recorded).canonicalize() {
                Ok(canon) => canon,
                Err(_) => {
                    let _ = tx.commit();
                    return search_doc(
                        &input.repo,
                        &recorded,
                        Some(&snapshot),
                        "unknown",
                        &["root-unreadable".to_string()],
                        input,
                        limit,
                        offset,
                        0,
                        &[],
                    );
                }
            };
            let (status, reasons) =
                index::evaluate_snapshot(&tx, &input.repo, &canon_root, &snapshot);
            let (total, hits) =
                match store::search_symbols(&tx, snapshot.id, &input.query, limit, offset) {
                    Ok(found) => found,
                    Err(e) => return tool_error("store_error", &e),
                };
            let _ = tx.commit();
            search_doc(
                &input.repo,
                &snapshot.root,
                Some(&snapshot),
                &status,
                &reasons,
                input,
                limit,
                offset,
                total,
                &hits,
            )
        }
    }
}

/// One search document through the shared CP2 formatter.
#[allow(clippy::too_many_arguments)]
fn search_doc(
    repo: &str,
    root: &str,
    snapshot: Option<&store::Snapshot>,
    freshness: &str,
    reasons: &[String],
    input: &CodeSearchInput,
    limit: usize,
    offset: usize,
    total: usize,
    hits: &[store::Hit],
) -> CallToolResult {
    doc(&response::search_response_json(&response::SearchResponse {
        repo,
        root,
        snapshot,
        freshness,
        reasons,
        symbol: &input.query,
        limit,
        offset,
        total,
        hits,
    }))
}

/// Run the MCP stdio server to completion. Exit codes follow the CLI:
/// 0 clean shutdown (client EOF/disconnect), 3 runtime failure.
pub(crate) fn run() -> i32 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("error: mcp runtime: {e}");
            return 3;
        }
    };
    runtime.block_on(async {
        let service = match serve_server(RiltraMcp, rmcp::transport::io::stdio()).await {
            Ok(service) => service,
            Err(e) => {
                eprintln!("error: mcp initialize failed: {e}");
                return 3;
            }
        };
        match service.waiting().await {
            Ok(rmcp::service::QuitReason::JoinError(e)) => {
                eprintln!("error: mcp task failed: {e}");
                3
            }
            Ok(_) => 0,
            Err(e) => {
                eprintln!("error: mcp task failed: {e}");
                3
            }
        }
    })
}
