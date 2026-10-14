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
mod git;
mod ignore;
mod index;
mod json;
mod paths;
mod registry;
mod scan;
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
        "Usage:\n  {NAME} [--help] [--version] status\n  {NAME} init --repo <path> [--name <name>]\n  {NAME} repo list\n  {NAME} repo status --repo <name>\n  {NAME} doctor [--repo <name>]\n  {NAME} scan --repo <name> [--path <relpath>]... [--symbol <substr>]\n  {NAME} index --repo <name> [--full]\n  {NAME} index status --repo <name>\n  {NAME} search --repo <name> --symbol <substr> [--limit <n>] [--offset <n>]\n\nCommands:\n  status       Print deterministic bootstrap status as JSON\n  init         Register a local repository root\n  repo list    List registered repositories as JSON\n  repo status  Git/worktree fingerprint of a registered repository\n  doctor       Path-security and repository health checks as JSON\n  scan         Syntax facts (definitions/imports/calls) as JSON\n  index        Build the durable code index for a repository\n  index status Report index freshness (fresh/stale/unknown) as JSON\n  search       Search indexed symbols as JSON\n"
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
