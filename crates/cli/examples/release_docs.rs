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
//! It needs only std and serde_json (already a dependency). The SBOM lists every package of the resolved
//! graph for every platform, a superset of what one native build compiles.
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

/// `cargo metadata` of the locked graph. It reads the manifest of every package
/// for every platform, so a first run downloads the ones this host does not
/// build (crates.io, checksummed by Cargo.lock; no build script runs).
fn metadata(root: &Path) -> Value {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let out = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--locked"])
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

fn sbom(meta: &Value, packages: &[Package]) -> String {
    let by_id: BTreeMap<&str, &Package> = packages.iter().map(|p| (p.id.as_str(), p)).collect();
    let licenses = |p: &Package| match (&p.license, &p.license_file) {
        (Some(expression), _) => json!([{ "expression": expression }]),
        (None, Some(file)) => json!([{ "license": { "name": format!("see {file}") } }]),
        (None, None) => json!([]),
    };
    let component = |p: &Package, kind: &str| {
        let mut c = json!({
            "type": kind,
            "bom-ref": p.purl(),
            "name": p.name,
            "version": p.version,
            "purl": p.purl(),
            "licenses": licenses(p),
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
    let document = json!({
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "version": 1,
        "metadata": { "component": component(root, "application") },
        "components": components,
        "dependencies": dependencies,
    });
    serde_json::to_string_pretty(&document).expect("serialize SBOM") + "\n"
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
}

fn declares(license: &str, id: &str) -> bool {
    license.split([' ', '(', ')']).any(|token| token == id)
}

fn third_party_licenses(root: &Path, packages: &[Package]) -> LicenseBundle {
    let apache = std::fs::read_to_string(root.join("LICENSE")).unwrap_or_default();
    let mut bundle = LicenseBundle {
        text: String::from(
            "Third-party software included in this distribution
             ===================================================

             Each section names a package, its declared license expression and the
             license or notice files that the published crate carries. A package whose
             crate carries none gets the canonical text of the license it declares,
             with the crate's declared authors as the copyright holders.

",
        ),
        supplied: Vec::new(),
        missing: Vec::new(),
    };
    for p in packages.iter().filter(|p| !p.workspace) {
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
                bundle.text.push_str(&format!(
                    "
[{}]
{}
",
                    file.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                    text.trim_end()
                ));
                wrote = true;
            }
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
