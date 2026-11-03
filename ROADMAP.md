# Roadmap

This roadmap says what Riltra is working toward and what it has decided not to build. It carries no dates: items move when evidence arrives, and the order inside a section is not a ranking. For what exists today, read the [README](README.md); for what has shipped, read the [changelog](CHANGELOG.md).

Riltra is pre-release, and its working rule is to claim only what the evidence supports. Most items below therefore end in something other people can check: a release, a published report, a fixed defect.

## Now: the first pre-release

- **Publish `0.1.0` as a pre-release.** Native archives for Windows x64, Linux x64 and macOS Intel x64, each with a SHA-256 checksum and the third-party notices ([release/README.md](release/README.md)). No release has been published yet ([Project status](README.md#project-status)).
- **Check the installers against published assets.** The installers have been checked on hosted runners against locally built archives; the same check against a published release has not run ([M10 consolidation, section 6](docs/m10-consolidation.md#6-release-readiness)).
- **Make install URLs and repository metadata consistent.**
- **Harden parsing and error handling.** A very deeply nested source file must not exhaust the stack, and the MCP server's failure handling must match what its own comments promise in release builds.
- **Help agents use the tools.** The [agent setup guide](docs/guide/agent-setup.md) says when to call which tool and how to read its labels; the tool descriptions get the same treatment.
- **Give CI time limits and concurrency control**, so a stuck runner cannot hold a required check indefinitely.
- **Watch someone else install it.** One person outside the project installs from the release and calls one tool; their friction becomes issues.

## Next: test one narrow claim

- **Evaluate on real Rust repositories** against a baseline of reading and searching only. Publish the method, raw data, scripts and every result, including null and negative ones, and a section on when not to use Riltra. Report per client, with host, client version, attempt count, date and limits beside each result, and without naming other products ([M10 consolidation, section 10](docs/m10-consolidation.md#10-public-reporting-boundary)).
- **Count whether agents call Riltra at all.** In the earlier agent pilot, no agent made a Riltra call in any run of the arm that had Riltra available ([M10 consolidation, section 4.1](docs/m10-consolidation.md#41-totals)), so a run with zero calls is inconclusive, not a pass.
- **Skip build and vendor directories by default**, with a documented override. Today `.riltraignore` matches exact paths and directory prefixes only, from the repository root ([guide](docs/guide/agent-setup.md#excluding-paths)).
- **Profile the fixed cost of re-indexing an unchanged repository**, then decide what to change.
- **Add tests for non-ASCII source**, where graph columns count bytes.
- **Add supply-chain checks to CI:** dependency advisory and license checks, workflow actions pinned by commit, automated update proposals, and a sign-off check on commits.
- **Document a reproducible way to produce a Rust SCIP index.** It stays opt-in: the producer runs the project's build code, so it is only for source you trust.

## Later: only with results and outside demand

- Real semantic producers for Python and TypeScript. Today the README lists SCIP import for Python and JavaScript as contract-only and for TypeScript as verified on a fixture ([Language support](README.md#language-support)).
- Build-graph evidence from Cargo metadata, to narrow which Rust files count as call candidates. This needs a decision on what "complete" means for an impact answer.
- Deeper extraction for Rust and JavaScript, for example calls inside Rust macro bodies ([What, Impact, Why: remaining work](docs/riltra-what-impact-why-implementation.md#remaining-work-in-priority-order)).
- Re-testing against newer MCP SDK and specification revisions, and whether declared output schemas help agents choose tools.
- C++ semantic resolution stays deferred; four capability criteria are documented as unmet ([Known limitations](README.md#known-limitations)).

## Not planned

- A network service: no HTTP server, daemon or file watcher.
- Embeddings or vector search.
- MCP tools that write.
- A user interface or a graph database.
- Telemetry.
- Promoting name-based `POTENTIAL_CALL` candidates to proven calls, or replacing the `UNKNOWN` default of impact answers.
- Bundling indexers or language models, or writing a C++ resolver of our own.
- Splitting the crate, or adding languages for their own sake.
- Package-manager distribution, signing and macOS ARM64 archives, until separately decided (see below).

## Open decisions

- Whether to consolidate or rename MCP tools. This will be decided from evaluation data; the [stability notes](README.md#stability-pre-10) allow such a change in a minor version.
- Distribution beyond GitHub release archives.
- How complete the evidence must be before an impact answer can say anything other than `UNKNOWN`.
- Whether and when to lift the C++ semantic deferral.

This file changes when a release is cut or a decision is made. The changelog records what shipped.
