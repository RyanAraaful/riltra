// SPDX-License-Identifier: Apache-2.0
// M1: native CLI + repository registry. No networking; repository code is never
// executed (git subprocesses are read-only plumbing invoked by explicit user
// commands with GIT_TERMINAL_PROMPT=0).
//
// Exit codes: 0 ok, 2 usage error, 3 repository/data error. Successful commands
// print JSON to stdout; `repo status` and `doctor` print JSON even on data
// errors so scripts stay machine-readable.

mod args;
mod doctor;
mod edges;
mod git;
mod ignore;
mod index;
mod json;
mod knowledge_store;
mod lsp;
mod paths;
mod registry;
mod scan;
mod scip;
mod scip_proto;
mod semantic;
mod store;
mod syntax;

#[cfg(test)]
mod testutil;

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use args::Command;

const NAME: &str = "riltra";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const SCOPE: &str = "m1-registry";

pub(crate) fn usage() -> String {
    format!(
        "Usage:\n  {NAME} [--help] [--version] status\n  {NAME} init --repo <path> [--name <name>]\n  {NAME} repo list\n  {NAME} repo status --repo <name>\n  {NAME} doctor [--repo <name>]\n  {NAME} scan --repo <name> [--path <relpath>]... [--symbol <substr>]\n  {NAME} index --repo <name> [--full]\n  {NAME} index status --repo <name>\n  {NAME} search --repo <name> --symbol <substr> [--limit <n>] [--offset <n>]
  {NAME} semantic import --repo <name> --scip <file> [--source-digest <hex>]
  {NAME} semantic status --repo <name>
  {NAME} semantic query --repo <name> --path <rel> --line <n> --col <n> [--refs] [--lsp-server <abs-path>]
  {NAME} callers --repo <name> --path <rel> --line <n> --col <n> [--symbol <uri>] [--limit <n>] [--offset <n>]
  {NAME} callees --repo <name> --path <rel> --line <n> --col <n> [--symbol <uri>] [--limit <n>] [--offset <n>]
  {NAME} trace --repo <name> --path <rel> --line <n> --col <n> [--symbol <uri>] [--to <path:line:col>] [--depth <n>] [--limit <n>] [--offset <n>]\n  {NAME} impact --repo <name> --path <rel> --line <n> --col <n> [--symbol <uri>] [--depth <n>] [--limit <n>] [--offset <n>]\n\nCommands:\n  status       Print deterministic bootstrap status as JSON\n  init         Register a local repository root\n  repo list    List registered repositories as JSON\n  repo status  Git/worktree fingerprint of a registered repository\n  doctor       Path-security and repository health checks as JSON\n  scan         Syntax facts (definitions/imports/calls) as JSON\n  index        Build the durable code index for a repository\n  index status Report index freshness (fresh/stale/unknown) as JSON\n  search       Search indexed symbols as JSON\n  semantic import Import a SCIP index against the live snapshot as JSON\n  semantic status Per-provider binding verification as JSON\n  semantic query Single-hop definition/reference lookup as JSON\n  callers Direct callers of the anchored identity as JSON\n  callees Direct callees of the anchored identity as JSON\n  trace Bounded plausible static paths with per-hop evidence\n  impact Conservative inbound impact with evidence and UNKNOWN default\n"
    )
}

fn version_line() -> String {
    format!("{NAME} {VERSION}")
}

// Single-line JSON with fixed key order so output is stable.
fn status_json() -> String {
    format!(
        "{{\"name\":\"{NAME}\",\"version\":\"{VERSION}\",\"scope\":\"{SCOPE}\",\"networking\":\"disabled\"}}"
    )
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn cmd_init(repo: &str, name: Option<&str>, out: &mut String, err: &mut String) -> i32 {
    let reg_path = paths::registry_path();
    let mut reg = match registry::load(&reg_path) {
        Ok(r) => r,
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            return 3;
        }
    };
    let canon = match paths::canonical_root(Path::new(repo)) {
        Ok(c) => c,
        Err(e) => {
            err.push_str(&format!("error: cannot register '{repo}': {e}\n"));
            return 3;
        }
    };
    let root = canon.to_string_lossy().to_string();
    let name = match name {
        Some(n) => n.to_string(),
        None => match canon.file_name().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => {
                err.push_str("error: cannot derive a name for this root; pass --name\n");
                return 3;
            }
        },
    };
    if !paths::validate_name(&name) {
        err.push_str(&format!("error: invalid repository name: '{name}'\n"));
        return 2;
    }
    let is_new = match registry::add(&mut reg, &name, &root, now_unix()) {
        Ok(is_new) => is_new,
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            return 3;
        }
    };
    if let Err(e) = registry::save(&reg_path, &reg) {
        err.push_str(&format!("error: {e}\n"));
        return 3;
    }
    *out = format!(
        "{{\"name\":\"{}\",\"root\":\"{}\",\"registered\":{}}}\n",
        json::escape(&name),
        json::escape(&root),
        is_new
    );
    0
}

fn cmd_repo_list(out: &mut String, err: &mut String) -> i32 {
    match registry::load(&paths::registry_path()) {
        Ok(reg) => {
            *out = registry::list_json(&reg) + "\n";
            0
        }
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            3
        }
    }
}

fn repo_status_json(name: &str, root: &str, g: &git::GitInfo, ig: &ignore::IgnoreInfo) -> String {
    format!(
        "{{\"name\":\"{}\",\"root\":\"{}\",\"git\":{},\"ignore\":{}}}",
        json::escape(name),
        json::escape(root),
        git::git_json(g),
        ignore::ignore_json(ig)
    )
}

