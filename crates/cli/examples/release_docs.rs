// SPDX-License-Identifier: Apache-2.0
//! Release documents generated from the locked dependency graph.
//!
//! ```text
//! cargo run --locked --example release_docs -- --out <dir>
//!     writes sbom.cdx.json (CycloneDX 1.5) and THIRD_PARTY_LICENSES.txt
//! cargo run --locked --example release_docs -- --check
//!     exits 1 on drift: the packages of `cargo metadata` must equal the ones
//!     of Cargo.lock, every package must declare a license, and the committed
//!     release/sbom.cdx.json must equal a freshly generated one
//! ```
//!
//! It needs only std and serde_json (already a dependency). Metadata enables
//! every workspace feature, so the SBOM covers optional dependencies and all
//! target packages. It is a superset of what the default core build links.
//! Nothing here is part of the shipped binary.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

struct Package {
    id: String,
    name: String,
    version: String,
    license: Option<String>,
    license_file: Option<String>,
    repository: Option<String>,
    authors: Vec<String>,
    checksum: Option<String>,
    dir: PathBuf,
    workspace: bool,
}

impl Package {
    fn purl(&self) -> String {
        format!("pkg:cargo/{}@{}", self.name, self.version)
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root")
}

/// `cargo metadata` of the locked graph. All-feature metadata includes optional
/// and target-specific packages. Release and installer jobs run `cargo fetch
/// --locked` first so their license and notice files are locally available;
/// this command runs no package build scripts and fetches no model/runtime.
fn metadata(root: &Path) -> Value {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let out = Command::new(cargo)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--all-features",
        ])
        .current_dir(root)
        .output()
        .expect("cargo metadata must run");
    assert!(
        out.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("cargo metadata is JSON")
}

/// (name, version) -> checksum of every `[[package]]` block of Cargo.lock.
fn lock_packages(root: &Path) -> BTreeMap<(String, String), Option<String>> {
    let text = std::fs::read_to_string(root.join("Cargo.lock")).expect("Cargo.lock");
    let mut packages = BTreeMap::new();
    let mut current: (Option<String>, Option<String>, Option<String>) = (None, None, None);
    let mut flush = |current: &mut (Option<String>, Option<String>, Option<String>)| {
        if let (Some(name), Some(version)) = (current.0.take(), current.1.take()) {
            packages.insert((name, version), current.2.take());
        }
        *current = (None, None, None);
    };
    let value = |line: &str, key: &str| {
        line.strip_prefix(key)
            .and_then(|rest| rest.strip_prefix(" = \""))
            .and_then(|rest| rest.strip_suffix('"'))
            .map(str::to_string)
    };
    for line in text.lines() {
        if line == "[[package]]" {
            flush(&mut current);
        } else if let Some(v) = value(line, "name") {
            current.0 = Some(v);
        } else if let Some(v) = value(line, "version") {
            current.1 = Some(v);
        } else if let Some(v) = value(line, "checksum") {
            current.2 = Some(v);
        }
    }
    flush(&mut current);
    packages
}

fn packages(meta: &Value, lock: &BTreeMap<(String, String), Option<String>>) -> Vec<Package> {
    let members: BTreeSet<&str> = meta["workspace_members"]
        .as_array()
        .expect("workspace_members")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let text = |v: &Value| v.as_str().map(str::to_string);
    let mut out: Vec<Package> = meta["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .map(|p| {
            let name = p["name"].as_str().expect("name").to_string();
            let version = p["version"].as_str().expect("version").to_string();
            Package {
                id: p["id"].as_str().expect("id").to_string(),
                checksum: lock
                    .get(&(name.clone(), version.clone()))
                    .cloned()
                    .flatten(),
                name,
                version,
                // Cargo still accepts the old `MIT/Apache-2.0` spelling.
                license: text(&p["license"]).map(|l| l.replace('/', " OR ")),
                license_file: text(&p["license_file"]),
                repository: text(&p["repository"]),
                authors: p["authors"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect(),
                dir: Path::new(p["manifest_path"].as_str().expect("manifest_path"))
                    .parent()
                    .expect("manifest dir")
                    .to_path_buf(),
                workspace: members.contains(p["id"].as_str().expect("id")),
            }
        })
        .collect();
    out.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    out
}

