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
    Index {
        repo: String,
        full: bool,
    },
    IndexStatus {
        repo: String,
    },
    Search {
        repo: String,
        symbol: String,
        limit: Option<String>,
        offset: Option<String>,
    },
    Semantic {
        op: SemanticOp,
    },
}

#[derive(Debug, PartialEq)]
pub enum SemanticOp {
    Import {
        repo: String,
        scip: String,
        claim: Option<String>,
    },
    Status {
        repo: String,
    },
    Query {
        repo: String,
        path: String,
        line: String,
        col: String,
        refs: bool,
        lsp_server: Option<String>,
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
        "index" => {
            let (subcommand, mut i) = match args.get(1).map(String::as_str) {
                Some("status") => (true, 2),
                _ => (false, 1),
            };
            let mut repo = None;
            let mut full = false;
            while i < args.len() {
                if args[i] == "--full" && !subcommand {
                    full = true;
                    i += 1;
                } else if let Some(v) = take_value(args, &mut i, "--repo") {
                    repo = Some(v);
                } else {
                    return Err(crate::usage());
                }
            }
            match repo {
                Some(repo) if subcommand => Ok(Command::IndexStatus { repo }),
                Some(repo) => Ok(Command::Index { repo, full }),
                None => Err(crate::usage()),
            }
        }
        "search" => {
            let mut repo = None;
            let mut symbol = None;
            let mut limit = None;
            let mut offset = None;
            let mut i = 1;
            while i < args.len() {
                if let Some(v) = take_value(args, &mut i, "--repo") {
                    repo = Some(v);
                } else if let Some(v) = take_value(args, &mut i, "--symbol") {
                    symbol = Some(v);
                } else if let Some(v) = take_value(args, &mut i, "--limit") {
                    limit = Some(v);
                } else if let Some(v) = take_value(args, &mut i, "--offset") {
                    offset = Some(v);
                } else {
                    return Err(crate::usage());
                }
            }
            match (repo, symbol) {
                (Some(repo), Some(symbol)) => Ok(Command::Search {
                    repo,
                    symbol,
                    limit,
                    offset,
                }),
                _ => Err(crate::usage()),
            }
        }
        "semantic" => match args.get(1).map(String::as_str) {
            Some("import") => {
                let mut repo = None;
                let mut scip = None;
                let mut claim = None;
                let mut i = 2;
                while i < args.len() {
                    if let Some(v) = take_value(args, &mut i, "--repo") {
                        repo = Some(v);
                    } else if let Some(v) = take_value(args, &mut i, "--scip") {
                        scip = Some(v);
                    } else if let Some(v) = take_value(args, &mut i, "--source-digest") {
                        claim = Some(v);
                    } else {
                        return Err(crate::usage());
                    }
                }
                match (repo, scip) {
                    (Some(repo), Some(scip)) => Ok(Command::Semantic {
                        op: SemanticOp::Import { repo, scip, claim },
                    }),
                    _ => Err(crate::usage()),
                }
            }
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
                    Some(repo) => Ok(Command::Semantic {
                        op: SemanticOp::Status { repo },
                    }),
                    None => Err(crate::usage()),
                }
            }
            Some("query") => {
                let mut repo = None;
                let mut path = None;
                let mut line = None;
                let mut col = None;
                let mut refs = false;
                let mut lsp_server = None;
                let mut i = 2;
                while i < args.len() {
                    if let Some(v) = take_value(args, &mut i, "--repo") {
                        repo = Some(v);
                    } else if let Some(v) = take_value(args, &mut i, "--path") {
                        path = Some(v);
                    } else if let Some(v) = take_value(args, &mut i, "--line") {
                        line = Some(v);
                    } else if let Some(v) = take_value(args, &mut i, "--col") {
                        col = Some(v);
                    } else if args[i] == "--refs" {
                        refs = true;
                        i += 1;
                    } else if let Some(v) = take_value(args, &mut i, "--lsp-server") {
                        lsp_server = Some(v);
                    } else {
                        return Err(crate::usage());
                    }
                }
                match (repo, path, line, col) {
                    (Some(repo), Some(path), Some(line), Some(col)) => Ok(Command::Semantic {
                        op: SemanticOp::Query {
                            repo,
                            path,
                            line,
                            col,
                            refs,
                            lsp_server,
                        },
                    }),
                    _ => Err(crate::usage()),
                }
            }
            _ => Err(crate::usage()),
        },
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
    use super::{parse, Command, SemanticOp};

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
    fn parses_semantic_commands() {
        assert!(matches!(
            p(&["semantic", "import", "--repo", "r", "--scip", "i.scip"]),
            Ok(Command::Semantic {
                op: SemanticOp::Import { claim: None, .. }
            })
        ));
        assert!(matches!(
            p(&[
                "semantic",
                "import",
                "--repo",
                "r",
                "--scip",
                "i.scip",
                "--source-digest",
                "ab"
            ]),
            Ok(Command::Semantic {
                op: SemanticOp::Import { claim: Some(_), .. }
            })
        ));
        assert!(matches!(
            p(&["semantic", "status", "--repo", "r"]),
            Ok(Command::Semantic {
                op: SemanticOp::Status { .. }
            })
        ));
        assert!(matches!(
            p(
                &[
                    "semantic", "query", "--repo", "r", "--path", "a.py", "--line", "3", "--col",
                    "5"
                ]
            ),
            Ok(Command::Semantic {
                op: SemanticOp::Query {
                    refs: false,
                    lsp_server: None,
                    ..
                }
            })
        ));
        assert!(matches!(
            p(&[
                "semantic", "query", "--repo", "r", "--path", "a.py", "--line", "3", "--col", "5",
                "--refs"
            ]),
            Ok(Command::Semantic {
                op: SemanticOp::Query { refs: true, .. }
            })
        ));
        // M6 boundary: no traversal flags exist; unknown flags are usage errors.
        assert!(p(&[
            "semantic",
            "query",
            "--repo",
            "r",
            "--path",
            "a.py",
            "--line",
            "3",
            "--col",
            "5",
            "--refs",
            "--refs-depth",
            "2"
        ])
        .is_err());
        assert!(p(&["semantic", "import", "--repo", "r"]).is_err());
        assert!(p(&["semantic", "query", "--repo", "r", "--path", "a.py"]).is_err());
        assert!(p(&["semantic", "frobnicate", "--repo", "r"]).is_err());
        assert!(p(&["semantic"]).is_err());
    }

    #[test]
    fn parses_index_full_flag() {
        assert!(matches!(
            p(&["index", "--repo", "r", "--full"]),
            Ok(Command::Index { repo, full: true }) if repo == "r"
        ));
        assert!(matches!(
            p(&["index", "--repo", "r"]),
            Ok(Command::Index { repo, full: false }) if repo == "r"
        ));
        assert!(p(&["index", "--repo", "r", "--full=x"]).is_err());
        assert!(p(&["index", "--repo", "r", "bogus"]).is_err());
    }

    #[test]
    fn parses_index_and_search() {
        assert!(matches!(
            p(&["index", "--repo", "r"]),
            Ok(Command::Index { .. })
        ));
        assert!(matches!(
            p(&["index", "status", "--repo", "r"]),
            Ok(Command::IndexStatus { .. })
        ));
        assert!(p(&["index"]).is_err());
        assert!(p(&["index", "status"]).is_err());
        assert!(p(&["index", "bogus", "--repo", "r"]).is_err());
        assert!(matches!(
            p(&["search", "--repo", "r", "--symbol", "f"]),
            Ok(Command::Search {
                limit: None,
                offset: None,
                ..
            })
        ));
        assert!(matches!(
            p(&[
                "search",
                "--repo",
                "r",
                "--symbol",
                "f",
                "--limit",
                "10",
                "--offset=5"
            ]),
            Ok(Command::Search { .. })
        ));
        assert!(p(&["search", "--repo", "r"]).is_err());
        assert!(p(&["search", "--symbol", "f"]).is_err());
    }

    #[test]
    fn rejects_unknown_commands() {
        assert!(p(&["index"]).is_err());
        assert!(p(&["repo", "list", "extra"]).is_err());
        assert!(p(&["--bogus"]).is_err());
    }
}