fn cmd_repo_status(name: &str, out: &mut String, err: &mut String) -> i32 {
    let reg = match registry::load(&paths::registry_path()) {
        Ok(r) => r,
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            return 3;
        }
    };
    let entry = match registry::find(&reg, name) {
        Some(e) => e,
        None => {
            err.push_str(&format!("error: unknown repository: '{name}'\n"));
            return 3;
        }
    };
    let recorded = entry.root.clone();
    let canon = match paths::canonical_root(Path::new(&recorded)) {
        Ok(c) => c,
        Err(_) => {
            err.push_str(&format!(
                "error: registered root is missing: '{recorded}'\n"
            ));
            return 3;
        }
    };
    let root = canon.to_string_lossy().to_string();
    // The record must still resolve to itself: a moved root or a
    // symlink/junction retarget is rejected instead of fingerprinted.
    if root != recorded {
        err.push_str(&format!(
            "error: registered root moved or retargeted (record '{recorded}')\n"
        ));
        return 3;
    }
    let info = git::fingerprint(&canon);
    let (_, ignore) = ignore::load_rules(&canon);
    *out = repo_status_json(name, &root, &info, &ignore) + "\n";
    if info.present && !info.valid {
        3
    } else {
        0
    }
}

fn resolve_recorded_root(name: &str, err: &mut String) -> Result<(String, String), i32> {
    let reg = match registry::load(&paths::registry_path()) {
        Ok(r) => r,
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            return Err(3);
        }
    };
    let entry = match registry::find(&reg, name) {
        Some(e) => e,
        None => {
            err.push_str(&format!("error: unknown repository: '{name}'\n"));
            return Err(3);
        }
    };
    let recorded = entry.root.clone();
    let canon = match paths::canonical_root(Path::new(&recorded)) {
        Ok(c) => c,
        Err(_) => {
            err.push_str(&format!(
                "error: registered root is missing: '{recorded}'\n"
            ));
            return Err(3);
        }
    };
    let root = canon.to_string_lossy().to_string();
    if root != recorded {
        err.push_str(&format!(
            "error: registered root moved or retargeted (record '{recorded}')\n"
        ));
        return Err(3);
    }
    Ok((root, recorded))
}

fn cmd_index(name: &str, full: bool, out: &mut String, err: &mut String) -> i32 {
    let home = paths::config_dir();
    let (root, _) = match resolve_recorded_root(name, err) {
        Ok(paths) => paths,
        Err(code) => return code,
    };
    match index::build(&home, name, Path::new(&root), full) {
        Ok(summary) => {
            *out = format!(
                "{{\"repo\":\"{}\",\"root\":\"{}\",\"snapshot\":{},\"status_at_write\":\"{}\",\"mode\":\"{}\",\"reused\":{},\"reparsed\":{},\"added\":{},\"removed\":{},\"fallback_reason\":\"{}\",\"timings_ms\":{{\"diff\":{},\"ingest\":{},\"verify\":{},\"promote\":{}}},\"files\":{{\"discovered\":{},\"processed\":{},\"definitions\":{},\"imports\":{},\"calls\":{},\"errors\":{},\"skipped_ignored\":{},\"skipped_large\":{},\"skipped_binary\":{},\"skipped_symlink\":{},\"skipped_escape\":{},\"skipped_unreadable\":{},\"unsupported\":{}}}}}",
                json::escape(name),
                json::escape(&root),
                summary.snapshot_id,
                summary.status_at_write,
                summary.mode,
                summary.reused,
                summary.reparsed,
                summary.added,
                summary.removed,
                summary.fallback_reason,
                summary.timings_ms.diff,
                summary.timings_ms.ingest,
                summary.timings_ms.verify,
                summary.timings_ms.promote,
                summary.files_discovered,
                summary.processed,
                summary.definitions,
                summary.imports,
                summary.calls,
                summary.errors,
                summary.skipped_ignored,
                summary.skipped_large,
                summary.skipped_binary,
                summary.skipped_symlink,
                summary.skipped_escape,
                summary.skipped_unreadable,
                summary.unsupported_total
            ) + "\n";
            0
        }
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            3
        }
    }
}

fn snapshot_json(snapshot: &store::Snapshot) -> String {
    format!(
        "{{\"id\":{},\"created_unix\":{},\"head\":{},\"branch\":{},\"dirty\":{},\"status_at_write\":\"{}\",\"nonce\":\"{}\"}}",
        snapshot.id,
        snapshot.created_unix,
        match &snapshot.head {
            Some(h) => format!("\"{}\"", json::escape(h)),
            None => "null".to_string(),
        },
        match &snapshot.branch {
            Some(b) => format!("\"{}\"", json::escape(b)),
            None => "null".to_string(),
        },
        snapshot.dirty,
        snapshot.status_at_write,
        snapshot.nonce
    )
}

fn cmd_index_status(name: &str, out: &mut String, err: &mut String) -> i32 {
    let home = paths::config_dir();
    let reg = match registry::load(&paths::registry_path()) {
        Ok(r) => r,
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            return 3;
        }
    };
    let entry = match registry::find(&reg, name) {
        Some(e) => e,
        None => {
            err.push_str(&format!("error: unknown repository: '{name}'\n"));
            return 3;
        }
    };
    let recorded = entry.root.clone();
    let freshness = index::evaluate(&home, name, Path::new(&recorded));
    // The FTS flag reflects build capability, not revision content, so a
    // separate connection is safe here.
    let fts = store::open_live(&store::db_path(&home, name))
        .map(|conn| store::fts_available(&conn))
        .unwrap_or(false);
    let reasons: Vec<String> = freshness
        .reasons
        .iter()
        .map(|r| format!("\"{r}\""))
        .collect();
    *out = format!(
        "{{\"repo\":\"{}\",\"root\":\"{}\",\"freshness\":\"{}\",\"reasons\":[{}],\"fts\":{},\"snapshot\":{}}}",
        json::escape(name),
        json::escape(&recorded),
        freshness.status,
        reasons.join(","),
        fts,
        match &freshness.snapshot {
            Some(snapshot) => snapshot_json(snapshot),
            None => "null".to_string(),
        }
    ) + "\n";
    0
}

