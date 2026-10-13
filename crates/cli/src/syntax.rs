// SPDX-License-Identifier: Apache-2.0
//! M2 syntax baseline: tree-sitter definitions, imports, and local call
//! candidates with source spans and byte offsets.
//!
//! No semantic resolution anywhere here: a "call" records the callee text as
//! written (e.g. `w.process`), never a resolved target. Anything the grammar
//! cannot represent is reported via `errors` or left out, never guessed.

use std::path::Path;

use crate::json::escape;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Language {
    Python,
    JavaScript,
    TypeScript,
    Tsx,
    Rust,
    C,
    Cpp,
}

pub fn language_name(lang: Language) -> &'static str {
    match lang {
        Language::Python => "python",
        Language::JavaScript => "javascript",
        Language::TypeScript => "typescript",
        Language::Tsx => "tsx",
        Language::Rust => "rust",
        Language::C => "c",
        Language::Cpp => "cpp",
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileKind {
    Supported(Language),
    /// CUDA is explicitly experimental/unsupported; never force-parsed.
    CudaExperimental,
    UnsupportedExtension,
}

/// Extension map. Headers (`.h`) parse as C++: real-world headers mix both
/// languages and the C++ grammar covers C declarations, while the C grammar
/// misreads C++ constructs (namespaces/classes) as functions with errors.
/// Use `.c` for strict C parsing.
pub fn classify(path: &Path) -> FileKind {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("py") => FileKind::Supported(Language::Python),
        Some("js" | "mjs" | "cjs") => FileKind::Supported(Language::JavaScript),
        Some("ts" | "mts" | "cts") => FileKind::Supported(Language::TypeScript),
        Some("tsx") => FileKind::Supported(Language::Tsx),
        Some("rs") => FileKind::Supported(Language::Rust),
        Some("c") => FileKind::Supported(Language::C),
        Some("h" | "hh" | "hpp" | "hxx") => FileKind::Supported(Language::Cpp),
        Some("cc" | "cpp" | "cxx") => FileKind::Supported(Language::Cpp),
        Some("cu" | "cuh") => FileKind::CudaExperimental,
        _ => FileKind::UnsupportedExtension,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fact {
    /// "definition" | "import" | "call" (call = unresolved candidate).
    pub kind: &'static str,
    /// Definition/call name; `None` only when the grammar yields none.
    pub name: Option<String>,
    /// Raw source text for imports; `None` otherwise.
    pub text: Option<String>,
    pub span: Span,
    pub byte_start: usize,
    pub byte_end: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileFacts {
    pub language: Language,
    pub bytes: usize,
    pub definitions: Vec<Fact>,
    pub imports: Vec<Fact>,
    pub calls: Vec<Fact>,
    pub errors: u32,
}

fn ts_language(lang: Language) -> tree_sitter::Language {
    match lang {
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::C => tree_sitter_c::LANGUAGE.into(),
        Language::Cpp => tree_sitter_cpp::LANGUAGE.into(),
    }
}

fn node_text<'a>(node: tree_sitter::Node, src: &'a [u8]) -> Option<&'a str> {
    node.utf8_text(src).ok()
}

fn span_of(node: tree_sitter::Node) -> (Span, usize, usize) {
    let s = node.start_position();
    let e = node.end_position();
    (
        Span {
            start_line: (s.row + 1) as u32,
            start_col: (s.column + 1) as u32,
            end_line: (e.row + 1) as u32,
            end_col: (e.column + 1) as u32,
        },
        node.start_byte(),
        node.end_byte(),
    )
}

/// Name child via the `name` field, else the first identifier-like child.
fn def_name(node: tree_sitter::Node, src: &[u8]) -> Option<String> {
    if let Some(n) = node.child_by_field_name("name") {
        if let Some(t) = node_text(n, src) {
            return Some(t.to_string());
        }
    }
    for i in 0..node.named_child_count() {
        let Some(child) = node.named_child(i) else {
            continue;
        };
        if child.is_named()
            && matches!(
                child.kind(),
                "identifier" | "type_identifier" | "property_identifier"
            )
        {
            if let Some(t) = node_text(child, src) {
                return Some(t.to_string());
            }
        }
    }
    None
}

/// C/C++ declarator chain: `function_declarator`/`pointer`/`parenthesized`
/// wrappers around the real identifier (`field_identifier` for methods).
/// Bounded walk, `None` when anonymous.
fn c_function_name(node: tree_sitter::Node, src: &[u8]) -> Option<String> {
    let mut current = node.child_by_field_name("declarator")?;
    for _ in 0..6 {
        match current.kind() {
            "identifier" | "field_identifier" => {
                return node_text(current, src).map(|s| s.to_string())
            }
            "function_declarator"
            | "parenthesized_declarator"
            | "pointer_declarator"
            | "array_declarator" => {
                current = current.child_by_field_name("declarator").or_else(|| {
                    for i in 0..current.named_child_count() {
                        if let Some(c) = current.named_child(i) {
                            if c.is_named()
                                && matches!(
                                    c.kind(),
                                    "identifier"
                                        | "field_identifier"
                                        | "function_declarator"
                                        | "parenthesized_declarator"
                                        | "pointer_declarator"
                                        | "array_declarator"
                                )
                            {
                                return Some(c);
                            }
                        }
                    }
                    None
                })?;
            }
            _ => return None,
        }
    }
    None
}

fn push_def(facts: &mut FileFacts, kind: &'static str, node: tree_sitter::Node, src: &[u8]) {
    let (span, byte_start, byte_end) = span_of(node);
    facts.definitions.push(Fact {
        kind,
        name: def_name(node, src),
        text: None,
        span,
        byte_start,
        byte_end,
    });
}

fn push_c_def(facts: &mut FileFacts, kind: &'static str, node: tree_sitter::Node, src: &[u8]) {
    let (span, byte_start, byte_end) = span_of(node);
    facts.definitions.push(Fact {
        kind,
        name: c_function_name(node, src),
        text: None,
        span,
        byte_start,
        byte_end,
    });
}

fn push_import(facts: &mut FileFacts, node: tree_sitter::Node, src: &[u8]) {
    // Preprocessor nodes span the trailing newline; trim one so the fact
    // covers the directive itself.
    let mut end = node.end_byte();
    if end > node.start_byte() && src.get(end - 1) == Some(&b'\n') {
        end -= 1;
    }
    let pos = node.start_position();
    let span = Span {
        start_line: (pos.row + 1) as u32,
        start_col: (pos.column + 1) as u32,
        end_line: (pos.row + 1) as u32,
        end_col: (end - line_start(src, end) + 1) as u32,
    };
    facts.imports.push(Fact {
        kind: "import",
        name: None,
        text: std::str::from_utf8(&src[node.start_byte()..end])
            .ok()
            .map(|s| s.trim().to_string()),
        span,
        byte_start: node.start_byte(),
        byte_end: end,
    });
}

fn line_start(src: &[u8], end: usize) -> usize {
    let mut start = end.min(src.len());
    while start > 0 && src[start - 1] != b'\n' {
        start -= 1;
    }
    start
}

/// Named `struct X {...}` or anonymous `typedef struct {...} X` (name from the
/// enclosing `type_definition` declarator, span of the whole declaration).
fn push_struct(facts: &mut FileFacts, node: tree_sitter::Node, src: &[u8]) {
    if node.child_by_field_name("name").is_some() {
        push_def(facts, "struct", node, src);
        return;
    }
    let Some(parent) = node.parent() else {
        return;
    };
    if parent.kind() != "type_definition" {
        return;
    }
    let name = parent
        .child_by_field_name("declarator")
        .filter(|d| matches!(d.kind(), "identifier" | "type_identifier"))
        .and_then(|d| node_text(d, src).map(|s| s.to_string()));
    let (span, byte_start, byte_end) = span_of(parent);
    facts.definitions.push(Fact {
        kind: "struct",
        name,
        text: None,
        span,
        byte_start,
        byte_end,
    });
}

fn push_call(facts: &mut FileFacts, node: tree_sitter::Node, src: &[u8]) {
    let callee = node
        .child_by_field_name("function")
        .and_then(|f| node_text(f, src).map(|s| s.to_string()));
    let (span, byte_start, byte_end) = span_of(node);
    facts.calls.push(Fact {
        kind: "call",
        name: callee,
        text: None,
        span,
        byte_start,
        byte_end,
    });
}

fn visit(lang: Language, node: tree_sitter::Node, src: &[u8], facts: &mut FileFacts) {
    // Missing tokens (e.g. a `)` the parser inserts) are anonymous: count
    // every child, named or not, or recovery evidence goes missing.
    if node.is_missing() || node.kind() == "ERROR" {
        facts.errors += 1;
    }
    match lang {
        Language::Python => match node.kind() {
            "function_definition" => push_def(facts, "function", node, src),
            "class_definition" => push_def(facts, "class", node, src),
            "import_statement" | "import_from_statement" => push_import(facts, node, src),
            "call" => push_call(facts, node, src),
            _ => {}
        },
        Language::JavaScript => match node.kind() {
            "function_declaration" => push_def(facts, "function", node, src),
            "method_definition" => push_def(facts, "method", node, src),
            "class_declaration" => push_def(facts, "class", node, src),
            "import_statement" => push_import(facts, node, src),
            "call_expression" => push_call(facts, node, src),
            _ => {}
        },
        Language::TypeScript | Language::Tsx => match node.kind() {
            "function_declaration" => push_def(facts, "function", node, src),
            "method_definition" => push_def(facts, "method", node, src),
            "class_declaration" => push_def(facts, "class", node, src),
            "interface_declaration" => push_def(facts, "interface", node, src),
            "import_statement" => push_import(facts, node, src),
            "call_expression" => push_call(facts, node, src),
            _ => {}
        },
        Language::Rust => match node.kind() {
            "function_item" => push_def(facts, "function", node, src),
            "struct_item" => push_def(facts, "struct", node, src),
            "use_declaration" => push_import(facts, node, src),
            "call_expression" => push_call(facts, node, src),
            _ => {}
        },
        Language::C => match node.kind() {
            "function_definition" => push_c_def(facts, "function", node, src),
            "struct_specifier" => push_struct(facts, node, src),
            "preproc_include" => push_import(facts, node, src),
            "call_expression" => push_call(facts, node, src),
            _ => {}
        },
        Language::Cpp => match node.kind() {
            "function_definition" => push_c_def(facts, "function", node, src),
            "struct_specifier" => push_struct(facts, node, src),
            "class_specifier" => push_def(facts, "class", node, src),
            "preproc_include" => push_import(facts, node, src),
            "call_expression" => push_call(facts, node, src),
            _ => {}
        },
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            visit(lang, child, src, facts);
        }
    }
}