fn sbom_licenses(package: &Package) -> Value {
    match (&package.license, &package.license_file) {
        (Some(expression), _) => {
            let bundled = bundled_source_license_ids(&package.name);
            if bundled.is_empty() {
                json!([{ "expression": expression }])
            } else {
                let crate_expression =
                    if expression.contains(" AND ") || expression.contains(" OR ") {
                        format!("({expression})")
                    } else {
                        expression.clone()
                    };
                json!([{ "expression": format!("{crate_expression} AND {}", bundled.join(" AND ")) }])
            }
        }
        (None, Some(file)) => json!([{ "license": { "name": format!("see {file}") } }]),
        (None, None) => json!([]),
    }
}

fn sbom(meta: &Value, packages: &[Package]) -> String {
    let by_id: BTreeMap<&str, &Package> = packages.iter().map(|p| (p.id.as_str(), p)).collect();
    let component = |p: &Package, kind: &str| {
        let mut c = json!({
            "type": kind,
            "bom-ref": p.purl(),
            "name": p.name,
            "version": p.version,
            "purl": p.purl(),
            "licenses": sbom_licenses(p),
        });
        if let Some(checksum) = &p.checksum {
            c["hashes"] = json!([{ "alg": "SHA-256", "content": checksum }]);
        }
        if let Some(url) = &p.repository {
            c["externalReferences"] = json!([{ "type": "vcs", "url": url }]);
        }
        c
    };
    let root = packages
        .iter()
        .find(|p| p.workspace && p.name == "riltra")
        .expect("the riltra package");
    let mut dependencies = Vec::new();
    for node in meta["resolve"]["nodes"].as_array().expect("resolve nodes") {
        let Some(package) = node["id"].as_str().and_then(|id| by_id.get(id)) else {
            continue;
        };
        let mut depends_on: Vec<String> = node["deps"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|d| d["pkg"].as_str())
            .filter_map(|id| by_id.get(id))
            .map(|p| p.purl())
            .collect();
        depends_on.sort();
        depends_on.dedup();
        dependencies.push(json!({ "ref": package.purl(), "dependsOn": depends_on }));
    }
    dependencies.sort_by(|a, b| a["ref"].as_str().cmp(&b["ref"].as_str()));
    let components: Vec<Value> = packages
        .iter()
        .filter(|p| !p.workspace)
        .map(|p| component(p, "library"))
        .collect();
    let mut document = json!({
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "version": 1,
        "metadata": { "component": component(root, "application") },
        "components": components,
        "dependencies": dependencies,
    });
    sort_json_keys(&mut document);
    serde_json::to_string_pretty(&document).expect("serialize SBOM") + "\n"
}

/// Feature unification can enable serde_json's `preserve_order`, so normalize
/// every object before emitting the release SBOM.
fn sort_json_keys(value: &mut Value) {
    match value {
        Value::Object(object) => {
            let mut entries: Vec<_> = std::mem::take(object).into_iter().collect();
            for (_, child) in &mut entries {
                sort_json_keys(child);
            }
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));
            object.extend(entries);
        }
        Value::Array(items) => {
            for item in items {
                sort_json_keys(item);
            }
        }
        _ => {}
    }
}

/// The MIT license text, for a package whose published crate carries no
/// license file. The copyright line comes from the crate's declared authors.
const MIT_BODY: &str =
    "Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the \"Software\"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.";

/// What a package's section holds: the license and notice files its published
/// crate carries, or, when it carries none, the canonical text of the license
/// it declares (MIT, or Apache-2.0 from this repository's own LICENSE).
struct LicenseBundle {
    text: String,
    /// Packages whose crate carries no license file; the canonical text was used.
    supplied: Vec<String>,
    /// Packages for which no text could be produced at all.
    missing: Vec<String>,
    /// Required bundled-source license files missing from a native crate.
    problems: Vec<String>,
}

fn bundled_source_licenses(package_name: &str) -> &'static [&'static str] {
    match package_name {
        // These sys crates compile bundled native code with separate licenses.
        "onig_sys" => &["oniguruma/COPYING"],
        "lz4-sys" => &["liblz4/lib/LICENSE"],
        _ => &[],
    }
}

fn bundled_source_license_headers(package_name: &str) -> &'static [&'static str] {
    match package_name {
        // lz4-sys compiles this file, whose xxHash license is not in lib/LICENSE.
        "lz4-sys" => &["liblz4/lib/xxhash.c"],
        _ => &[],
    }
}

