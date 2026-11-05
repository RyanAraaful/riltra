# Contributing to Riltra

All contributions are under the Apache License 2.0 (inbound = outbound).
By submitting, you agree your work is your own or you have the right to
submit it, and it contains **no** code copied from sources without clear
rights, including any noncommercial/restricted-licensed material.

## DCO sign-off (required)

Every commit MUST include a `Signed-off-by:` trailer:

```text
Signed-off-by: Your Name <you@example.com>
```

via `git commit -s`. This certifies the Developer Certificate of Origin
(v1.1): you wrote the change or have permission to submit it under
Apache-2.0.

## Rules

- SPDX header `SPDX-License-Identifier: Apache-2.0` on new source files.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
  must pass before pushing.
- No network access in core paths; provider integrations are explicit
  opt-in only.
- New dependencies need a `docs/licensing.md` entry (package, version,
  license, origin) in the same PR.
- No performance/accuracy claims vs competitors without a reproducible
  benchmark committed under `tests/` + methodology in docs.

## Pull request checklist

Use the [pull request checklist](.github/pull_request_template.md) when preparing
a change.

For a bug report or feature request, use the repository's issue forms. Report
security vulnerabilities through the private channel in [SECURITY.md](SECURITY.md),
not as a public issue.
