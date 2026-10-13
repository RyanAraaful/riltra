// SPDX-License-Identifier: Apache-2.0
//! CLI argument parsing into typed commands. No external CLI library (M1 is
//! std-only); both `--flag value` and `--flag=value` forms are accepted.

#[derive(Debug, PartialEq)]
pub enum Command {
    Help,
    Version,
    Status,
    Init {
        repo: String,
        name: Option<String>,
    },
    RepoList,
    RepoStatus {
        repo: String,
    },
    Doctor {
        repo: Option<String>,
    },
    Scan {
        repo: String,
        paths: Vec<String>,
        symbol: Option<String>,
    },
}

/// Parse full argv (including argv[0]-style first token). `Err` is the usage
/// text; callers map it to exit code 2.
pub fn parse(args: &[String]) -> Result<Command, String> {
    if args.is_empty() {
        return Ok(Command::Help);
    }
    match args[0].as_str() {
        "-h" | "--help" | "help" if args.len() == 1 => Ok(Command::Help),
        "-V" | "--version" | "version" if args.len() == 1 => Ok(Command::Version),
        "status" if args.len() == 1 => Ok(Command::Status),
        "init" => {
            let mut repo = None;
            let mut name = None;
            let mut i = 1;
            while i < args.len() {
                if let Some(v) = take_value(args, &mut i, "--repo") {
                    repo = Some(v);
                } else if let Some(v) = take_value(args, &mut i, "--name") {
                    name = Some(v);
                } else {
                    return Err(crate::usage());
                }
            }
            match repo {
                Some(repo) => Ok(Command::Init { repo, name }),
                None => Err(crate::usage()),
            }
        }
        "repo" => match args.get(1).map(String::as_str) {
            Some("list") if args.len() == 2 => Ok(Command::RepoList),
            Some("status") => {
                let mut repo = None;
                let mut i = 2;
                while i < args.len() {
                    if let Some(v) = take_value(args, &mut i, "--repo") {
                        repo = Some(v);
                    } else {
                        return Err(crate::usage());
                    }
                }
                match repo {
                    Some(repo) => Ok(Command::RepoStatus { repo }),
                    None => Err(crate::usage()),
                }
            }
            _ => Err(crate::usage()),
        },
        "doctor" => {
            let mut repo = None;
            let mut i = 1;
            while i < args.len() {
                if let Some(v) = take_value(args, &mut i, "--repo") {
                    repo = Some(v);
                } else {
                    return Err(crate::usage());
                }
            }
            Ok(Command::Doctor { repo })
        }
        "scan" => {
            let mut repo = None;
            let mut paths = Vec::new();
            let mut symbol = None;
            let mut i = 1;
            while i < args.len() {
                if let Some(v) = take_value(args, &mut i, "--repo") {
                    repo = Some(v);
                } else if let Some(v) = take_value(args, &mut i, "--path") {
                    paths.push(v);
                } else if let Some(v) = take_value(args, &mut i, "--symbol") {
                    symbol = Some(v);
                } else {
                    return Err(crate::usage());
                }
            }
            match repo {
                Some(repo) => Ok(Command::Scan {
                    repo,
                    paths,
                    symbol,
                }),
                None => Err(crate::usage()),
            }
        }
        _ => Err(crate::usage()),
    }
}

/// Consume `--flag value` or `--flag=value` at `args[*i]`; `None` when the
/// token is not that flag (index untouched) or the value is missing.
fn take_value(args: &[String], i: &mut usize, flag: &str) -> Option<String> {
    let token = args.get(*i)?;
    if let Some(rest) = token.strip_prefix(flag) {
        if let Some(value) = rest.strip_prefix('=') {
            *i += 1;
            return Some(value.to_string());
        }
        if rest.is_empty() {
            *i += 1;
            let value = args.get(*i)?.clone();
            *i += 1;
            return Some(value);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{parse, Command};

    fn p(args: &[&str]) -> Result<Command, String> {
        parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn parses_init_with_name_forms() {
        assert!(matches!(
            p(&["init", "--repo", "D:/r", "--name", "r"]),
            Ok(Command::Init { .. })
        ));
        assert!(matches!(
            p(&["init", "--repo=D:/r"]),
            Ok(Command::Init { repo, name: None }) if repo == "D:/r"
        ));
    }

    #[test]
    fn rejects_init_without_repo() {
        assert!(p(&["init"]).is_err());
        assert!(p(&["init", "--repo"]).is_err());
        assert!(p(&["init", "--name", "x"]).is_err());
    }

    #[test]
    fn parses_repo_subcommands() {
        assert!(matches!(p(&["repo", "list"]), Ok(Command::RepoList)));
        assert!(matches!(
            p(&["repo", "status", "--repo", "r"]),
            Ok(Command::RepoStatus { .. })
        ));
        assert!(p(&["repo"]).is_err());
        assert!(p(&["repo", "status"]).is_err());
        assert!(p(&["repo", "drop", "--repo", "r"]).is_err());
    }

    #[test]
    fn parses_doctor_with_optional_repo() {
        assert!(matches!(p(&["doctor"]), Ok(Command::Doctor { repo: None })));
        assert!(matches!(
            p(&["doctor", "--repo", "r"]),
            Ok(Command::Doctor { repo: Some(_) })
        ));
        assert!(p(&["doctor", "--repo"]).is_err());
    }

    #[test]
    fn parses_scan_with_paths_and_symbol() {
        assert!(matches!(
            p(&["scan", "--repo", "r"]),
            Ok(Command::Scan { paths, symbol: None, .. }) if paths.is_empty()
        ));
        assert!(matches!(
            p(&[
                "scan",
                "--repo",
                "r",
                "--path",
                "a.py",
                "--path=b.py",
                "--symbol",
                "f"
            ]),
            Ok(Command::Scan { .. })
        ));
        assert!(p(&["scan"]).is_err());
        assert!(p(&["scan", "--path", "a.py"]).is_err());
    }

    #[test]
    fn rejects_unknown_commands() {
        assert!(p(&["index"]).is_err());
        assert!(p(&["repo", "list", "extra"]).is_err());
        assert!(p(&["--bogus"]).is_err());
    }
}
