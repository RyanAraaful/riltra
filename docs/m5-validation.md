# M5 validation (working draft)

## Dependency audit (Task 1, 2026-10-16)

| Crate | Version | License (manifest) | Role |
|---|---|---|---|
| serde_json | 1.0.154 | MIT OR Apache-2.0 | LSP JSON-RPC parse/emit |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | transitive via serde_json |
| protobuf | 3.7.2 | MIT | SCIP wire reader + generated bindings |
| protobuf-support | 3.7.2 | MIT | transitive via protobuf |

Rejected: prost/prost-build (needs protoc at build time; CI 3-OS must not
download toolchains). protobuf-codegen is a developer-side tool only
(bindings checked in), never a build dependency. No async/network/loader
crates in the tree (pinned by `new_dependencies_are_audited`).

## Tier A / Tier B

Pending implementation tasks 2–10.
