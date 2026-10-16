// SPDX-License-Identifier: Apache-2.0
//! Tier A contract corpus. Producer occurrences and expected relationships are
//! authored independently of tree-sitter/CLI output; no external indexer runs.
use super::*;
use serde_json::{json, Value};

fn number(v: &Value, key: &str) -> usize {
    v[key].as_u64().unwrap() as usize
}

fn raw_span(source: &str, occurrence: &Value) -> (usize, usize) {
    let line = number(occurrence, "line");
    let base: usize = source.split_inclusive('\n').take(line).map(str::len).sum();
    let span = (
        base + number(occurrence, "col"),
        base + number(occurrence, "end"),
    );
    assert_eq!(
        &source[span.0..span.1],
        occurrence["text"].as_str().unwrap()
    );
    span
}

fn query(h: &Path, command: &str, file: &str, anchor: &Value) -> Value {
    let line = (number(anchor, "line") + 1).to_string();
    let col = (number(anchor, "col") + 1).to_string();
    let (code, out, err) = run(
        h,
        &[
            command, "--repo", "sem", "--path", file, "--line", &line, "--col", &col,
        ],
    );
    assert_eq!(code, 0, "{command} {file}: {err}");
    let args = [
        command, "--repo", "sem", "--path", file, "--line", &line, "--col", &col,
    ];
    let (again, repeated, err) = run(h, &args);
    assert_eq!(again, 0, "repeat: {err}");
    assert_eq!(out, repeated, "raw JSON must repeat byte for byte");
    let value: Value = serde_json::from_str(&out).unwrap();
    fn artifacts(value: &Value, nonce: &str) {
        match value {
            Value::Object(fields) => {
                for (key, v) in fields {
                    if key == "artifact_id" {
                        assert!(!v.as_str().unwrap().is_empty());
                    }
                    if key == "source_revision" {
                        assert!(v.as_str().unwrap().contains(nonce));
                    }
                    artifacts(v, nonce);
                }
            }
            Value::Array(rows) => {
                for row in rows {
                    artifacts(row, nonce);
                }
            }
            _ => {}
        }
    }
    artifacts(&value, value["snapshot"]["nonce"].as_str().unwrap());
    normalized_edge_json(value)
}

fn check_evidence(value: &Value, source: &str, manifest: &Value, semantic: bool) {
    let symbols = manifest["symbols"].as_array().unwrap();
    let occurrences = manifest["occurrences"].as_array().unwrap();
    let mut digest = 0xcbf29ce484222325_u64;
    for b in source.bytes() {
        digest = (digest ^ b as u64).wrapping_mul(0x100000001b3);
    }
    fn edges(value: &Value) -> Vec<&Value> {
        if let Some(rows) = value["edges"].as_array() {
            return rows.iter().collect();
        }
        value["paths"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|p| p["edges"].as_array().unwrap())
            .collect()
    }
    for edge in edges(value) {
        let evidence = &edge["evidence"];
        assert_eq!(evidence["path"], manifest["file"]);
        assert_eq!(evidence["digest"], format!("{digest:016x}"));
        let span = evidence["span"].as_array().unwrap();
        let (a, b) = (
            span[0].as_u64().unwrap() as usize,
            span[1].as_u64().unwrap() as usize,
        );
        assert!(
            a < b && b <= source.len(),
            "invalid source evidence: {edge}"
        );
        assert_ne!(edge["verification"], "Current");
        if edge["kind"] != "CALLS" {
            continue;
        }
        assert!(semantic, "syntax must never prove CALLS");
        assert_eq!(edge["precision"], "Exact");
        assert_eq!(edge["caller"]["import_id"], 1);
        assert_eq!(edge["callee"]["import_id"], 1);
        assert_eq!(evidence["imports"][0]["provider"], "corpus");
        let terminal = evidence["terminal_span"].as_array().unwrap();
        let (ta, tb) = (
            terminal[0].as_u64().unwrap() as usize,
            terminal[1].as_u64().unwrap() as usize,
        );
        assert!(a <= ta && ta < tb && tb <= b);
        let uri = edge["callee"]["symbol_uri"].as_str().unwrap();
        let symbol = symbols
            .iter()
            .find(|s| s["uri"] == uri)
            .expect("callee must be fixture identity");
        assert!(
            occurrences
                .iter()
                .any(|o| o["id"] == symbol["id"] && o["definition"] == true),
            "callee definition proof"
        );
        assert!(
            occurrences.iter().any(|o| {
                !o["definition"].as_bool().unwrap() && o["id"] == symbol["id"] && {
                    let (ra, rb) = raw_span(source, o);
                    ra < tb && ta < rb && evidence["reference_span"] == json!([ra, rb])
                }
            }),
            "matching terminal producer proof: {edge}"
        );
        assert_eq!(&source[ta..tb], edge["callee_name"].as_str().unwrap());
        if uri.starts_with("local ") {
            assert_eq!(edge["callee"]["doc"], manifest["file"]);
        }
    }
    if let Some(verdict) = value.get("verdict") {
        assert!(matches!(
            verdict.as_str().unwrap(),
            "HIGH" | "MEDIUM" | "UNKNOWN"
        ));
        assert_eq!(value["completeness"]["status"], "partial");
    }
}