/// Parse UTF-8 source. Tree-sitter always produces a tree (error recovery);
/// syntax problems surface as `errors`, never as a failure.
pub fn parse_source(lang: Language, src: &[u8]) -> FileFacts {
    let mut facts = FileFacts {
        language: lang,
        bytes: src.len(),
        definitions: Vec::new(),
        imports: Vec::new(),
        calls: Vec::new(),
        errors: 0,
    };
    let mut parser = tree_sitter::Parser::new();
    let language = ts_language(lang);
    if parser.set_language(&language).is_err() {
        facts.errors += 1;
        return facts;
    }
    if let Some(tree) = parser.parse(src, None) {
        visit(lang, tree.root_node(), src, &mut facts);
    } else {
        facts.errors += 1;
    }
    facts
}

fn span_json(span: &Span) -> String {
    format!(
        "{{\"start_line\":{},\"start_col\":{},\"end_line\":{},\"end_col\":{}}}",
        span.start_line, span.start_col, span.end_line, span.end_col
    )
}

fn opt_json(value: &Option<String>) -> String {
    match value {
        Some(s) => format!("\"{}\"", escape(s)),
        None => "null".to_string(),
    }
}

fn fact_json(f: &Fact) -> String {
    format!(
        "{{\"kind\":\"{}\",\"name\":{},\"text\":{},\"span\":{},\"bytes\":[{},{}]}}",
        f.kind,
        opt_json(&f.name),
        opt_json(&f.text),
        span_json(&f.span),
        f.byte_start,
        f.byte_end
    )
}