fn parse_count(value: Option<&str>, fallback: usize, arg: &str) -> Result<usize, String> {
    match value {
        None => Ok(fallback),
        Some(text) => text
            .parse::<usize>()
            .map_err(|_| format!("invalid --{arg} value: '{text}'")),
    }
}

fn cmd_search(
    name: &str,
    symbol: &str,
    limit: Option<&str>,
    offset: Option<&str>,
    out: &mut String,
    err: &mut String,
) -> i32 {
    let limit = match parse_count(limit, index::DEFAULT_SEARCH_LIMIT, "limit") {
        Ok(n) => n,
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            return 2;
        }
    };
    let offset = match parse_count(offset, 0, "offset") {
        Ok(n) => n,
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            return 2;
        }
    };
    if limit > index::MAX_SEARCH_LIMIT {
        err.push_str(&format!(
            "error: --limit exceeds maximum {}\n",
            index::MAX_SEARCH_LIMIT
        ));
        return 2;
    }
    let home = paths::config_dir();
    let reg = match registry::load(&paths::registry_path()) {
        Ok(r) => r,
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            return 3;
        }
    };
    let entry = match registry::find(&reg, name) {
        Some(e) => e,
        None => {
            err.push_str(&format!("error: unknown repository: '{name}'\n"));
            return 3;
        }
    };
    let recorded = entry.root.clone();
    let db = store::db_path(&home, name);
    if !db.exists() {
        *out = format!(
            "{{\"repo\":\"{}\",\"root\":\"{}\",\"snapshot\":null,\"freshness\":\"unknown\",\"reasons\":[\"no-index\"],\"query\":{{\"symbol\":\"{}\",\"limit\":{},\"offset\":{}}},\"total\":0,\"results\":[],\"truncated\":false}}\n",
            json::escape(name),
            json::escape(&recorded),
            json::escape(symbol),
            limit,
            offset
        );
        return 0;
    }
    // One read transaction across snapshot read, verdict, and fact rows:
    // the response can never mix two revisions.
    let mut conn = match store::open_live(&db) {
        Ok(c) => c,
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            return 3;
        }
    };
    let tx = match conn.transaction() {
        Ok(tx) => tx,
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            return 3;
        }
    };
    let snapshot = match store::latest_snapshot(&tx) {
        Ok(Some(snapshot)) => snapshot,
        _ => {
            err.push_str("error: index database unreadable; rebuild with `index`\n");
            return 3;
        }
    };
    let canon_root = match Path::new(&recorded).canonicalize() {
        Ok(c) => c,
        Err(_) => {
            let _ = tx.commit();
            *out = format!(
                "{{\"repo\":\"{}\",\"root\":\"{}\",\"snapshot\":{},\"freshness\":\"unknown\",\"reasons\":[\"root-unreadable\"],\"query\":{{\"symbol\":\"{}\",\"limit\":{},\"offset\":{}}},\"total\":0,\"results\":[],\"truncated\":false}}\n",
                json::escape(name),
                json::escape(&recorded),
                snapshot_json(&snapshot),
                json::escape(symbol),
                limit,
                offset
            );
            return 0;
        }
    };
    let (status, reasons) = index::evaluate_snapshot(&tx, name, &canon_root, &snapshot);
    let (total, hits) = match store::search_symbols(&tx, snapshot.id, symbol, limit, offset) {
        Ok(found) => found,
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            return 3;
        }
    };
    let _ = tx.commit();
    let results: Vec<String> = hits
        .iter()
        .map(|hit| {
            format!(
                "{{\"path\":\"{}\",\"kind\":\"{}\",\"name\":{},\"span\":{{\"start_line\":{},\"start_col\":{},\"end_line\":{},\"end_col\":{}}},\"bytes\":[{},{}],\"provenance\":{{\"provider\":\"{}\",\"grammar\":\"{}\",\"language\":\"{}\"}},\"snapshot\":{}}}",
                json::escape(&hit.path),
                hit.kind,
                match &hit.name {
                    Some(n) => format!("\"{}\"", json::escape(n)),
                    None => "null".to_string(),
                },
                hit.start_line,
                hit.start_col,
                hit.end_line,
                hit.end_col,
                hit.byte_start,
                hit.byte_end,
                hit.provider,
                hit.grammar,
                hit.language,
                hit.snapshot_id
            )
        })
        .collect();
    let truncated = offset + results.len() < total;
    *out = format!(
        "{{\"repo\":\"{}\",\"root\":\"{}\",\"snapshot\":{},\"freshness\":\"{}\",\"reasons\":[{}],\"query\":{{\"symbol\":\"{}\",\"limit\":{},\"offset\":{}}},\"total\":{},\"results\":[{}],\"truncated\":{}}}",
        json::escape(name),
        json::escape(&snapshot.root),
        snapshot_json(&snapshot),
        status,
        reasons.iter().map(|r| format!("\"{r}\"")).collect::<Vec<_>>().join(","),
        json::escape(symbol),
        limit,
        offset,
        total,
        results.join(","),
        truncated
    ) + "\n";
    0
}

