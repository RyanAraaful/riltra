// SPDX-License-Identifier: Apache-2.0
//! M5 semantic evidence contract: the single shared shape every provider
//! adapter produces, one path+range validator, one provenance/diagnostics
//! shape, and the static capability registry. No provider-specific logic
//! lives anywhere else.

use std::path::Path;

use crate::json::escape;
use crate::paths;

/// Precision of one evidence item: compiler-backed exact or heuristic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Precision {
    // Kept for contract stability; never constructed in M5 (see verify_live).
    #[allow(dead_code)]
    Exact,
    Heuristic,
}

impl Precision {
    pub fn as_str(self) -> &'static str {
        match self {
            Precision::Exact => "Exact",
            Precision::Heuristic => "Heuristic",
        }
    }
}

/// Verification state. `Current` exists for contract stability but is
/// unreachable in M5: no independent producer-to-source proof mechanism
/// exists, so imports land `Unverified` and drift flips them to `Stale`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verification {
    // Kept for contract stability; unreachable in M5 (see verify_live).
    #[allow(dead_code)]
    Current,
    Unverified,
    Stale,
}

impl Verification {
    pub fn as_str(self) -> &'static str {
        match self {
            Verification::Current => "Current",
            Verification::Unverified => "Unverified",
            Verification::Stale => "Stale",
        }
    }
}

/// Byte + line/col span. Byte offsets are authoritative for slicing;
/// line/col are 0-based display coordinates (SCIP convention).
#[derive(Debug, Clone, PartialEq)]
pub struct SpanBytes {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub byte_start: usize,
    pub byte_end: usize,
}

/// Location evidence: the mandatory half of every fact.
#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    pub path: String,
    pub digest: String,
    pub span: SpanBytes,
}

/// Optional provider symbol identity. `Some` for SCIP URIs, `None` for LSP
/// results — no SCIP-style identity is ever required of or fabricated for
/// LSP output.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderSymbol {
    pub uri: String,
    pub display_name: Option<String>,
}

/// Occurrence roles decoded from the full provider bitset. `raw` preserves
/// every bit (SCIP `0x1`..`0x40`); the booleans are the decoded view.
/// Reference predicate: `!definition`.
#[derive(Debug, Clone, PartialEq)]
pub struct Roles {
    pub raw: u32,
    pub definition: bool,
    pub reference: bool,
    pub import: bool,
    pub read: bool,
    pub write: bool,
    pub generated: bool,
    pub test: bool,
    pub forward_declaration: bool,
}

impl Roles {
    /// Decode a SCIP `SymbolRole` bitset. Bit values pinned by `scip.proto`:
    /// Definition `0x1`, Import `0x2`, WriteAccess `0x4`, ReadAccess `0x8`,
    /// Generated `0x10`, Test `0x20`, ForwardDefinition `0x40`.
    pub fn from_scip_bits(raw: u32) -> Roles {
        let definition = raw & 0x1 != 0;
        Roles {
            raw,
            definition,
            reference: !definition,
            import: raw & 0x2 != 0,
            read: raw & 0x8 != 0,
            write: raw & 0x4 != 0,
            generated: raw & 0x10 != 0,
            test: raw & 0x20 != 0,
            forward_declaration: raw & 0x40 != 0,
        }
    }

    /// LSP answers carry no role bitset: the request kind is the role.
    pub fn from_lsp_kind(definition: bool) -> Roles {
        Roles {
            raw: u32::from(definition),
            definition,
            reference: !definition,
            import: false,
            read: false,
            write: false,
            generated: false,
            test: false,
            forward_declaration: false,
        }
    }
}

/// Verbatim provider relationship. Stored, never traversed (M6 owns that).
#[derive(Debug, Clone, PartialEq)]
pub struct Relationship {
    pub target: String,
    pub is_reference: bool,
    pub is_implementation: bool,
    pub is_type_definition: bool,
    pub is_definition: bool,
}

/// Metadata for facts a producer references but defines externally
/// (SCIP `external_symbols` package triple). Marks, never locates.
#[derive(Debug, Clone, PartialEq)]
pub struct ExternalMeta {
    pub package_manager: String,
    pub package: String,
    pub version: String,
}

/// Provider identity: name + version, always recorded with the evidence.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderId {
    pub name: String,
    pub version: String,
}

/// One semantic evidence item: location (required) + optional identity.
#[derive(Debug, Clone, PartialEq)]
pub struct Evidence {
    pub location: Location,
    pub symbol: Option<ProviderSymbol>,
    pub roles: Roles,
    pub relationships: Vec<Relationship>,
    pub external: Option<ExternalMeta>,
    pub precision: Precision,
    pub verification: Verification,
    pub provider: ProviderId,
}

impl Evidence {
    /// Constructor for LSP answers: location only, no symbol identity.
    pub fn lsp_location(path: &str, digest: &str, span: SpanBytes) -> Evidence {
        Evidence {
            location: Location {
                path: path.to_string(),
                digest: digest.to_string(),
                span,
            },
            symbol: None,
            roles: Roles::from_lsp_kind(false),
            relationships: Vec::new(),
            external: None,
            // M5 honesty rule: no provider output is Exact without
            // producer attestation (deferred with `Current`).
            precision: Precision::Heuristic,
            verification: Verification::Unverified,
            provider: ProviderId {
                name: "lsp-subprocess".to_string(),
                version: String::new(),
            },
        }
    }
}

