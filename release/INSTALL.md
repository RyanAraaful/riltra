# Installing a Riltra release-candidate archive

Riltra ships as one native executable. There is no installer, no service and
no background process, and the binary makes no network call.

1. Extract the archive anywhere. Check it first against the `.sha256` file that
   comes with it (`sha256sum -c`, `shasum -a 256 -c`, or `Get-FileHash` on
   Windows).
2. Put the directory on your `PATH`, or call the binary by its full path.
3. Run `riltra --version`, `riltra --help` and `riltra status`.

## First use

```text
riltra init --repo <path-to-a-repository> --name myrepo
riltra index --repo myrepo
riltra search --repo myrepo --symbol <name>
riltra search --repo myrepo --symbol <field-name> --occurrences
riltra callers --repo myrepo --path <file> --line <n> --col <n>
```

Riltra keeps its registry and its databases under `RILTRA_HOME` (by default
`%APPDATA%\riltra` on Windows and `~/.config/riltra` elsewhere), never inside
the repository it indexes. Point `RILTRA_HOME` at a scratch directory to try it
without touching your own registry.

## Using it from an MCP client such as OpenCode

`riltra mcp` serves twelve tools over stdio (eleven read-only ones and the
provider-backed `riltra_code_semantic`, which stays off until an operator sets
`RILTRA_LSP_SERVER` to a language server's absolute path). An OpenCode
configuration entry looks like:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "servers": {
      "riltra": {
        "type": "local",
        "command": ["<absolute path to riltra>", "mcp"],
        "environment": { "RILTRA_HOME": "<absolute path to a Riltra home>" }
      }
    }
  }
}
```

## What this build is, and is not

A pre-release candidate for local, evidence-first code intelligence. Answers
are source-grounded and labelled: syntax-only relations are
`POTENTIAL_CALL` candidates, never proven calls, and an impact answer is
`UNKNOWN` unless a semantic provider backs it. C++ semantic resolution and
CUDA are not provided. The archive carries `NOTICE`, `LICENSE`,
`THIRD_PARTY_LICENSES.txt` and `sbom.cdx.json` for the bundled dependencies.