fn xxhash_license_header(source: &str) -> Option<&str> {
    let source = source.trim_start();
    let end = source.find("*/")? + 2;
    let header = &source[..end];
    let normalized = header
        .lines()
        .map(|line| {
            line.trim()
                .trim_start_matches("/*")
                .trim_start_matches('*')
                .trim_end_matches("*/")
                .trim()
        })
        .collect::<Vec<_>>()
        .join(" ");
    let required = [
        "xxHash - Fast Hash algorithm",
        "Copyright (C) 2012-2016, Yann Collet",
        "BSD 2-Clause License",
        "Redistribution and use in source and binary forms",
        "Redistributions in binary form must reproduce",
        "THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS",
        "IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE",
        "OF THE POSSIBILITY OF SUCH DAMAGE.",
    ];
    let missing: Vec<_> = required
        .iter()
        .filter(|required| !normalized.contains(**required))
        .collect();
    missing.is_empty().then_some(header)
}

fn bundled_source_license_ids(package_name: &str) -> &'static [&'static str] {
    match package_name {
        "onig_sys" | "lz4-sys" => &["BSD-2-Clause"],
        _ => &[],
    }
}

fn declares(license: &str, id: &str) -> bool {
    license.split([' ', '(', ')']).any(|token| token == id)
}

