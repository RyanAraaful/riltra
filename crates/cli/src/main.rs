// SPDX-License-Identifier: Apache-2.0
// M0 placeholder: std-only CLI, no networking, no file access.
// Real commands (index/search/graph/MCP/knowledge) land in M1+.

const NAME: &str = "riltra";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const SCOPE: &str = "m0-bootstrap";

fn usage() -> String {
    format!(
        "Usage:\n  {NAME} [--help] [--version] status\n\nCommands:\n  status    Print deterministic bootstrap status as JSON\n"
    )
}

fn version_line() -> String {
    format!("{NAME} {VERSION}")
}

// Single-line JSON with fixed key order so `--format json` output is stable.
fn status_json() -> String {
    format!(
        "{{\"name\":\"{NAME}\",\"version\":\"{VERSION}\",\"scope\":\"{SCOPE}\",\"networking\":\"disabled\"}}"
    )
}

// Returns process exit code; `out` goes to stdout, `err` to stderr.
fn run(args: &[String], out: &mut String, err: &mut String) -> i32 {
    if args.is_empty() {
        *out = usage();
        return 0;
    }
    match args[0].as_str() {
        "-h" | "--help" | "help" => {
            *out = usage();
            0
        }
        "-V" | "--version" | "version" => {
            *out = version_line() + "\n";
            0
        }
        "status" if args.len() == 1 => {
            *out = status_json() + "\n";
            0
        }
        _ => {
            *err = format!("error: unknown arguments\n{}", usage());
            2
        }
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
