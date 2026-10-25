# Release candidate packaging

How a Riltra release candidate is built, checked and verified. Nothing here
publishes a release, creates a tag or signs anything: the `v0.1.0` release is a
separate owner decision (design gate decision D-M9-6: native archives,
checksums and notices only).

## What the workflow does

`.github/workflows/release-candidate.yml` runs by hand (`workflow_dispatch`) or
for a `v*-rc*` tag, on Windows, Linux and macOS x64, and for each of them:

1. builds the release-profile binary with `cargo build --release --locked`;
2. checks the release documents against the locked dependency graph
   (`cargo run --locked --example release_docs -- --check`);
3. packages `riltra-<version>-<platform>.zip` (Windows) or `.tar.gz`, with a
   `.sha256` file (`release/package.sh`);
4. unpacks the archive in a scratch directory and runs the clean-install smoke
   on the shipped binary alone (`release/smoke.sh`);
5. keeps the archive and its checksum as a workflow artifact.

The macOS runner is the x64 pool (see the comment in `ci.yml`); macOS ARM64 is
not covered.

## Reproduce locally

```bash
cargo build --release --locked -p riltra
bash release/package.sh linux-x64 tar.gz        # or: windows-x64 zip, macos-x64 tar.gz
bash release/smoke.sh dist/riltra-*-linux-x64.tar.gz
sha256sum -c dist/riltra-*.sha256                # shasum -a 256 -c on macOS
```

## The generated documents

`sbom.cdx.json` is a CycloneDX 1.5 document of the locked dependency graph
(every package of `Cargo.lock`, for every platform, with its license
expression, its crate checksum and its dependency edges). The committed copy is
`release/sbom.cdx.json`; `--check` fails when it no longer matches `Cargo.lock`
or when a package has no license or no checksum. After a dependency change:

```bash
cargo run --locked --example release_docs -- --out release   # rewrites the SBOM
```

Do not commit `release/THIRD_PARTY_LICENSES.txt`: it is regenerated into every
archive from the license and notice files that each published crate carries,
and a package whose crate carries none is named in the tool's output.

The licenses themselves are audited by the `new_dependencies_are_audited` test
(allowlist: MIT, Apache-2.0, BSD-3-Clause, ISC, Zlib, Unlicense, Unicode-3.0,
Unicode-DFS-2016) and recorded in `docs/licensing.md`.

## Gates

The acceptance tests of G-LIC, G-NAT, G-CLI, G-MCP and G-SEC that run on a
candidate, and their evidence, are recorded in `docs/m9-cp4-evidence.md`.
