// SPDX-License-Identifier: Apache-2.0
use std::collections::BTreeMap;
use std::path::Path;

pub const SOURCE: &[u8] =
    include_bytes!("../../../../docs/pilots/wiy-caller-precision-2026-10-31/source/src/lib.rs");

pub fn write_fixture(root: &Path) {
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"precision_fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .unwrap();
    std::fs::write(root.join("src/lib.rs"), SOURCE).unwrap();
}

pub fn scip_payload(root: &Path) -> Vec<u8> {
    let a = "rust-analyzer cargo precision_fixture 0.1.0 a/target().";
    let b = "rust-analyzer cargo precision_fixture 0.1.0 b/target().";
    let direct = "rust-analyzer cargo precision_fixture 0.1.0 direct().";
    let alias = "rust-analyzer cargo precision_fixture 0.1.0 alias().";
    let reference = "rust-analyzer cargo precision_fixture 0.1.0 reference().";
    let other = "rust-analyzer cargo precision_fixture 0.1.0 other().";
    let local_f = "local 0";
    let local_reference = "local 1";
    let mut occurrences = Vec::new();
    let mut symbols = BTreeMap::new();
    for (uri, display, roles, context, needle) in [
        (
            a,
            "target",
            1,
            "pub mod a { pub fn target() -> i32 { 1 } }",
            "target",
        ),
        (
            b,
            "target",
            1,
            "pub mod b { pub fn target() -> i32 { 2 } }",
            "target",
        ),
        (
            direct,
            "direct",
            1,
            "pub fn direct() -> i32 { a::target() }",
            "direct",
        ),
        (
            a,
            "target",
            0,
            "pub fn direct() -> i32 { a::target() }",
            "target",
        ),
        (
            alias,
            "alias",
            1,
            "pub fn alias() -> i32 { let f = a::target; f() }",
            "alias",
        ),
        (local_f, "f", 1, "let f = a::target;", "f"),
        (
            a,
            "target",
            0,
            "pub fn alias() -> i32 { let f = a::target; f() }",
            "target",
        ),
        (local_f, "f", 0, "; f() }", "f"),
        (
            reference,
            "reference",
            1,
            "pub fn reference() { let _f = a::target; }",
            "reference",
        ),
        (local_reference, "_f", 1, "let _f = a::target;", "_f"),
        (
            a,
            "target",
            0,
            "pub fn reference() { let _f = a::target; }",
            "target",
        ),
        (
            other,
            "other",
            1,
            "pub fn other() -> i32 { b::target() }",
            "other",
        ),
        (
            b,
            "target",
            0,
            "pub fn other() -> i32 { b::target() }",
            "target",
        ),
    ] {
        occurrences.push(occurrence(uri, roles, location(context, needle)));
        symbols.insert(uri, display);
    }

    let mut document = Vec::new();
    str_field(1, "src/lib.rs", &mut document);
    for occurrence in occurrences {
        len_field(2, &occurrence, &mut document);
    }
    for (uri, display) in symbols {
        let mut symbol = Vec::new();
        str_field(1, uri, &mut symbol);
        str_field(6, display, &mut symbol);
        len_field(3, &symbol, &mut document);
    }
    varint_field(6, 1, &mut document);

    let mut tool = Vec::new();
    str_field(1, "rust-analyzer", &mut tool);
    str_field(2, "1.0", &mut tool);
    let mut metadata = Vec::new();
    len_field(2, &tool, &mut metadata);
    str_field(
        3,
        &format!("file://{}", root.to_string_lossy().replace('\\', "/")),
        &mut metadata,
    );
    varint_field(4, 1, &mut metadata);
    let mut payload = Vec::new();
    len_field(1, &metadata, &mut payload);
    len_field(2, &document, &mut payload);
    payload
}

fn location(context: &str, needle: &str) -> (i64, i64, i64) {
    let context = context.as_bytes();
    let context_start = find(SOURCE, context).expect("fixture context");
    let within = &SOURCE[context_start..context_start + context.len()];
    let needle_start = find(within, needle.as_bytes()).expect("fixture occurrence");
    let start = context_start + needle_start;
    let end = start + needle.len();
    let row = SOURCE[..start].iter().filter(|b| **b == b'\n').count();
    let line_start = SOURCE[..start]
        .iter()
        .rposition(|b| *b == b'\n')
        .map_or(0, |i| i + 1);
    (
        row as i64,
        (start - line_start) as i64,
        (end - line_start) as i64,
    )
}

fn find(source: &[u8], needle: &[u8]) -> Option<usize> {
    source
        .windows(needle.len())
        .position(|candidate| candidate == needle)
}

fn occurrence(symbol: &str, roles: u64, (line, start, end): (i64, i64, i64)) -> Vec<u8> {
    let mut range = Vec::new();
    varint_field(1, line as u64, &mut range);
    varint_field(2, start as u64, &mut range);
    varint_field(3, end as u64, &mut range);
    let mut occurrence = Vec::new();
    str_field(2, symbol, &mut occurrence);
    varint_field(3, roles, &mut occurrence);
    len_field(8, &range, &mut occurrence);
    occurrence
}

fn varint(mut value: u64, out: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}

fn tag(field: u32, wire: u32, out: &mut Vec<u8>) {
    varint(((field << 3) | wire) as u64, out);
}

fn len_field(field: u32, bytes: &[u8], out: &mut Vec<u8>) {
    tag(field, 2, out);
    varint(bytes.len() as u64, out);
    out.extend_from_slice(bytes);
}

fn str_field(field: u32, value: &str, out: &mut Vec<u8>) {
    len_field(field, value.as_bytes(), out);
}

fn varint_field(field: u32, value: u64, out: &mut Vec<u8>) {
    tag(field, 0, out);
    varint(value, out);
}