fn facts_json(facts: &[Fact]) -> String {
    facts.iter().map(fact_json).collect::<Vec<_>>().join(",")
}

/// File entry with relative slash-separated path. Fixed key order.
pub fn file_facts_json(rel: &str, facts: &FileFacts) -> String {
    format!(
        "{{\"path\":\"{}\",\"status\":\"ok\",\"language\":\"{}\",\"bytes\":{},\"definitions\":[{}],\"imports\":[{}],\"calls\":[{}],\"errors\":{}}}",
        escape(rel),
        language_name(facts.language),
        facts.bytes,
        facts_json(&facts.definitions),
        facts_json(&facts.imports),
        facts_json(&facts.calls),
        facts.errors
    )
}

#[cfg(test)]
mod tests {
    use super::{classify, language_name, parse_source, FileKind, Language};
    use std::path::Path;

    #[test]
    fn maps_extensions_to_languages() {
        assert_eq!(
            classify(Path::new("a.py")),
            FileKind::Supported(Language::Python)
        );
        assert_eq!(
            classify(Path::new("a.js")),
            FileKind::Supported(Language::JavaScript)
        );
        assert_eq!(
            classify(Path::new("a.ts")),
            FileKind::Supported(Language::TypeScript)
        );
        assert_eq!(
            classify(Path::new("a.tsx")),
            FileKind::Supported(Language::Tsx)
        );
        assert_eq!(
            classify(Path::new("a.rs")),
            FileKind::Supported(Language::Rust)
        );
        assert_eq!(classify(Path::new("a.c")), FileKind::Supported(Language::C));
        assert_eq!(
            classify(Path::new("a.h")),
            FileKind::Supported(Language::Cpp)
        );
        assert_eq!(
            classify(Path::new("a.cpp")),
            FileKind::Supported(Language::Cpp)
        );
        assert_eq!(
            classify(Path::new("A.PY")),
            FileKind::Supported(Language::Python)
        );
    }