fn cmd_scan(
    name: &str,
    paths: &[String],
    symbol: Option<&str>,
    out: &mut String,
    err: &mut String,
) -> i32 {
    let reg = match registry::load(&paths::registry_path()) {
        Ok(r) => r,
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            return 3;
        }
    };
    let entry = match registry::find(&reg, name) {
        Some(e) => e,
        None => {
            err.push_str(&format!("error: unknown repository: '{name}'\n"));
            return 3;
        }
    };
    let recorded = entry.root.clone();
    let canon = match paths::canonical_root(Path::new(&recorded)) {
        Ok(c) => c,
        Err(_) => {
            err.push_str(&format!(
                "error: registered root is missing: '{recorded}'\n"
            ));
            return 3;
        }
    };
    if canon.to_string_lossy() != recorded {
        err.push_str(&format!(
            "error: registered root moved or retargeted (record '{recorded}')\n"
        ));
        return 3;
    }
    let opts = scan::ScanOptions {
        paths: paths.to_vec(),
        symbol: symbol.map(str::to_string),
    };
    match scan::scan_repo(name, &canon, &opts) {
        Ok(json) => {
            *out = json + "\n";
            0
        }
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            3
        }
    }
}

fn cmd_doctor(repo: Option<&str>, out: &mut String, err: &mut String) -> i32 {
    match repo {
        None => {
            let reg_ok = registry::load(&paths::registry_path()).is_ok();
            let git_ok = git::git_available();
            let checks = [
                doctor::Check {
                    name: "registry-readable".to_string(),
                    pass: reg_ok,
                    detail: if reg_ok {
                        "registry file loads".to_string()
                    } else {
                        "registry file unreadable".to_string()
                    },
                },
                doctor::Check {
                    name: "git-available".to_string(),
                    pass: git_ok,
                    detail: if git_ok {
                        "git executable found".to_string()
                    } else {
                        "git executable not found".to_string()
                    },
                },
            ];
            let pass = checks.iter().all(|c| c.pass);
            let inner: Vec<String> = checks
                .iter()
                .map(|c| {
                    format!(
                        "{{\"name\":\"{}\",\"pass\":{},\"detail\":\"{}\"}}",
                        json::escape(&c.name),
                        c.pass,
                        json::escape(&c.detail)
                    )
                })
                .collect();
            *out = format!(
                "{{\"scope\":\"environment\",\"pass\":{},\"checks\":[{}]}}\n",
                pass,
                inner.join(",")
            );
            if pass {
                0
            } else {
                3
            }
        }
        Some(name) => {
            let reg = match registry::load(&paths::registry_path()) {
                Ok(r) => r,
                Err(e) => {
                    err.push_str(&format!("error: {e}\n"));
                    return 3;
                }
            };
            let entry = match registry::find(&reg, name) {
                Some(e) => e,
                None => {
                    err.push_str(&format!("error: unknown repository: '{name}'\n"));
                    return 3;
                }
            };
            let recorded = entry.root.clone();
            let canon = match paths::canonical_root(Path::new(&recorded)) {
                Ok(c) => c,
                Err(_) => {
                    *out = doctor::doctor_json(
                        name,
                        &recorded,
                        &[doctor::Check {
                            name: "root-exists-dir".to_string(),
                            pass: false,
                            detail: format!("root {recorded} is missing or not a directory"),
                        }],
                    ) + "\n";
                    return 3;
                }
            };
            let info = git::fingerprint(&canon);
            let (_, ignore) = ignore::load_rules(&canon);
            let root = canon.to_string_lossy().to_string();
            let checks = doctor::run_checks(name, &canon, &recorded, &info, &ignore);
            let pass = checks.iter().all(|c| c.pass);
            *out = doctor::doctor_json(name, &root, &checks) + "\n";
            if pass {
                0
            } else {
                3
            }
        }
    }
}

fn dispatch(cmd: Command, out: &mut String, err: &mut String) -> i32 {
    match cmd {
        Command::Help => {
            *out = usage();
            0
        }
        Command::Version => {
            *out = version_line() + "\n";
            0
        }
        Command::Status => {
            *out = status_json() + "\n";
            0
        }
        Command::Init { repo, name } => cmd_init(&repo, name.as_deref(), out, err),
        Command::RepoList => cmd_repo_list(out, err),
        Command::RepoStatus { repo } => cmd_repo_status(&repo, out, err),
        Command::Doctor { repo } => cmd_doctor(repo.as_deref(), out, err),
        Command::Scan {
            repo,
            paths,
            symbol,
        } => cmd_scan(&repo, &paths, symbol.as_deref(), out, err),
        Command::Index { repo, full } => cmd_index(&repo, full, out, err),
        Command::IndexStatus { repo } => cmd_index_status(&repo, out, err),
        Command::Search {
            repo,
            symbol,
            limit,
            offset,
        } => cmd_search(
            &repo,
            &symbol,
            limit.as_deref(),
            offset.as_deref(),
            out,
            err,
        ),
        Command::Semantic { op } => cmd_semantic(op, out, err),
        Command::Edge { op } => cmd_edge(op, out, err),
        Command::Trace(args) => cmd_trace(args, out, err),
        Command::Impact(args) => cmd_graph(args, out, err, true),
    }
}

fn cmd_semantic(op: args::SemanticOp, out: &mut String, err: &mut String) -> i32 {
    match op {
        args::SemanticOp::Import { repo, scip, claim } => {
            cmd_semantic_import(&repo, &scip, claim.as_deref(), out, err)
        }
        args::SemanticOp::Status { repo } => cmd_semantic_status(&repo, out, err),
        args::SemanticOp::Query {
            repo,
            path,
            line,
            col,
            refs,
            lsp_server,
        } => {
            let query = QueryArgs {
                path: &path,
                line: &line,
                col: &col,
                refs,
                lsp_server: lsp_server.as_deref(),
            };
            cmd_semantic_query(&repo, &query, out, err)
        }
    }
}