fn third_party_licenses(root: &Path, packages: &[Package]) -> LicenseBundle {
    let apache = std::fs::read_to_string(root.join("LICENSE")).unwrap_or_default();
    let mut bundle = LicenseBundle {
        text: String::from(
            "Third-party software covered by the locked dependency inventory
             ===============================================================

             This inventory covers every package in Cargo.lock for all workspace
             features and targets. Optional packages may not be linked in the
             default core build; their notices are included as a superset.

             Each section names a package, its declared license expression and the
             license or notice files that the published crate carries. A package whose
             crate carries none gets the canonical text of the license it declares,
             with the crate's declared authors as the copyright holders.

",
        ),
        supplied: Vec::new(),
        missing: Vec::new(),
        problems: Vec::new(),
    };
    for p in packages.iter().filter(|p| !p.workspace) {
        let mut source_headers = Vec::new();
        let mut files: Vec<PathBuf> = std::fs::read_dir(&p.dir)
            .map(|rd| {
                rd.filter_map(Result::ok)
                    .map(|e| e.path())
                    .filter(|path| {
                        let name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_ascii_lowercase();
                        path.is_file()
                            && ["license", "licence", "copying", "notice", "unlicense"]
                                .iter()
                                .any(|prefix| name.starts_with(prefix))
                    })
                    .collect()
            })
            .unwrap_or_default();
        files.sort();
        for relative in bundled_source_licenses(&p.name) {
            let path = p.dir.join(relative);
            match std::fs::read_to_string(&path) {
                Ok(_) => files.push(path),
                Err(error) => bundle.problems.push(format!(
                    "{} {} cannot read bundled-source license {}: {error}",
                    p.name, p.version, relative
                )),
            }
        }
        for relative in bundled_source_license_headers(&p.name) {
            let path = p.dir.join(relative);
            match std::fs::read_to_string(&path) {
                Ok(source) => match xxhash_license_header(&source) {
                    Some(header) => {
                        source_headers.push((relative.replace('\\', "/"), header.to_string()))
                    }
                    None => bundle.problems.push(format!(
                        "{} {} cannot read bundled-source license header {}: unrecognized xxHash BSD-2-Clause header",
                        p.name, p.version, relative
                    )),
                },
                Err(error) => bundle.problems.push(format!(
                    "{} {} cannot read bundled-source license header {}: {error}",
                    p.name, p.version, relative
                )),
            }
        }
        files.sort();
        let license = p.license.as_deref().unwrap_or("");
        bundle.text.push_str(&format!(
            "-----
{} {} ({})
-----
",
            p.name,
            p.version,
            if license.is_empty() {
                "no license expression"
            } else {
                license
            }
        ));
        let mut wrote = false;
        for file in files {
            if let Ok(text) = std::fs::read_to_string(&file) {
                let relative = file.strip_prefix(&p.dir).unwrap_or(&file);
                let display_path = relative
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                bundle.text.push_str(&format!(
                    "
[{}]
{}
",
                    display_path,
                    text.trim_end()
                ));
                wrote = true;
            }
        }
        for (relative, header) in source_headers {
            bundle.text.push_str(&format!(
                "\n[{relative} license header]\n{}\n",
                header.trim_end()
            ));
            wrote = true;
        }
        if !wrote {
            let who = format!("{} {}", p.name, p.version);
            let holders = if p.authors.is_empty() {
                format!("The {} authors", p.name)
            } else {
                p.authors.join(", ")
            };
            if declares(license, "MIT") {
                bundle.text.push_str(&format!(
                    "
[MIT license text supplied by the release tool: the published crate carries no license file]
Copyright (c) {holders}

{MIT_BODY}
"
                ));
                bundle.supplied.push(who);
            } else if declares(license, "Apache-2.0") && !apache.is_empty() {
                bundle.text.push_str(&format!(
                    "
[Apache-2.0 license text supplied by the release tool: the published crate carries no license file; copyright: {holders}]
{}
",
                    apache.trim_end()
                ));
                bundle.supplied.push(who);
            } else {
                bundle.text.push_str(
                    "
(no license text could be produced)
",
                );
                bundle.missing.push(who);
            }
        }
        bundle.text.push('\n');
    }
    bundle
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = workspace_root();
    let meta = metadata(&root);
    let lock = lock_packages(&root);
    let packages = packages(&meta, &lock);

    // Zero drift against Cargo.lock, in both directions.
    let from_meta: BTreeSet<(String, String)> = packages
        .iter()
        .map(|p| (p.name.clone(), p.version.clone()))
        .collect();
    let from_lock: BTreeSet<(String, String)> = lock.keys().cloned().collect();
    let mut problems: Vec<String> = Vec::new();
    for missing in from_lock.difference(&from_meta) {
        problems.push(format!("in Cargo.lock but not in the graph: {missing:?}"));
    }
    for extra in from_meta.difference(&from_lock) {
        problems.push(format!("in the graph but not in Cargo.lock: {extra:?}"));
    }
    for p in packages.iter().filter(|p| !p.workspace) {
        if p.license.is_none() && p.license_file.is_none() {
            problems.push(format!("{} {} declares no license", p.name, p.version));
        }
        if p.checksum.is_none() {
            problems.push(format!(
                "{} {} has no checksum in Cargo.lock",
                p.name, p.version
            ));
        }
    }

    let sbom_text = sbom(&meta, &packages);
    let bundle = third_party_licenses(&root, &packages);
    problems.extend(bundle.problems.iter().cloned());
    eprintln!(
        "release_docs: {} packages in the locked graph ({} third-party), {} without a license file in the crate (canonical text supplied)",
        packages.len(),
        packages.iter().filter(|p| !p.workspace).count(),
        bundle.supplied.len()
    );
    for name in &bundle.supplied {
        eprintln!("  canonical text supplied: {name}");
    }
    for name in &bundle.missing {
        problems.push(format!("{name}: no license text could be produced"));
    }

    match args.first().map(String::as_str) {
        Some("--out") => {
            let dir = PathBuf::from(args.get(1).expect("--out needs a directory"));
            std::fs::create_dir_all(&dir).expect("create output directory");
            std::fs::write(dir.join("sbom.cdx.json"), &sbom_text).expect("write SBOM");
            std::fs::write(dir.join("THIRD_PARTY_LICENSES.txt"), &bundle.text)
                .expect("write licenses");
            if !problems.is_empty() {
                for p in &problems {
                    eprintln!("drift: {p}");
                }
                std::process::exit(1);
            }
        }
        Some("--check") => {
            // Every third-party package has a row in the audit table.
            let licensing =
                std::fs::read_to_string(root.join("docs").join("licensing.md")).unwrap_or_default();
            for p in packages.iter().filter(|p| !p.workspace) {
                if !licensing.contains(&format!("| {} | {} |", p.name, p.version)) {
                    problems.push(format!(
                        "docs/licensing.md has no row for {} {}",
                        p.name, p.version
                    ));
                }
            }
            let committed = root.join("release").join("sbom.cdx.json");
            match std::fs::read_to_string(&committed) {
                Ok(text) if text == sbom_text => {}
                Ok(_) => problems.push(
                    "release/sbom.cdx.json differs from the locked graph; regenerate it with --out release".to_string(),
                ),
                Err(e) => problems.push(format!("release/sbom.cdx.json is unreadable: {e}")),
            }
            if problems.is_empty() {
                eprintln!("release_docs: no drift");
            } else {
                for p in &problems {
                    eprintln!("drift: {p}");
                }
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("usage: release_docs --out <dir> | --check");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{sbom, third_party_licenses, Package};

    static NEXT_TEMP_DIR: AtomicU64 = AtomicU64::new(0);

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let id = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("riltra-release-docs-{}-{id}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn package(name: &str, dir: PathBuf) -> Package {
        Package {
            id: format!("registry+https://example.invalid/{name}@1.0.0"),
            name: name.to_string(),
            version: "1.0.0".to_string(),
            license: Some("MIT".to_string()),
            license_file: None,
            repository: None,
            authors: Vec::new(),
            checksum: Some("0".repeat(64)),
            dir,
            workspace: false,
        }
    }

    fn assert_json_keys_sorted(value: &serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                let keys: Vec<_> = object.keys().map(String::as_str).collect();
                assert!(
                    keys.windows(2).all(|pair| pair[0] <= pair[1]),
                    "JSON object keys are not sorted: {keys:?}"
                );
                for value in object.values() {
                    assert_json_keys_sorted(value);
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    assert_json_keys_sorted(item);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn sbom_json_keys_are_sorted_under_preserve_order() {
        let temp = TempDir::new();
        let mut root = package("riltra", temp.0.join("riltra"));
        root.workspace = true;
        let dependency = package("example-dep", temp.0.join("example-dep"));
        let root_id = root.id.clone();
        let dependency_id = dependency.id.clone();
        let metadata = serde_json::json!({
            "resolve": {
                "nodes": [
                    { "id": root_id, "deps": [{ "pkg": dependency_id }] },
                    { "id": dependency_id, "deps": [] },
                ],
            },
        });

        let document = sbom(&metadata, &[root, dependency]);
        let parsed: serde_json::Value = serde_json::from_str(&document).unwrap();
        assert_json_keys_sorted(&parsed);
    }

    #[test]
    fn native_sys_crates_include_the_bundled_code_licenses_in_notices() {
        const XXHASH_HEADER: &str = "/* xxHash - Fast Hash algorithm\nCopyright (C) 2012-2016, Yann Collet\nBSD 2-Clause License\nRedistribution and use in source and binary forms\nRedistributions in binary form must reproduce\nTHIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS\nIN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE\nOF THE POSSIBILITY OF SUCH DAMAGE.\n*/";
        let temp = TempDir::new();
        let root = temp.0.join("root");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("LICENSE"), "workspace Apache license").unwrap();

        let cases = [
            (
                "onig_sys",
                "oniguruma/COPYING",
                "Oniguruma BSD-2-Clause source license",
            ),
            (
                "lz4-sys",
                "liblz4/lib/LICENSE",
                "bundled LZ4 BSD-2-Clause source license",
            ),
        ];
        let mut packages = Vec::new();
        for (name, relative, text) in cases {
            let dir = root.join(name);
            std::fs::create_dir_all(dir.join(std::path::Path::new(relative).parent().unwrap()))
                .unwrap();
            std::fs::write(dir.join("LICENSE.md"), "crate MIT license").unwrap();
            std::fs::write(dir.join(relative), text).unwrap();
            if name == "lz4-sys" {
                std::fs::write(dir.join("liblz4/lib/xxhash.c"), XXHASH_HEADER).unwrap();
            }
            packages.push(package(name, dir));
        }

        let bundle = third_party_licenses(&root, &packages);

        assert!(bundle.text.contains("Optional packages may not be linked"));
        assert!(bundle
            .text
            .contains("[oniguruma/COPYING]\nOniguruma BSD-2-Clause source license"));
        assert!(bundle
            .text
            .contains("[liblz4/lib/LICENSE]\nbundled LZ4 BSD-2-Clause source license"));
        assert!(bundle.text.contains("Copyright (C) 2012-2016, Yann Collet"));
        assert!(bundle.problems.is_empty(), "{:?}", bundle.problems);
    }

    #[test]
    fn lz4_notice_includes_compiled_xxhash_copyright_and_license_header() {
        const XXHASH_HEADER: &str = r#"/*
*  xxHash - Fast Hash algorithm
*  Copyright (C) 2012-2016, Yann Collet
*
*  BSD 2-Clause License (http://www.opensource.org/licenses/bsd-license.php)
*
*  Redistribution and use in source and binary forms, with or without
*  modification, are permitted provided that the following conditions are
*  met:
*
*  * Redistributions of source code must retain the above copyright
*  notice, this list of conditions and the following disclaimer.
*  * Redistributions in binary form must reproduce the above
*  copyright notice, this list of conditions and the following disclaimer
*  in the documentation and/or other materials provided with the
*  distribution.
*
*  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
*  "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
*  LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
*  A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
*  OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
*  SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
*  LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
*  DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
*  THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
*  (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
*  OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
*/
"#;
        let temp = TempDir::new();
        let root = temp.0.join("root");
        let dir = root.join("lz4-sys");
        std::fs::create_dir_all(dir.join("liblz4/lib")).unwrap();
        std::fs::write(root.join("LICENSE"), "workspace Apache license").unwrap();
        std::fs::write(dir.join("LICENSE.md"), "crate MIT license").unwrap();
        std::fs::write(dir.join("liblz4/lib/LICENSE"), "LZ4 BSD-2-Clause license").unwrap();
        std::fs::write(dir.join("liblz4/lib/xxhash.c"), XXHASH_HEADER).unwrap();

        let bundle = third_party_licenses(&root, &[package("lz4-sys", dir)]);

        assert!(
            bundle.text.contains("Copyright (C) 2012-2016, Yann Collet"),
            "problems: {:?}; text: {}",
            bundle.problems,
            bundle.text
        );
        assert!(bundle.text.contains("BSD 2-Clause License"));
        assert!(bundle
            .text
            .contains("Redistributions in binary form must reproduce"));
        assert!(bundle.text.contains("MERCHANTABILITY AND FITNESS FOR"));
        assert!(bundle.problems.is_empty(), "{:?}", bundle.problems);
    }

    #[test]
    fn missing_or_unrecognized_xxhash_license_header_fails_notice_generation() {
        for source in [
            None,
            Some("/* xxHash source with no license */"),
            Some("int xxhash_source;\n"),
        ] {
            let temp = TempDir::new();
            let root = temp.0.join("root");
            let dir = root.join("lz4-sys");
            std::fs::create_dir_all(dir.join("liblz4/lib")).unwrap();
            std::fs::write(root.join("LICENSE"), "workspace Apache license").unwrap();
            std::fs::write(dir.join("LICENSE.md"), "crate MIT license").unwrap();
            std::fs::write(dir.join("liblz4/lib/LICENSE"), "LZ4 BSD-2-Clause license").unwrap();
            if let Some(source) = source {
                std::fs::write(dir.join("liblz4/lib/xxhash.c"), source).unwrap();
            }

            let bundle = third_party_licenses(&root, &[package("lz4-sys", dir)]);

            assert_eq!(bundle.problems.len(), 1);
            assert!(bundle.problems[0].contains(
                "lz4-sys 1.0.0 cannot read bundled-source license header liblz4/lib/xxhash.c"
            ));
        }
    }

    #[test]
    fn missing_bundled_native_license_fails_notice_generation() {
        let temp = TempDir::new();
        let root = temp.0.join("root");
        let dir = root.join("onig_sys");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(root.join("LICENSE"), "workspace Apache license").unwrap();
        std::fs::write(dir.join("LICENSE.md"), "crate MIT license").unwrap();

        let bundle = third_party_licenses(&root, &[package("onig_sys", dir)]);

        assert_eq!(bundle.problems.len(), 1);
        assert!(bundle.problems[0]
            .contains("onig_sys 1.0.0 cannot read bundled-source license oniguruma/COPYING"));
    }

    #[test]
    fn sbom_combines_crate_and_bundled_native_source_licenses() {
        assert_eq!(
            super::sbom_licenses(&package("onig_sys", PathBuf::new())),
            serde_json::json!([{ "expression": "MIT AND BSD-2-Clause" }])
        );
        assert_eq!(
            super::sbom_licenses(&package("lz4-sys", PathBuf::new())),
            serde_json::json!([{ "expression": "MIT AND BSD-2-Clause" }])
        );
    }
}
