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
mod json;
mod paths;
mod registry;

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
        "Usage:\n  {NAME} [--help] [--version] status\n  {NAME} init --repo <path> [--name <name>]\n  {NAME} repo list\n  {NAME} repo status --repo <name>\n  {NAME} doctor [--repo <name>]\n\nCommands:\n  status       Print deterministic bootstrap status as JSON\n  init         Register a local repository root\n  repo list    List registered repositories as JSON\n  repo status  Git/worktree fingerprint of a registered repository\n  doctor       Path-security and repository health checks as JSON\n"
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