/// Bundled `semantic query` arguments: one struct instead of an
/// eight-parameter tail.
struct QueryArgs<'a> {
    path: &'a str,
    line: &'a str,
    col: &'a str,
    refs: bool,
    lsp_server: Option<&'a str>,
}

fn cmd_edge(op: args::EdgeOp, out: &mut String, err: &mut String) -> i32 {
    let (repo, path, line, col, symbol, limit, offset, dir) = match op {
        args::EdgeOp::Callers {
            repo,
            path,
            line,
            col,
            symbol,
            limit,
            offset,
        } => (
            repo,
            path,
            line,
            col,
            symbol,
            limit,
            offset,
            index::EdgeDir::Callers,
        ),
        args::EdgeOp::Callees {
            repo,
            path,
            line,
            col,
            symbol,
            limit,
            offset,
        } => (
            repo,
            path,
            line,
            col,
            symbol,
            limit,
            offset,
            index::EdgeDir::Callees,
        ),
    };
    let home = paths::config_dir();
    let (root, _) = match resolve_recorded_root(&repo, err) {
        Ok(paths) => paths,
        Err(code) => return code,
    };
    let line_number: u32 = match line.parse() {
        Ok(n) if n > 0 => n,
        _ => {
            err.push_str(&format!("error: invalid --line value: '{line}'\n"));
            return 2;
        }
    };
    let col_number: u32 = match col.parse() {
        Ok(n) if n > 0 => n,
        _ => {
            err.push_str(&format!("error: invalid --col value: '{col}'\n"));
            return 2;
        }
    };
    let limit_number = match parse_count(limit.as_deref(), index::DEFAULT_SEARCH_LIMIT, "limit") {
        Ok(n) => n,
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            return 2;
        }
    };
    let offset_number = match parse_count(offset.as_deref(), 0, "offset") {
        Ok(n) => n,
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            return 2;
        }
    };
    if limit_number == 0 || limit_number > index::MAX_SEARCH_LIMIT {
        err.push_str(&format!(
            "error: --limit must be between 1 and {}\n",
            index::MAX_SEARCH_LIMIT
        ));
        return 2;
    }
    if offset_number > i64::MAX as usize - index::MAX_SEARCH_LIMIT {
        err.push_str("error: --offset exceeds supported range\n");
        return 2;
    }
    let request = index::EdgeRequest {
        anchor: index::SpanQuery {
            rel: &path,
            line1: line_number,
            col1: col_number,
            refs: false,
        },
        symbol: symbol.as_deref(),
        direction: dir,
        limit: limit_number,
        offset: offset_number,
    };
    match index::edge_query(&home, &repo, Path::new(&root), &request) {
        Ok(o) => {
            *out = edge_query_json(&o) + "\n";
            0
        }
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            3
        }
    }
}

fn edge_string(value: &str) -> String {
    format!("\"{}\"", json::escape(value))
}
fn edge_optional(value: Option<&str>) -> String {
    value.map(edge_string).unwrap_or_else(|| "null".to_string())
}
fn edge_number<T: std::fmt::Display>(value: Option<T>) -> String {
    value
        .map(|n| n.to_string())
        .unwrap_or_else(|| "null".to_string())
}
fn edge_span(value: Option<(usize, usize)>) -> String {
    value
        .map(|(a, b)| format!("[{a},{b}]"))
        .unwrap_or_else(|| "null".to_string())
}
fn edge_strings(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|s| edge_string(s))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn edge_endpoint_json(endpoint: &Option<store::EdgeEndpoint>) -> String {
    match endpoint {
        Some(store::EdgeEndpoint::Scip { import_id, symbol_uri, doc }) => format!("{{\"type\":\"scip\",\"import_id\":{import_id},\"symbol_uri\":{},\"doc\":{},\"path\":null,\"byte_start\":null,\"byte_end\":null}}", edge_string(symbol_uri), edge_optional(doc.as_deref())),
        Some(store::EdgeEndpoint::SyntaxDef { path, byte_start, byte_end }) => format!("{{\"type\":\"syntax_def\",\"import_id\":null,\"symbol_uri\":null,\"doc\":null,\"path\":{},\"byte_start\":{byte_start},\"byte_end\":{byte_end}}}", edge_string(path)),
        None => "null".to_string(),
    }
}

fn edge_hit_json(hit: &index::EdgeHitOut) -> String {
    let e = &hit.edge;
    let imports = hit.evidence.imports.iter().map(|(id, i)| format!("{{\"id\":{id},\"provider\":{},\"version\":{},\"artifact_id\":{},\"source_revision\":{},\"content_identity\":{}}}", edge_string(&i.provider_name), edge_string(&i.provider_version), edge_string(&i.artifact_id), edge_string(&i.source_revision), edge_string(&i.content_identity))).collect::<Vec<_>>().join(",");
    // Legacy convenience fields retain only a UNIQUE proven import, never an
    // arbitrary first overlap. Complete provenance is in evidence.imports.
    let unique = if hit.evidence.imports.len() == 1 {
        hit.evidence.imports.first()
    } else {
        None
    };
    format!("{{\"kind\":{},\"precision\":{},\"caller\":{},\"callee\":{},\"caller_path\":{},\"caller_span\":{},\"callee_name\":{},\"provider\":{},\"import_id\":{},\"tool\":{},\"verification\":{},\"unresolved\":{},\"evidence\":{{\"path\":{},\"span\":{},\"callee_span\":{},\"terminal_span\":{},\"reference_span\":{},\"digest\":{},\"provider\":{},\"imports\":[{imports}]}}}}",
        edge_string(&e.edge_kind), edge_string(&e.precision), edge_endpoint_json(&e.caller), edge_endpoint_json(&e.callee), edge_string(&e.caller_path), edge_span(Some(e.caller_span_bytes)), edge_string(&e.callee_name), edge_string(&e.provider), edge_number(unique.map(|(id,_)| id)), edge_optional(unique.map(|(_,i)| i.provider_name.as_str())), edge_string(&hit.verification), edge_optional(e.unresolved.as_deref()),
        edge_string(&e.caller_path), edge_span(Some(e.caller_span_bytes)), edge_span(hit.evidence.callee_span), edge_span(hit.evidence.terminal_span), edge_span(hit.evidence.reference_span), edge_string(&hit.evidence.digest), edge_string(&e.provider))
}

