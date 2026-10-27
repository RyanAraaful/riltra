# Installing Riltra

Riltra ships as one native executable. There is no service and no background
process, and the binary makes no network call. The installer runs at user level:
no administrative rights, no PATH change.

## Install from a release

Windows PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

Linux or macOS:

```sh
sh install.sh
```

The scripts download the archive for this platform and its `.sha256` file from
the GitHub release, verify the checksum, and then install. To use a local
archive instead, pass it explicitly:

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1 -ArchivePath .\riltra-0.1.0-windows-x64.zip
```

```sh
sh install.sh --archive ./riltra-0.1.0-linux-x64.tar.gz
```

The archive and its `.sha256` must sit together. A missing, malformed, or
mismatched checksum stops the install before anything is extracted. Unsupported
platforms and ARM64 are refused with a non-zero exit.

## Install locations

| Platform | Command | Payload and documents |
|---|---|---|
| Windows x64 | `%LOCALAPPDATA%\Programs\Riltra\bin\riltra.exe` | Same directory |
| Linux x64 | `$HOME/.local/bin/riltra` (symlink) | `$HOME/.local/share/riltra/` |
| macOS Intel x64 | `$HOME/.local/bin/riltra` (symlink) | `$HOME/.local/share/riltra/` |

The installer prints the directory to add to `PATH`; it does not edit a shell
profile, the registry, or a system-wide path. On Linux and macOS the shared
`~/.local/bin` directory receives only the `riltra` symlink; the notices stay in
the private payload directory.

## Reinstall and removal

Run the same installer again. It replaces only files recorded in the Riltra
manifest and refuses to overwrite an unrelated `riltra` file or symlink.

Removal is manual:

- Windows: delete `%LOCALAPPDATA%\Programs\Riltra`.
- Linux and macOS: delete the `riltra` symlink in `~/.local/bin` (only after
  checking that it points into `~/.local/share/riltra`) and delete
  `~/.local/share/riltra`.

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
