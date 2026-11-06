# Roadmap

This roadmap tracks product work and non-goals. The owner-adopted [hybrid architecture roadmap](docs/hybrid-architecture-roadmap.md) is the master for architecture direction; its future stages remain planned until separately approved and implemented. Items below carry no dates: they move when evidence arrives, and the order inside a section is not a ranking. For what exists today, read the [README](README.md); for what has shipped, read the [changelog](CHANGELOG.md).

Riltra is pre-release, and its working rule is to claim only what the evidence supports. Most items below therefore end in something other people can check: a release, a published report, a fixed defect.

The published `0.1.0` assets received one Phase B installer check on hosted Windows, Linux and macOS Intel x64 runners on 2026-11-04; see the [dated M10 amendment](docs/m10-phase-b-amendment-2026-11-04.md) for scope and limits. The earlier M10 consolidation remains unchanged as a historical record.

## Now: after the first pre-release

- **Make the repository metadata consistent** with the README.
- **Watch someone else install it.** One person outside the project installs from the release and calls one tool; their friction becomes issues.

## Next: test one narrow claim

- **Evaluate on real Rust repositories** against a baseline of reading and searching only. Publish the method, raw data, scripts and every result, including null and negative ones, and a section on when not to use Riltra. Report per client, with host, client version, attempt count, date and limits beside each result, and without naming other products ([M10 consolidation, section 10](docs/m10-consolidation.md#10-public-reporting-boundary)).
- **Count whether agents call Riltra at all.** In the earlier agent pilot, no agent made a Riltra call in any run of the arm that had Riltra available ([M10 consolidation, section 4.1](docs/m10-consolidation.md#41-totals)), so a run with zero calls is inconclusive, not a pass.
- **Skip build and vendor directories by default**, with a documented override. Today `.riltraignore` matches exact paths and directory prefixes only, from the repository root ([guide](docs/guide/agent-setup.md#excluding-paths)).
- **Profile the fixed cost of re-indexing an unchanged repository**, then decide what to change.
- **Add tests for non-ASCII source**, where graph columns count bytes.
- **Extend supply-chain checks:** CI already audits the locked Rust dependency graph against RustSec and checks dependency-license metadata and release-document drift. Workflow actions pinned by commit SHA, automated dependency update proposals, and CI enforcement of commit sign-off remain deferred; see the [hybrid architecture roadmap](docs/hybrid-architecture-roadmap.md#existing-checks-and-deferred-supply-chain-work).
- **Document a reproducible way to produce a Rust SCIP index.** It stays opt-in: the producer runs the project's build code, so it is only for source you trust.

## Later: only with results and outside demand

- Real semantic producers for Python and TypeScript. Today the README lists SCIP import for Python and JavaScript as contract-only and for TypeScript as verified on a fixture ([Language support](README.md#language-support)).
- Build-graph evidence from Cargo metadata, to narrow which Rust files count as call candidates. This needs a decision on what "complete" means for an impact answer.
- Deeper extraction for Rust and JavaScript, for example calls inside Rust macro bodies ([What, Impact, Why: remaining work](docs/riltra-what-impact-why-implementation.md#remaining-work-in-priority-order)).
- Re-testing against newer MCP SDK and specification revisions, and whether declared output schemas help agents choose tools.
- C++ semantic resolution stays deferred; four capability criteria are documented as unmet ([Known limitations](README.md#known-limitations)).

## Not planned

- A network service: no HTTP server, daemon or file watcher.
- Embeddings or vector search for source code. The adopted architecture roadmap plans opt-in semantic retrieval for active engineering-knowledge records only; it is not implemented yet and does not change core or lexical-search behavior.
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