/// Actual capabilities of one provider. Static table, not configuration:
/// capabilities describe what the adapter implements, not wishes.
/// Fields are read by the registry tests; dispatch consults `resolve`.
#[allow(dead_code)]
pub struct Capability {
    pub name: &'static str,
    pub definitions: bool,
    pub references: bool,
}

/// The whole M5 provider universe. Unknown names are unsupported, never
/// guessed.
pub fn capabilities() -> &'static [Capability] {
    &[
        Capability {
            name: "scip-import",
            definitions: true,
            references: true,
        },
        Capability {
            name: "lsp-subprocess",
            definitions: true,
            references: true,
        },
    ]
}

/// Look up a provider by name. Unknown → explicit unsupported error.
pub fn resolve(provider: &str) -> Result<&'static Capability, String> {
    capabilities()
        .iter()
        .find(|c| c.name == provider)
        .ok_or_else(|| format!("unsupported semantic provider: '{provider}'"))
}

/// Single shared document-path validator for both providers. Enforces the
/// SCIP `relative_path` rules lexically (relative, `/` separators, no
/// empty/`.`/`..` components) then proves containment + existence through
/// `paths::contains` (canonicalizes, so symlink escapes fail). Returns the
/// normalized `/`-separated relative path.
pub fn validate_doc_path(root: &Path, rel: &str) -> Result<String, String> {
    if rel.is_empty() {
        return Err("empty document path".to_string());
    }
    if rel.starts_with('/') || rel.contains('\\') || rel.contains('\0') {
        return Err(format!("document path not relative/portable: '{rel}'"));
    }
    let mut parts = Vec::new();
    for part in rel.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return Err(format!("non-canonical document path: '{rel}'"));
        }
        parts.push(part);
    }
    let mut candidate = root.to_path_buf();
    for part in &parts {
        candidate.push(part);
    }
    let canonical_root = root
        .canonicalize()
        .map_err(|e| format!("repository root unavailable: {e}"))?;
    if !paths::contains(&canonical_root, &candidate) {
        return Err(format!("document path escapes repository: '{rel}'"));
    }
    Ok(parts.join("/"))
}

/// Single shared range checker: ordered half-open ranges only. Empty
/// (start==end) is allowed — SCIP permits it. Base/encoding checks belong
/// to the provider converters, not here.
pub fn validate_range(
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
) -> Result<(), String> {
    if (start_line, start_col) > (end_line, end_col) {
        return Err(format!(
            "disordered range: ({start_line},{start_col})-({end_line},{end_col})"
        ));
    }
    Ok(())
}

/// The single diagnostics shape for every unavailable/timeout/error/
/// unverifiable path. Syntax-only results stay usable alongside it.
pub fn fallback_report(provider: &str, reason: &str) -> String {
    format!(
        "{{\"provider\":\"{}\",\"status\":\"fallback\",\"reason\":\"{}\",\"syntax_fallback\":true}}",
        escape(provider),
        escape(reason)
    )
}

/// Live source state for `verify_live`. Nonce proves rebuild identity;
/// `content_identity` proves the bytes; git/config fields prove the rest.
/// Compared independently: equal fingerprints never excuse different bytes.
pub struct LiveState {
    pub head: String,
    pub branch: String,
    pub dirty: bool,
    pub untracked: u32,
    pub config_fingerprint: String,
    pub snapshot_nonce: String,
    pub content_identity: String,
}

/// Canonical revision string. One builder used at import binding AND query
/// time: the comparison is exact, never field-by-field drift.
pub fn revision_string(live: &LiveState) -> String {
    format!(
        "head={}|branch={}|dirty={}|untracked={}|cfg={}|nonce={}",
        live.head,
        live.branch,
        live.dirty,
        live.untracked,
        live.config_fingerprint,
        live.snapshot_nonce
    )
}