    #[test]
    fn refuses_cuda_and_unknown_extensions() {
        assert_eq!(classify(Path::new("k.cu")), FileKind::CudaExperimental);
        assert_eq!(classify(Path::new("k.cuh")), FileKind::CudaExperimental);
        assert_eq!(
            classify(Path::new("note.xyz")),
            FileKind::UnsupportedExtension
        );
        assert_eq!(
            classify(Path::new("Makefile")),
            FileKind::UnsupportedExtension
        );
    }

    #[test]
    fn extracts_python_facts_with_spans() {
        let src = b"import os\n\n\ndef add(a, b):\n    return calc(a)\n";
        let facts = parse_source(Language::Python, src);
        assert_eq!(facts.errors, 0);
        assert_eq!(facts.definitions.len(), 1);
        assert_eq!(facts.definitions[0].name.as_deref(), Some("add"));
        assert_eq!(facts.definitions[0].span.start_line, 4);
        assert_eq!(facts.imports.len(), 1);
        assert_eq!(facts.calls.len(), 1);
        assert_eq!(facts.calls[0].name.as_deref(), Some("calc"));
    }

    #[test]
    fn counts_syntax_errors_without_failing() {
        let src = b"def broken(:\n    pass\n";
        let facts = parse_source(Language::Python, src);
        assert!(facts.errors > 0, "malformed input must surface errors");
    }

    #[test]
    fn language_names_are_stable() {
        assert_eq!(language_name(Language::Cpp), "cpp");
        assert_eq!(language_name(Language::Tsx), "tsx");
    }
}
