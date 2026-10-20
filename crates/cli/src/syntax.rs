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

/// Grammar crate version backing a language, matching `store::grammar_versions`
/// and `Cargo.lock` (pinned by test on both sides).
pub fn grammar_version(lang: Language) -> &'static str {
    match lang {
        Language::Python => "python:0.25.0",
        Language::JavaScript => "javascript:0.25.0",
        Language::TypeScript => "typescript:0.23.2",
        Language::Tsx => "tsx:0.23.2",
        Language::Rust => "rust:0.24.2",
        Language::C => "c:0.24.2",
        Language::Cpp => "cpp:0.23.4",
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
pub struct CalleeSpan {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub byte_start: usize,
    pub byte_end: usize,
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
    /// Call-callee expression range (`function` field); `None` for defs,
    /// imports, and calls whose grammar yields no function field.
    pub callee: Option<CalleeSpan>,
    /// Terminal identifier/property within `callee` (receiver split off);
    /// `None` when inseparable or absent.
    pub tcallee: Option<CalleeSpan>,
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
/// wrappers around the real identifier (`field_identifier` for methods,
/// `qualified_identifier` for `Scope::name` definitions, kept verbatim as an
/// unresolved candidate). Bounded walk, `None` when anonymous.
fn c_function_name(node: tree_sitter::Node, src: &[u8]) -> Option<String> {
    c_name_node(node).and_then(|n| node_text(n, src).map(str::to_string))
}

fn c_name_node(node: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
    let mut current = node.child_by_field_name("declarator")?;
    for _ in 0..6 {
        match current.kind() {
            "identifier" | "field_identifier" | "qualified_identifier" => return Some(current),
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
                                        | "qualified_identifier"
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

/// Extra validation for relationship derivation, never new syntax facts.
/// Whole-function ranges cannot identify a declaration token or distinguish
/// a nested anonymous scope. All ranges index the digest-checked source.
#[derive(Default)]
pub struct RelationshipProof {
    pub declarations: Vec<(usize, usize)>,
    pub owners: std::collections::BTreeMap<(usize, usize), Option<(usize, usize)>>,
}

pub fn relationship_proof(lang: Language, src: &[u8]) -> RelationshipProof {
    fn callable(node: tree_sitter::Node<'_>) -> bool {
        matches!(
            node.kind(),
            "function_definition" | "function_declaration" | "function_item" | "method_definition"
        )
    }
    fn name(node: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
        node.child_by_field_name("name")
            .or_else(|| c_name_node(node))
    }
    /// Nearest enclosing callable definition of `node`, if any.
    fn enclosing_owner(node: tree_sitter::Node<'_>) -> Option<(usize, usize)> {
        let mut parent = node.parent();
        while let Some(p) = parent {
            if callable(p) && name(p).is_some() {
                return Some((p.start_byte(), p.end_byte()));
            }
            if matches!(
                p.kind(),
                "lambda"
                    | "lambda_expression"
                    | "arrow_function"
                    | "closure_expression"
                    | "function_expression"
                    | "generator_function"
                    | "class_definition"
                    | "class_declaration"
                    | "class_specifier"
                    | "struct_specifier"
            ) {
                return None;
            }
            parent = p.parent();
        }
        None
    }
    let mut proof = RelationshipProof::default();
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&ts_language(lang)).is_err() {
        return proof;
    }
    let Some(tree) = parser.parse(src, None) else {
        return proof;
    };
    if tree.root_node().has_error() {
        return proof;
    }
    let mut pending = vec![tree.root_node()];
    while let Some(node) = pending.pop() {
        if callable(node) {
            if let Some(n) = name(node) {
                proof.declarations.push((n.start_byte(), n.end_byte()));
            }
        }
        if matches!(node.kind(), "call" | "call_expression") {
            proof
                .owners
                .insert((node.start_byte(), node.end_byte()), enclosing_owner(node));
        }
        // A C++ direct-initialization call has no `call_expression` node, so
        // its owner is recorded against the same range the call fact uses.
        if node.kind() == "declaration" {
            if let Some((ty, args)) = direct_init_parts(node) {
                proof
                    .owners
                    .insert((ty.start_byte(), args.end_byte()), enclosing_owner(node));
            }
        }
        for i in (0..node.named_child_count()).rev() {
            if let Some(child) = node.named_child(i) {
                pending.push(child);
            }
        }
    }
    proof
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
        callee: None,
        tcallee: None,
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
        callee: None,
        tcallee: None,
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
        callee: None,
        tcallee: None,
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
        callee: None,
        tcallee: None,
    });
}

fn push_call(facts: &mut FileFacts, node: tree_sitter::Node, src: &[u8]) {
    let func = node.child_by_field_name("function");
    let callee = func.and_then(|f| node_text(f, src).map(|s| s.to_string()));
    let (span, byte_start, byte_end) = span_of(node);
    let (callee_span, terminal_span) = match func {
        Some(f) => {
            let (cspan, cbs, cbe) = span_of(f);
            let callee_span = CalleeSpan {
                start_line: cspan.start_line,
                start_col: cspan.start_col,
                end_line: cspan.end_line,
                end_col: cspan.end_col,
                byte_start: cbs,
                byte_end: cbe,
            };
            (Some(callee_span), terminal_span(src, cbs, cbe))
        }
        None => (None, None),
    };
    facts.calls.push(Fact {
        kind: "call",
        name: callee,
        text: None,
        span,
        byte_start,
        byte_end,
        callee: callee_span,
        tcallee: terminal_span,
    });
}

/// Whether `node` sits inside a function or lambda body. A declaration in
/// such a scope is a local variable, never a function declaration: C++ does
/// not allow declaring a function inside a block. Class bodies and
/// namespace scope are excluded, and a preprocessor branch does not change
/// the scope.
fn inside_function_body(node: tree_sitter::Node) -> bool {
    let mut parent = node.parent();
    while let Some(p) = parent {
        match p.kind() {
            "compound_statement" => {}
            "function_definition" | "lambda_expression" | "lambda" => return true,
            "field_declaration_list"
            | "class_specifier"
            | "struct_specifier"
            | "namespace_definition"
            | "translation_unit" => return false,
            _ => {}
        }
        parent = p.parent();
    }
    false
}

/// C++ direct initialization (`T var(args);`) has no `call_expression` node:
/// the grammar models it as `declaration` -> `init_declarator` ->
/// `argument_list`, with the called type in the declaration's type field.
/// The grammar also applies C++'s most-vexing-parse rule: when every
/// argument looks like a type, `T var(a, b);` is parsed as a
/// `function_declarator` (a function prototype) instead. Inside a function
/// body a function declaration is not legal C++, so that form is a
/// constructor call too. Returns the called type node and the argument
/// list node.
fn direct_init_parts(node: tree_sitter::Node) -> Option<(tree_sitter::Node, tree_sitter::Node)> {
    let decl = node.child_by_field_name("declarator")?;
    let args = match decl.kind() {
        // Direct initialization puts the argument list in the declarator's
        // `value` field; copy initialization (`T v = T(args);`) puts a
        // `call_expression` there, which is captured on its own.
        "init_declarator" => {
            let value = decl.child_by_field_name("value")?;
            if value.kind() != "argument_list" {
                return None;
            }
            value
        }
        // Most-vexing-parse form: only a local declaration, never a
        // prototype, because a function cannot be declared in a block.
        "function_declarator" => {
            if !inside_function_body(node) {
                return None;
            }
            let mut walker = decl.walk();
            let list = decl
                .named_children(&mut walker)
                .find(|c| c.kind() == "parameter_list")?;
            list
        }
        _ => return None,
    };
    let ty = node.child_by_field_name("type")?;
    // Only a named type can be a constructor: `int x(5);` is not a call.
    if !matches!(ty.kind(), "type_identifier" | "qualified_identifier") {
        return None;
    }
    Some((ty, args))
}

/// Record a C++ direct-initialization constructor call as a call fact. The
/// call range runs from the called type to the end of the argument list
/// (`Widget w(1)`), the callee is the type (`Widget`), and the terminal is
/// the type's last name part.
fn push_direct_init_call(facts: &mut FileFacts, node: tree_sitter::Node, src: &[u8]) {
    let Some((ty, args)) = direct_init_parts(node) else {
        return;
    };
    let (ty_span, ty_start, ty_end) = span_of(ty);
    let (args_span, _, args_end) = span_of(args);
    let callee_span = CalleeSpan {
        start_line: ty_span.start_line,
        start_col: ty_span.start_col,
        end_line: ty_span.end_line,
        end_col: ty_span.end_col,
        byte_start: ty_start,
        byte_end: ty_end,
    };
    let span = Span {
        start_line: ty_span.start_line,
        start_col: ty_span.start_col,
        end_line: args_span.end_line,
        end_col: args_span.end_col,
    };
    facts.calls.push(Fact {
        kind: "call",
        name: node_text(ty, src).map(|s| s.to_string()),
        text: None,
        span,
        byte_start: ty_start,
        byte_end: args_end,
        callee: Some(callee_span),
        tcallee: terminal_span(src, ty_start, ty_end),
    });
}

/// Terminal identifier/property of a callee expression: the text after the
/// last `.`, `::`, or `->` separator. Separators are ASCII, so byte
/// arithmetic on the UTF-8 source is exact. Returns `None` when no
/// separator-split terminal exists (plain `foo`) — callers treat the whole
/// callee as the terminal — or when the range is empty.
fn terminal_span(src: &[u8], callee_bs: usize, callee_be: usize) -> Option<CalleeSpan> {
    let text = std::str::from_utf8(src.get(callee_bs..callee_be)?).ok()?;
    let mut cut: Option<usize> = None;
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'.' {
            cut = Some(i + 1);
            i += 1;
        } else if bytes.get(i..i + 2) == Some(b"::".as_slice())
            || bytes.get(i..i + 2) == Some(b"->".as_slice())
        {
            cut = Some(i + 2);
            i += 2;
        } else {
            i += 1;
        }
    }
    let start = match cut {
        // Plain identifier call: the whole callee IS the terminal.
        None => callee_bs,
        Some(off) => callee_bs + off,
    };
    if start >= callee_be {
        return None;
    }
    let (start_line, start_col) = position_of(src, start);
    let (end_line, end_col) = position_of(src, callee_be);
    Some(CalleeSpan {
        start_line,
        start_col,
        end_line,
        end_col,
        byte_start: start,
        byte_end: callee_be,
    })
}

/// 1-based (line, byte-column) of a byte offset, matching `span_of`.
fn position_of(src: &[u8], off: usize) -> (u32, u32) {
    let off = off.min(src.len());
    let line = src[..off].iter().filter(|b| **b == b'\n').count() as u32 + 1;
    let col = (off
        - src[..off]
            .iter()
            .rposition(|b| *b == b'\n')
            .map(|p| p + 1)
            .unwrap_or(0)) as u32
        + 1;
    (line, col)
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
            "declaration" => push_direct_init_call(facts, node, src),
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
    use super::{classify, grammar_version, language_name, parse_source, FileKind, Language};
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
    fn callee_and_terminal_spans_are_precise() {
        // Whole-call parity: the call span still covers the full expression.
        let src = b"def f():\n    return foo(a, b)\n";
        let facts = parse_source(Language::Python, src);
        assert_eq!(facts.calls.len(), 1);
        let call = &facts.calls[0];
        assert_eq!(&src[call.byte_start..call.byte_end], b"foo(a, b)");
        let callee = call.callee.as_ref().expect("callee span recorded");
        assert_eq!(&src[callee.byte_start..callee.byte_end], b"foo");
        let terminal = call.tcallee.as_ref().expect("terminal span recorded");
        assert_eq!(&src[terminal.byte_start..terminal.byte_end], b"foo");
        // Member call: callee covers the dotted path, terminal only the
        // property — the object-base reference must never prove a call.
        let src = b"def f():\n    return w.process(x)\n";
        let facts = parse_source(Language::Python, src);
        let call = &facts.calls[0];
        let callee = call.callee.as_ref().expect("callee span recorded");
        assert_eq!(&src[callee.byte_start..callee.byte_end], b"w.process");
        let terminal = call.tcallee.as_ref().expect("terminal span recorded");
        assert_eq!(&src[terminal.byte_start..terminal.byte_end], b"process");
        assert!(terminal.byte_start > callee.byte_start);
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

    #[test]
    fn grammar_versions_match_lockfile_entries() {
        assert_eq!(grammar_version(Language::Python), "python:0.25.0");
        assert_eq!(grammar_version(Language::Cpp), "cpp:0.23.4");
    }

    #[test]
    fn cpp_direct_initialization_is_a_call() {
        // C++ has no `call_expression` for `T var(args);`: the grammar models
        // it as declaration -> init_declarator -> argument_list, so the
        // constructor call is invisible unless it is captured explicitly.
        let src = b"struct Widget { Widget(int a); };\nvoid caller() {\n  Widget w(1);\n  Widget w2 = Widget(2);\n}\n";
        let facts = parse_source(Language::Cpp, src);
        let names: Vec<&str> = facts
            .calls
            .iter()
            .map(|c| c.name.as_deref().unwrap_or(""))
            .collect();
        assert_eq!(
            names,
            vec!["Widget", "Widget"],
            "direct initialization and copy initialization are both constructor calls"
        );
        let direct = &facts.calls[0];
        // The call range covers the called type through the argument list.
        assert_eq!(&src[direct.byte_start..direct.byte_end], b"Widget w(1)");
        let callee = direct.callee.as_ref().expect("callee span recorded");
        assert_eq!(&src[callee.byte_start..callee.byte_end], b"Widget");
        let terminal = direct.tcallee.as_ref().expect("terminal span recorded");
        assert_eq!(&src[terminal.byte_start..terminal.byte_end], b"Widget");
    }

    #[test]
    fn cpp_direct_initialization_keeps_the_qualified_terminal() {
        let src = b"void caller() {\n  ns::Widget w(1);\n}\n";
        let facts = parse_source(Language::Cpp, src);
        assert_eq!(facts.calls.len(), 1);
        let call = &facts.calls[0];
        assert_eq!(call.name.as_deref(), Some("ns::Widget"));
        let terminal = call.tcallee.as_ref().expect("terminal span recorded");
        assert_eq!(&src[terminal.byte_start..terminal.byte_end], b"Widget");
    }

    #[test]
    fn cpp_primitive_direct_initialization_is_not_a_call() {
        // `int x(5);` is not a constructor call; a primitive type must not
        // become a call name.
        let src = b"void caller() {\n  int x(5);\n}\n";
        let facts = parse_source(Language::Cpp, src);
        assert!(
            facts.calls.is_empty(),
            "primitive initialization is not a call"
        );
    }

    #[test]
    fn cpp_most_vexing_parse_local_declaration_is_a_call() {
        // `T v(a, b);` with identifier-like arguments is parsed as a
        // function prototype by the grammar. Inside a function body a
        // function declaration is not legal C++, so it is a constructor call.
        let src = b"void caller() {\n  Widget w(options, problem);\n}\n";
        let facts = parse_source(Language::Cpp, src);
        assert_eq!(facts.calls.len(), 1);
        let call = &facts.calls[0];
        assert_eq!(call.name.as_deref(), Some("Widget"));
        let callee = call.callee.as_ref().expect("callee span recorded");
        assert_eq!(&src[callee.byte_start..callee.byte_end], b"Widget");
        let terminal = call.tcallee.as_ref().expect("terminal span recorded");
        assert_eq!(&src[terminal.byte_start..terminal.byte_end], b"Widget");
    }

    #[test]
    fn cpp_prototype_at_namespace_scope_is_not_a_call() {
        // The same shape outside a function body is a prototype, not a call.
        let src = b"Widget w(options, problem);\n";
        let facts = parse_source(Language::Cpp, src);
        assert!(
            facts.calls.is_empty(),
            "a namespace-scope prototype is not a constructor call"
        );
    }

    #[test]
    fn cpp_local_declaration_in_a_preprocessor_branch_is_a_call() {
        // A preprocessor branch does not change the scope: the declaration is
        // still local, so it is a constructor call.
        let src = b"void caller() {\n#if defined(X)\n  Widget w(options, problem);\n#endif\n}\n";
        let facts = parse_source(Language::Cpp, src);
        assert_eq!(facts.calls.len(), 1);
        assert_eq!(facts.calls[0].name.as_deref(), Some("Widget"));
    }

    #[test]
    fn cpp_class_member_declaration_is_not_a_call() {
        // A declaration inside a class body is a member declaration, not a
        // constructor call.
        let src = b"struct Holder { Holder(int a); };\n";
        let facts = parse_source(Language::Cpp, src);
        assert!(
            facts.calls.is_empty(),
            "a class member declaration is not a constructor call"
        );
    }
}