fn edge_query_json(o: &index::EdgeQueryOut) -> String {
    let a = &o.anchor;
    let edges = o
        .edges
        .iter()
        .map(edge_hit_json)
        .collect::<Vec<_>>()
        .join(",");
    let denominator = o
        .completeness
        .denominator
        .iter()
        .map(|(k, v)| format!("{}:{v}", edge_string(k)))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"schema_version\":1,\"repo\":{},\"snapshot\":{{\"id\":{},\"nonce\":{},\"revision\":{}}},\"direction\":{},\"anchor\":{{\"kind\":{},\"path\":{},\"line\":{},\"col\":{},\"symbol_uri\":{},\"import_id\":{},\"precision\":{},\"anchor_uncertainty\":{},\"span\":{},\"endpoint\":{}}},\"freshness\":{{\"status\":{},\"reasons\":{}}},\"verification\":{},\"completeness\":{{\"status\":{},\"covered\":{},\"total\":{},\"denominator\":{{{denominator}}},\"unresolved\":{}}},\"edges\":[{edges}],\"truncated\":{},\"limits\":{{\"limit\":{},\"offset\":{},\"next_offset\":{}}}}}",
        edge_string(&o.repo), o.snapshot.id, edge_string(&o.snapshot.nonce), edge_string(&o.revision), edge_string(o.direction.as_str()), edge_string(a.kind.as_str()), edge_string(&a.path), a.line1, a.col1, edge_optional(a.symbol_uri.as_deref()), edge_number(a.import_id), edge_string(&a.precision), edge_optional(a.uncertainty.as_deref()), edge_span(a.span_bytes), edge_endpoint_json(&index::anchor_endpoint(a)),
        edge_string(&o.freshness), edge_strings(&o.freshness_reasons), edge_string(&o.verification), edge_string(&o.completeness.status), o.completeness.covered, o.completeness.total, edge_strings(&o.completeness.unresolved), o.truncated, o.limit, o.offset, edge_number(o.next_offset))
}

fn trace_position(value: &str, flag: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .ok()
        .filter(|n| *n > 0)
        .ok_or_else(|| format!("invalid {flag} value: '{value}'"))
}

fn cmd_trace(args: args::TraceArgs, out: &mut String, err: &mut String) -> i32 {
    cmd_graph(args, out, err, false)
}

fn cmd_graph(args: args::TraceArgs, out: &mut String, err: &mut String, impact: bool) -> i32 {
    let parsed = (|| {
        let line = trace_position(&args.line, "--line")?;
        let col = trace_position(&args.col, "--col")?;
        let depth = parse_count(args.depth.as_deref(), 3, "depth")?;
        if !(1..=edges::TRACE_MAX_DEPTH).contains(&depth) {
            return Err("--depth must be between 1 and 6".to_string());
        }
        let limit = parse_count(args.limit.as_deref(), index::DEFAULT_SEARCH_LIMIT, "limit")?;
        if !(1..=index::MAX_SEARCH_LIMIT).contains(&limit) {
            return Err("--limit must be between 1 and 1000".to_string());
        }
        let offset = parse_count(args.offset.as_deref(), 0, "offset")?;
        if offset > i64::MAX as usize - index::MAX_SEARCH_LIMIT {
            return Err("--offset exceeds supported range".to_string());
        }
        let to = args
            .to
            .as_deref()
            .map(|to| {
                let (rest, col) = to.rsplit_once(':').ok_or("--to must be PATH:LINE:COL")?;
                let (path, line) = rest.rsplit_once(':').ok_or("--to must be PATH:LINE:COL")?;
                if path.is_empty() {
                    return Err("--to requires a path".to_string());
                }
                Ok((
                    path.to_string(),
                    trace_position(line, "--to line")?,
                    trace_position(col, "--to col")?,
                ))
            })
            .transpose()?;
        Ok((line, col, depth, limit, offset, to))
    })();
    let (line, col, depth, limit, offset, to) = match parsed {
        Ok(p) => p,
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            return 2;
        }
    };
    let (root, _) = match resolve_recorded_root(&args.repo, err) {
        Ok(p) => p,
        Err(code) => return code,
    };
    let request = index::TraceRequest {
        source: index::SpanQuery {
            rel: &args.path,
            line1: line,
            col1: col,
            refs: false,
        },
        symbol: args.symbol.as_deref(),
        destination: to.as_ref().map(|(p, l, c)| index::SpanQuery {
            rel: p,
            line1: *l,
            col1: *c,
            refs: false,
        }),
        depth,
        limit,
        offset,
    };
    let query = if impact {
        index::impact_query
    } else {
        index::trace_query
    };
    match query(&paths::config_dir(), &args.repo, Path::new(&root), &request) {
        Ok(result) => {
            *out = trace_query_json(&result) + "\n";
            0
        }
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            3
        }
    }
}