fn corpus(lang: &str, case: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let fixture = root.join("fixtures/m6").join(lang);
    let manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(fixture.join(format!("{case}.json"))).unwrap(),
    )
    .unwrap();
    let file = manifest["file"].as_str().unwrap();
    let target = manifest["target"].as_str().unwrap();
    let source = std::fs::read_to_string(fixture.join(file)).unwrap();
    let occurrences = manifest["occurrences"].as_array().unwrap();
    for o in occurrences {
        raw_span(&source, o);
    }
    let definition = |id| {
        occurrences
            .iter()
            .find(|o| o["id"] == id && o["definition"] == true)
            .expect("fixture anchor")
    };
    let h = home(&format!("corpus-{lang}-{case}"));
    let mirror = format!("other_{file}");
    let files = if case == "local_scope" {
        vec![(file, source.as_str()), (mirror.as_str(), source.as_str())]
    } else {
        vec![(file, source.as_str())]
    };
    let dir = repo_with(&h, "sem", &files);
    do_index(&h);
    let mut bundle = json!({});
    for (key, cmd, id) in [
        ("syntax_callees", "callees", "c"),
        ("syntax_impact", "impact", target),
    ] {
        let v = query(&h, cmd, file, definition(id));
        check_evidence(&v, &source, &manifest, false);
        if cmd == "impact" {
            assert_eq!(v["verdict"], "UNKNOWN");
        }
        bundle[key] = v;
    }
    let symbols = manifest["symbols"].as_array().unwrap();
    let occ: Vec<_> = occurrences
        .iter()
        .map(|o| {
            let symbol = symbols.iter().find(|s| s["id"] == o["id"]).unwrap();
            occurrence(
                symbol["uri"].as_str().unwrap(),
                u64::from(o["definition"].as_bool().unwrap()),
                number(o, "line") as i64,
                number(o, "col") as i64,
                number(o, "end") as i64,
            )
        })
        .collect();
    let info: Vec<_> = symbols
        .iter()
        .map(|s| (s["uri"].as_str().unwrap(), s["name"].as_str().unwrap()))
        .collect();
    let mut documents = vec![test_document(file, &occ, &info)];
    if case == "local_scope" {
        // Same local URI in two documents: only the document is identity
        // evidence. Global caller symbols belong to their own documents.
        let mirrored: Vec<_> = symbols
            .iter()
            .map(|s| {
                let uri = s["uri"].as_str().unwrap();
                (
                    s["id"].as_str().unwrap(),
                    if uri.starts_with("local ") {
                        uri.to_string()
                    } else {
                        uri.replace(file, &mirror)
                    },
                    s["name"].as_str().unwrap(),
                )
            })
            .collect();
        let other_occ: Vec<_> = occurrences
            .iter()
            .map(|o| {
                let (_, uri, _) = mirrored
                    .iter()
                    .find(|(id, _, _)| *id == o["id"].as_str().unwrap())
                    .unwrap();
                occurrence(
                    uri,
                    u64::from(o["definition"].as_bool().unwrap()),
                    number(o, "line") as i64,
                    number(o, "col") as i64,
                    number(o, "end") as i64,
                )
            })
            .collect();
        let other_info: Vec<_> = mirrored
            .iter()
            .map(|(_, uri, name)| (uri.as_str(), *name))
            .collect();
        documents.push(test_document(&mirror, &other_occ, &other_info));
    }
    import_documents(&h, &dir, "corpus", &documents);
    for (cmd, id) in [
        ("callers", target),
        ("callees", "c"),
        ("trace", "c"),
        ("impact", target),
    ] {
        let v = query(&h, cmd, file, definition(id));
        check_evidence(&v, &source, &manifest, true);
        assert_eq!(
            v["snapshot"], bundle["syntax_impact"]["snapshot"],
            "SCIP import must work without reindex"
        );
        if cmd == "callees" || cmd == "callers" {
            let rows = v["edges"].as_array().unwrap();
            let calls = rows.iter().filter(|e| e["kind"] == "CALLS").count();
            let expected = if cmd == "callees" {
                "callees_calls"
            } else {
                "inbound_calls"
            };
            assert_eq!(
                calls,
                number(&manifest, expected),
                "{lang}/{case} {cmd}: {v}"
            );
            if cmd == "callers" {
                assert_eq!(
                    rows.iter().filter(|e| e["kind"] == "REFERENCES").count(),
                    number(&manifest, "inbound_references")
                );
            }
        }
        if cmd == "impact" {
            assert_eq!(v["verdict"], manifest["impact"], "{lang}/{case}: {v}");
        }
        if cmd == "trace" && matches!(case, "recursion" | "cycle") {
            assert_eq!(v["cycle"], true);
        }
        bundle[cmd] = v;
    }
    if manifest["stale"] == true {
        let comment = if lang == "py" {
            "# edited\n"
        } else {
            "// edited\n"
        };
        std::fs::write(dir.join(file), format!("{source}{comment}")).unwrap();
        for (cmd, id) in [
            ("callers", "t"),
            ("callees", "c"),
            ("trace", "c"),
            ("impact", "t"),
        ] {
            let v = query(&h, cmd, file, definition(id));
            check_evidence(&v, &source, &manifest, true);
            assert_eq!(v["verification"], "Stale");
            if cmd == "impact" {
                assert_eq!(v["verdict"], "UNKNOWN");
            }
            bundle[format!("stale_{cmd}")] = v;
        }
    }
    let golden = root
        .join("golden/m6-corpus")
        .join(lang)
        .join(format!("{case}.json"));
    if std::env::var_os("RILTRA_UPDATE_M6_GOLDENS").as_deref() == Some(std::ffi::OsStr::new("1")) {
        // Explicit maintainer action, after every independent truth assertion.
        std::fs::write(
            &golden,
            serde_json::to_string_pretty(&bundle).unwrap() + "\n",
        )
        .unwrap();
    }
    let expected: Value = serde_json::from_str(&std::fs::read_to_string(golden).unwrap()).unwrap();
    assert!(bundle==expected,"{lang}/{case} full JSON golden differs (inspect the checked-in JSON and source-truth assertions)");
}

macro_rules! language {
    ($lang:ident) => {
        mod $lang {
            macro_rules! case {
                ($name:ident) => {
                    #[test]
                    fn $name() {
                        super::corpus(stringify!($lang), stringify!($name));
                    }
                };
            }
            case!(direct);
            case!(reference);
            case!(nested_calls);
            case!(local_scope);
            case!(same_name);
            case!(alias);
            case!(recursion);
            case!(cycle);
            case!(missing_definition);
            case!(dynamic_dispatch);
            case!(unresolved);
            case!(stale_after_edit);
            case!(unsupported);
            case!(competing_targets);
            case!(same_name_binding);
        }
    };
}
language!(py);
language!(ts);
language!(rs);
language!(c);
language!(cpp);

#[test]
fn py_nested_anonymous() {
    corpus("py", "nested_anonymous");
}
#[test]
fn ts_nested_anonymous() {
    corpus("ts", "nested_anonymous");
}
#[test]
fn rs_nested_anonymous() {
    corpus("rs", "nested_anonymous");
}
#[test]
fn cpp_nested_anonymous() {
    corpus("cpp", "nested_anonymous");
}