/// The ONE freshness rule for semantic facts (M5). Import-granular and
/// conservative: an unchanged file digest never implies the relationships
/// are still valid, so ANY drift in content or revision marks the whole
/// import `Stale` (still served with explicit marking, never as `Current`).
/// `Current` is unreachable in M5: no independent producer-to-source proof
/// exists, so a fully matching import is `Unverified`, and `--source-digest`
/// claims are recorded, never trusted.
pub fn verify_live(import: &crate::store::SemanticImport, live: &LiveState) -> Verification {
    if import.content_identity != live.content_identity {
        return Verification::Stale;
    }
    if import.source_revision != revision_string(live) {
        return Verification::Stale;
    }
    Verification::Unverified
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::tmpdir;

    #[test]
    fn doc_path_rejects_traversal_and_absolutes() {
        let dir = tmpdir("sem-path");
        std::fs::write(dir.join("ok.py"), "x").unwrap();
        assert!(validate_doc_path(&dir, "ok.py").is_ok());
        for bad in [
            "",
            "/abs.py",
            "../evil.py",
            "a/../../evil.py",
            "a//b.py",
            "a/./b.py",
            "a\\b.py",
            "sub/",
        ] {
            assert!(validate_doc_path(&dir, bad).is_err(), "must reject {bad:?}");
        }
        // Lexically clean but missing on disk: no bytes to digest → reject.
        assert!(validate_doc_path(&dir, "missing.py").is_err());
    }

    #[test]
    fn doc_path_rejects_symlink_escape() {
        let dir = tmpdir("sem-link");
        let outside = tmpdir("sem-link-out");
        std::fs::write(outside.join("secret.py"), "x").unwrap();
        let link = dir.join("escape.py");
        #[cfg(unix)]
        let made = std::os::unix::fs::symlink(outside.join("secret.py"), &link).is_ok();
        #[cfg(windows)]
        let made = std::os::windows::fs::symlink_file(outside.join("secret.py"), &link).is_ok();
        if !made {
            return; // OS refused symlinks here; nothing to prove.
        }
        assert!(validate_doc_path(&dir, "escape.py").is_err());
    }

    #[test]
    fn range_checker_rejects_disorder_and_zero_width_rules() {
        // end before start: reject. Empty (start==end): allowed (SCIP permits).
        assert!(validate_range(3, 5, 3, 2).is_err());
        assert!(validate_range(4, 0, 3, 9).is_err());
        assert!(validate_range(3, 5, 3, 5).is_ok());
        assert!(validate_range(1, 0, 9, 0).is_ok());
    }

    #[test]
    fn fallback_report_has_exact_keys() {
        let json = fallback_report("lsp-subprocess", "timeout");
        assert!(json.contains("\"provider\":\"lsp-subprocess\""));
        assert!(json.contains("\"status\":\"fallback\""));
        assert!(json.contains("\"reason\":\"timeout\""));
        assert!(json.contains("\"syntax_fallback\":true"));
    }

    #[test]
    fn capability_registry_knows_both_providers_and_rejects_unknown() {
        let scip = resolve("scip-import").unwrap();
        assert!(scip.definitions && scip.references);
        let lsp = resolve("lsp-subprocess").unwrap();
        assert!(lsp.definitions && lsp.references);
        assert!(resolve("scip-typescript").is_err());
    }

    fn live_fixture() -> (crate::store::SemanticImport, LiveState) {
        let live = LiveState {
            head: "abc".to_string(),
            branch: "main".to_string(),
            dirty: false,
            untracked: 0,
            config_fingerprint: "cfg".to_string(),
            snapshot_nonce: "n1".to_string(),
            content_identity: "ci".to_string(),
        };
        let import = crate::store::SemanticImport {
            snapshot_id: 1,
            provider_name: "scip-import".to_string(),
            provider_version: "v".to_string(),
            capability: "definitions,references".to_string(),
            project_root: "r".to_string(),
            text_encoding: "UTF8".to_string(),
            artifact_id: "a".to_string(),
            source_revision: revision_string(&live),
            claimed_source_digest: "user-claim".to_string(),
            content_identity: "ci".to_string(),
            external_symbols: "[]".to_string(),
            verification: "Unverified".to_string(),
        };
        (import, live)
    }

    #[test]
    fn verify_live_matrix() {
        let (import, live) = live_fixture();
        // Fully matching attested import: still Unverified (claim ≠ proof).
        assert_eq!(verify_live(&import, &live), Verification::Unverified);
        // Same git fingerprint, different bytes: Stale (nonce ≠ content).
        let mut moved = live_fixture().1;
        moved.content_identity = "ci2".to_string();
        assert_eq!(verify_live(&import, &moved), Verification::Stale);
        // Same bytes, rebuilt nonce without rebind: Stale.
        let mut rebuilt = live_fixture().1;
        rebuilt.snapshot_nonce = "n2".to_string();
        assert_eq!(verify_live(&import, &rebuilt), Verification::Stale);
        // Dependency/config drift with identical bytes: Stale.
        let mut depdrift = live_fixture().1;
        depdrift.config_fingerprint = "cfg2".to_string();
        assert_eq!(verify_live(&import, &depdrift), Verification::Stale);
        // Dirty flag drift: Stale.
        let mut dirty = live_fixture().1;
        dirty.dirty = true;
        assert_eq!(verify_live(&import, &dirty), Verification::Stale);
    }

    #[test]
    fn same_name_across_providers_never_merges() {
        // Identity includes the provider namespace: equal display names
        // from different providers are different evidence, never one symbol.
        let a = Evidence::lsp_location(
            "a.py",
            "d",
            SpanBytes {
                start_line: 1,
                start_col: 0,
                end_line: 1,
                end_col: 5,
                byte_start: 0,
                byte_end: 5,
            },
        );
        let b = Evidence::lsp_location(
            "a.py",
            "d",
            SpanBytes {
                start_line: 1,
                start_col: 0,
                end_line: 1,
                end_col: 5,
                byte_start: 0,
                byte_end: 5,
            },
        );
        assert_eq!(a.location, b.location);
        assert!(a.symbol.is_none() && b.symbol.is_none());
    }
}