fn trace_anchor_json(anchor: &index::EdgeAnchor) -> String {
    format!("{{\"kind\":{},\"path\":{},\"line\":{},\"col\":{},\"symbol_uri\":{},\"import_id\":{},\"precision\":{},\"anchor_uncertainty\":{},\"span\":{},\"endpoint\":{}}}",edge_string(anchor.kind.as_str()),edge_string(&anchor.path),anchor.line1,anchor.col1,edge_optional(anchor.symbol_uri.as_deref()),edge_number(anchor.import_id),edge_string(&anchor.precision),edge_optional(anchor.uncertainty.as_deref()),edge_span(anchor.span_bytes),edge_endpoint_json(&index::anchor_endpoint(anchor)))
}

fn trace_query_json(o: &index::TraceQueryOut) -> String {
    let direction = if o.impact_verdict.is_some() {
        "impact"
    } else {
        "trace"
    };
    let disclaimer = if o.impact_verdict.is_some() {
        "plausible-impact-not-execution"
    } else {
        "plausible-path-not-execution"
    };
    let verdict = o
        .impact_verdict
        .map(|v| format!("\"verdict\":{},", edge_string(v)))
        .unwrap_or_default();
    let paths = o.paths.iter().map(|p| {
        let edges=p.edges.iter().map(edge_hit_json).collect::<Vec<_>>().join(",");
        let distance = p.edges.len();
        let scope = if distance == 1 { "direct" } else { "transitive" };
        let impact = if o.impact_verdict.is_some() { format!("\"distance\":{distance},\"scope\":\"{scope}\",") } else { String::new() };
        format!("{{{impact}\"edges\":[{edges}],\"cycle\":{},\"truncated\":{},\"verification\":{},\"unresolved\":{}}}",p.cycle,p.truncated,edge_string(&p.verification),edge_strings(&p.unresolved))
    }).collect::<Vec<_>>().join(",");
    let bridges = o
        .bridges
        .iter()
        .map(|b| {
            format!(
                "{{\"semantic\":{},\"syntax\":{},\"definition_span\":{},\"digest\":{}}}",
                edge_endpoint_json(&Some(b.semantic.clone())),
                edge_endpoint_json(&Some(b.syntax.clone())),
                edge_span(Some(b.definition_span)),
                edge_string(&b.digest)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let denominator = o
        .completeness
        .denominator
        .iter()
        .map(|(k, v)| format!("{}:{v}", edge_string(k)))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{verdict}\"schema_version\":1,\"repo\":{},\"snapshot\":{{\"id\":{},\"nonce\":{},\"revision\":{}}},\"direction\":\"{direction}\",\"anchor\":{},\"destination\":{},\"freshness\":{{\"status\":{},\"reasons\":{}}},\"verification\":{},\"completeness\":{{\"status\":{},\"covered\":{},\"total\":{},\"total_exact\":{},\"denominator\":{{{denominator}}},\"unresolved\":{}}},\"paths\":[{paths}],\"bridges\":[{bridges}],\"visits\":{{\"nodes\":{},\"edges\":{}}},\"budgets\":{{\"depth\":{},\"max_depth\":6,\"nodes\":500,\"edges\":3000,\"paths\":1000,\"mappings\":500,\"time_ms\":2000}},\"limits\":{{\"limit\":{},\"offset\":{},\"next_offset\":{}}},\"cycle\":{},\"truncated\":{},\"unresolved\":{},\"disclaimer\":\"{disclaimer}\"}}",
        edge_string(&o.repo),o.snapshot.id,edge_string(&o.snapshot.nonce),edge_string(&o.revision),trace_anchor_json(&o.anchor),o.destination.as_ref().map(trace_anchor_json).unwrap_or_else(||"null".to_string()),edge_string(&o.freshness),edge_strings(&o.freshness_reasons),edge_string(&o.verification),edge_string(&o.completeness.status),o.completeness.covered,o.completeness.total,o.total_exact,edge_strings(&o.completeness.unresolved),o.nodes,o.edge_visits,o.depth,o.limit,o.offset,edge_number(o.next_offset),o.cycle,o.truncated,edge_strings(&o.unresolved))
}

fn cmd_semantic_import(
    name: &str,
    scip: &str,
    claim: Option<&str>,
    out: &mut String,
    err: &mut String,
) -> i32 {
    let home = paths::config_dir();
    let (root, _) = match resolve_recorded_root(name, err) {
        Ok(paths) => paths,
        Err(code) => return code,
    };
    match index::semantic_import(&home, name, Path::new(&root), Path::new(scip), claim) {
        Ok(o) => {
            let skipped: Vec<String> = o
                .skipped
                .iter()
                .map(|s| format!("\"{}\"", json::escape(s)))
                .collect();
            *out = format!(
                "{{\"repo\":\"{}\",\"snapshot\":{},\"import\":{},\"provider\":\"{}\",\"provider_version\":\"{}\",\"documents\":{},\"definitions\":{},\"references\":{},\"skipped\":[{}],\"verification\":\"{}\",\"fallback_reason\":\"none\"}}\n",
                json::escape(name),
                o.snapshot_id,
                o.import_id,
                json::escape(&o.provider_name),
                json::escape(&o.provider_version),
                o.documents,
                o.definitions,
                o.references,
                skipped.join(","),
                o.verification
            );
            0
        }
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            3
        }
    }
}

fn cmd_semantic_status(name: &str, out: &mut String, err: &mut String) -> i32 {
    let home = paths::config_dir();
    let (root, _) = match resolve_recorded_root(name, err) {
        Ok(paths) => paths,
        Err(code) => return code,
    };
    match index::semantic_status(&home, name, Path::new(&root)) {
        Ok((snapshot, rows)) => {
            let mut items = Vec::new();
            for row in &rows {
                items.push(format!(
                    "{{\"import\":{},\"provider\":\"{}\",\"provider_version\":\"{}\",\"capability\":\"{}\",\"verification\":\"{}\",\"artifact_id\":\"{}\",\"claimed_source_digest\":\"{}\",\"content_identity_match\":{},\"source_revision_match\":{},\"limitation\":\"{}\"}}",
                    row.import_id,
                    json::escape(&row.provider_name),
                    json::escape(&row.provider_version),
                    json::escape(&row.capability),
                    row.verification,
                    json::escape(&row.artifact_id),
                    json::escape(&row.claimed_source_digest),
                    row.content_match,
                    row.revision_match,
                    json::escape(&row.limitation)
                ));
            }
            *out = format!(
                "{{\"repo\":\"{}\",\"snapshot\":{},\"imports\":[{}]}}\n",
                json::escape(name),
                match &snapshot {
                    Some(s) => s.id.to_string(),
                    None => "null".to_string(),
                },
                items.join(",")
            );
            0
        }
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            3
        }
    }
}

fn semantic_hit_json(hit: &index::SemanticHit) -> String {
    format!(
        "{{\"symbol_uri\":{},\"display_name\":{},\"path\":\"{}\",\"span\":{{\"start_line\":{},\"start_col\":{},\"end_line\":{},\"end_col\":{},\"byte_start\":{},\"byte_end\":{}}},\"roles\":{},\"precision\":\"{}\",\"verification\":\"{}\"}}",
        match &hit.symbol_uri {
            Some(u) => format!("\"{}\"", json::escape(u)),
            None => "null".to_string(),
        },
        match &hit.display_name {
            Some(n) => format!("\"{}\"", json::escape(n)),
            None => "null".to_string(),
        },
        json::escape(&hit.path),
        hit.start_line,
        hit.start_col,
        hit.end_line,
        hit.end_col,
        hit.byte_start,
        hit.byte_end,
        hit.roles,
        hit.precision,
        hit.verification
    )
}

fn cmd_semantic_query(
    name: &str,
    query: &QueryArgs<'_>,
    out: &mut String,
    err: &mut String,
) -> i32 {
    let home = paths::config_dir();
    let (root, _) = match resolve_recorded_root(name, err) {
        Ok(paths) => paths,
        Err(code) => return code,
    };
    let line_number: u32 = match query.line.parse() {
        Ok(n) => n,
        Err(_) => {
            err.push_str(&format!("error: invalid --line value: '{}'\n", query.line));
            return 2;
        }
    };
    let col_number: u32 = match query.col.parse() {
        Ok(n) => n,
        Err(_) => {
            err.push_str(&format!("error: invalid --col value: '{}'\n", query.col));
            return 2;
        }
    };
    let span = index::SpanQuery {
        rel: query.path,
        line1: line_number,
        col1: col_number,
        refs: query.refs,
    };
    let answer = if let Some(exe) = query.lsp_server {
        index::semantic_query_lsp(Path::new(&root), &span, exe)
    } else {
        index::semantic_query(&home, name, Path::new(&root), &span)
    };
    match answer {
        Ok(o) => {
            let defs: Vec<String> = o.definitions.iter().map(semantic_hit_json).collect();
            let references: Vec<String> = o.references.iter().map(semantic_hit_json).collect();
            let diags: Vec<String> = o
                .server_diagnostics
                .iter()
                .map(|d| format!("\"{}\"", json::escape(d)))
                .collect();
            *out = format!(
                "{{\"repo\":\"{}\",\"definitions\":[{}],\"references\":[{}],\"provider\":\"{}\",\"provider_version\":\"{}\",\"verification\":\"{}\",\"server_diagnostics\":[{}],\"fallback\":{}}}\n",
                json::escape(name),
                defs.join(","),
                references.join(","),
                json::escape(&o.provider_name),
                json::escape(&o.provider_version),
                o.verification,
                diags.join(","),
                match &o.fallback {
                    Some(reason) => semantic::fallback_report(&o.provider_name, reason),
                    None => "null".to_string(),
                }
            );
            0
        }
        Err(e) => {
            err.push_str(&format!("error: {e}\n"));
            3
        }
    }
}

fn run(args: &[String], out: &mut String, err: &mut String) -> i32 {
    match args::parse(args) {
        Err(usage) => {
            *err = format!("error: unknown arguments\n{usage}");
            2
        }
        Ok(cmd) => dispatch(cmd, out, err),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut out = String::new();
    let mut err = String::new();
    let code = run(&args, &mut out, &mut err);
    if !out.is_empty() {
        print!("{out}");
    }
    if !err.is_empty() {
        eprint!("{err}");
    }
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exec(args: &[&str]) -> (i32, String, String) {
        let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let mut out = String::new();
        let mut err = String::new();
        let code = run(&owned, &mut out, &mut err);
        (code, out, err)
    }

    #[test]
    fn help_lists_status() {
        let (code, out, _) = exec(&["--help"]);
        assert_eq!(code, 0);
        assert!(out.contains("status"), "help must list status");
    }

    #[test]
    fn version_matches_package() {
        let (code, out, _) = exec(&["--version"]);
        assert_eq!(code, 0);
        assert_eq!(out, format!("{NAME} {VERSION}\n"));
    }

    #[test]
    fn status_json_is_deterministic() {
        let (code, out, _) = exec(&["status"]);
        assert_eq!(code, 0);
        assert_eq!(
            out,
            format!(
                "{{\"name\":\"{NAME}\",\"version\":\"{VERSION}\",\"scope\":\"{SCOPE}\",\"networking\":\"disabled\"}}\n"
            )
        );
    }

    #[test]
    fn unknown_args_exit_2() {
        let (code, _, err) = exec(&["index"]);
        assert_eq!(code, 2);
        assert!(err.contains("unknown arguments"));
    }
}
