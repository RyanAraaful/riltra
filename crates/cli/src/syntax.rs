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

/// C++ data member: a `field_declaration` whose declarator ends in a
/// `field_identifier`. The fact points at the NAME (the identifier), so the
/// anchor of a member is its identifier, not its type.
///
/// A `function_declarator` is never a data member: a method declaration, a
/// constructor and the ambiguous function-pointer form
/// (`int (*target)();`) are all skipped rather than guessed.
fn push_field(facts: &mut FileFacts, node: tree_sitter::Node, src: &[u8]) {
    let mut cursor = node.walk();
    for declarator in node.children_by_field_name("declarator", &mut cursor) {
        let Some(name_node) = field_identifier_of(declarator) else {
            continue;
        };
        let (span, byte_start, byte_end) = span_of(name_node);
        facts.definitions.push(Fact {
            kind: "field",
            name: node_text(name_node, src).map(str::to_string),
            text: None,
            span,
            byte_start,
            byte_end,
            callee: None,
            tcallee: None,
        });
    }
}

/// The `field_identifier` at the end of a data-member declarator chain.
/// Pointer, reference, array, parenthesized and attributed declarators wrap
/// it; a `function_declarator` never does, so it yields nothing.
fn field_identifier_of(declarator: tree_sitter::Node) -> Option<tree_sitter::Node> {
    let mut current = declarator;
    for _ in 0..6 {
        match current.kind() {
            "field_identifier" => return Some(current),
            "function_declarator" => return None,
            "pointer_declarator"
            | "reference_declarator"
            | "array_declarator"
            | "parenthesized_declarator"
            | "attributed_declarator" => {
                current = inner_declarator(current)?;
            }
            _ => return None,
        }
    }
    None
}

/// The declarator inside a wrapper node: its `declarator` field when it has
/// one, else the first named child that can carry it (the reference and
/// parenthesized declarators have no field of their own).
fn inner_declarator(node: tree_sitter::Node) -> Option<tree_sitter::Node> {
    if let Some(inner) = node.child_by_field_name("declarator") {
        return Some(inner);
    }
    for i in 0..node.named_child_count() {
        let Some(child) = node.named_child(i) else {
            continue;
        };
        if matches!(
            child.kind(),
            "field_identifier"
                | "function_declarator"
                | "pointer_declarator"
                | "reference_declarator"
                | "array_declarator"
                | "parenthesized_declarator"
                | "attributed_declarator"
        ) {
            return Some(child);
        }
    }
    None
}

fn push_call(facts: &mut FileFacts, node: tree_sitter::Node, src: &[u8]) {
    let func = node.child_by_field_name("function");
    let callee = func.and_then(|f| node_text(f, src).map(|s| s.to_string()));
    let (span, byte_start, byte_end) = span_of(node);
    facts.calls.push(Fact {
        kind: "call",
        name: callee,
        text: None,
        span,
        byte_start,
        byte_end,
        callee: func.map(callee_span_of),
        tcallee: func.and_then(|f| terminal_span(src, f)),
    });
}

/// Span and byte range of a node, as a call fact records its callee.
fn callee_span_of(node: tree_sitter::Node) -> CalleeSpan {
    let (span, byte_start, byte_end) = span_of(node);
    CalleeSpan {
        start_line: span.start_line,
        start_col: span.start_col,
        end_line: span.end_line,
        end_col: span.end_col,
        byte_start,
        byte_end,
    }
}

/// C++ direct initialization (`T var(args);`) has no `call_expression` node:
/// the grammar models it as `declaration` -> `init_declarator` ->
/// `argument_list`, with the called type in the declaration's type field.
/// Only that form is a constructor call: the argument list holds
/// expressions, so C++ reads the declaration as a variable definition.
///
/// The grammar also models `T var(a, b);` as a `function_declarator`
/// (C++'s most-vexing-parse shape). There the arguments are read as a
/// parameter-type-list, and C++ reads the declaration as a function
/// declaration when the argument names denote types and as a variable
/// definition when they denote variables. Only name lookup can decide, so
/// the syntax layer must not assert a constructor call for that form: a
/// block-scope function declaration is legal C++, so the scope does not
/// settle it either. Returns the called type node and the argument list.
fn direct_init_parts(node: tree_sitter::Node) -> Option<(tree_sitter::Node, tree_sitter::Node)> {
    let decl = node.child_by_field_name("declarator")?;
    // Direct initialization puts the argument list in the declarator's
    // `value` field; copy initialization (`T v = T(args);`) puts a
    // `call_expression` there, which is captured on its own.
    if decl.kind() != "init_declarator" {
        return None;
    }
    let value = decl.child_by_field_name("value")?;
    if value.kind() != "argument_list" {
        return None;
    }
    let ty = node.child_by_field_name("type")?;
    // Only a named type can be a constructor: `int x(5);` is not a call.
    // shortcut: a typedef is indistinguishable from a class here, so
    // `size_t n(0);` is recorded as a call to `size_t` although it may be
    // a functional cast; a name lookup would settle it (gap E12).
    if !matches!(ty.kind(), "type_identifier" | "qualified_identifier") {
        return None;
    }
    Some((ty, value))
}

/// Record a C++ direct-initialization constructor call as a call fact. The
/// call range runs from the called type to the end of the argument list
/// (`Widget w(1)`), the callee is the type (`Widget`), and the terminal is
/// the type's last name part.
fn push_direct_init_call(facts: &mut FileFacts, node: tree_sitter::Node, src: &[u8]) {
    let Some((ty, args)) = direct_init_parts(node) else {
        return;
    };
    let (ty_span, ty_start, _) = span_of(ty);
    let (args_span, _, args_end) = span_of(args);
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
        callee: Some(callee_span_of(ty)),
        tcallee: terminal_span(src, ty),
    });
}

/// Name node of a callee whose last name carries a template argument list:
/// `make_shared` in `std::make_shared<T>`, `get` in `obj.get<T>()` and
/// `ptr->template get<T>()`, `vector` in `std::vector<T> v(n)`. `None` for
/// every other callee shape, including template arguments that only appear in
/// a scope (`Foo<T>::create`).
fn template_name(callee: tree_sitter::Node) -> Option<tree_sitter::Node> {
    let mut node = callee;
    loop {
        match node.kind() {
            "template_function" | "template_method" | "template_type" => {
                return node.child_by_field_name("name");
            }
            "qualified_identifier" => node = node.child_by_field_name("name")?,
            "field_expression" => node = node.child_by_field_name("field")?,
            "dependent_name" => node = node.named_child(0)?,
            _ => return None,
        }
    }
}

/// Terminal identifier/property of a callee expression. A callee whose last
/// name has template arguments takes its terminal from the grammar: the text
/// after a separator would start inside the arguments (`PatchMatchController>`
/// for `std::make_shared<mvs::PatchMatchController>`) and the arguments are
/// not part of the called name. Every other callee takes the text after the
/// last `.`, `::`, or `->` separator. Separators are ASCII, so byte
/// arithmetic on the UTF-8 source is exact. Returns `None` when the callee
/// ends in a separator or is not valid UTF-8; a plain `foo` is its own
/// terminal.
fn terminal_span(src: &[u8], callee: tree_sitter::Node) -> Option<CalleeSpan> {
    if let Some(name) = template_name(callee) {
        return Some(callee_span_of(name));
    }
    let (callee_bs, callee_be) = (callee.start_byte(), callee.end_byte());
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
            "field_declaration" => push_field(facts, node, src),
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

    /// The C++ data members of a source: (name, start_line, start_col, text).
    fn cpp_fields(src: &[u8]) -> Vec<(String, u32, u32, String)> {
        parse_source(Language::Cpp, src)
            .definitions
            .iter()
            .filter(|d| d.kind == "field")
            .map(|d| {
                (
                    d.name.clone().unwrap_or_default(),
                    d.span.start_line,
                    d.span.start_col,
                    String::from_utf8_lossy(&src[d.byte_start..d.byte_end]).into_owned(),
                )
            })
            .collect()
    }

    #[test]
    fn extracts_a_cpp_data_member_as_a_field() {
        // A data member points at its NAME, so the anchor is the identifier
        // (the frozen CP1 anchor `patch_match_options.h:56:10`).
        let src = b"class Options {\n  double max_triangulation_angle = -1.0f;\n};\n";
        let fields = cpp_fields(src);
        assert_eq!(fields.len(), 1, "{fields:?}");
        assert_eq!(fields[0].0, "max_triangulation_angle");
        assert_eq!((fields[0].1, fields[0].2), (2, 10));
        assert_eq!(fields[0].3, "max_triangulation_angle");
    }

    #[test]
    fn a_cpp_field_with_an_initializer_is_a_field() {
        let src = b"struct Holder { int value = 1; };\n";
        let fields = cpp_fields(src);
        assert_eq!(fields.len(), 1, "{fields:?}");
        assert_eq!(fields[0].0, "value");
        assert_eq!(fields[0].3, "value");
    }

    #[test]
    fn multiple_cpp_declarators_are_separate_fields() {
        let src = b"struct S { int a, b; };\n";
        let fields = cpp_fields(src);
        let names: Vec<&str> = fields.iter().map(|f| f.0.as_str()).collect();
        assert_eq!(names, vec!["a", "b"], "{fields:?}");
    }

    #[test]
    fn pointer_reference_and_array_members_are_fields() {
        let src = b"struct S {\n  double* p;\n  double& r;\n  double arr[4];\n};\n";
        let fields = cpp_fields(src);
        let names: Vec<&str> = fields.iter().map(|f| f.0.as_str()).collect();
        assert_eq!(names, vec!["p", "r", "arr"], "{fields:?}");
    }

    #[test]
    fn a_cpp_method_declaration_is_not_a_field() {
        // A method declaration is a field_declaration node whose declarator
        // is a function declarator: it is never a data member.
        let src = b"class W {\n public:\n  int process(int x);\n};\n";
        assert!(cpp_fields(src).is_empty(), "a method is not a data member");
    }

    #[test]
    fn a_cpp_function_pointer_member_is_not_a_field() {
        // `int (*target)();` is a member, but its declarator is a function
        // declarator: the ambiguous form is not indexed as a data member.
        let src = b"struct Obj { int (*target)(); };\n";
        assert!(
            cpp_fields(src).is_empty(),
            "a function-pointer declarator is not indexed"
        );
    }

    #[test]
    fn same_named_cpp_fields_stay_separate_symbols() {
        let src = b"struct A { int value; };\nstruct B { int value; };\n";
        let fields = cpp_fields(src);
        assert_eq!(fields.len(), 2, "{fields:?}");
        assert_eq!(fields[0].0, "value");
        assert_eq!(fields[1].0, "value");
        assert_ne!(fields[0].1, fields[1].1, "distinct declarations");
    }

    #[test]
    fn qualified_and_derived_cpp_members_are_fields() {
        // A static, a mutable, a bitfield and a member of an anonymous union
        // are data members; a template class member is one too.
        let src = b"class C {\n public:\n  static int counter;\n  mutable double ratio;\n  unsigned int flags : 3;\n  union { int raw; float as_float; };\n};\n";
        let fields = cpp_fields(src);
        let names: Vec<&str> = fields.iter().map(|f| f.0.as_str()).collect();
        assert_eq!(
            names,
            vec!["counter", "ratio", "flags", "raw", "as_float"],
            "{fields:?}"
        );
        // One row per declaration: a member is never indexed twice.
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), names.len(), "no duplicate field rows");
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
    fn cpp_most_vexing_parse_is_ambiguous_and_not_a_call() {
        // `T v(a, b);` with identifier-like arguments is parsed by the
        // grammar as a function declarator. C++ reads it as a function
        // declaration when `a`/`b` denote types and as a variable
        // definition when they denote variables; only name lookup can
        // decide, so the syntax layer must not assert a constructor call.
        let src = b"void caller() {\n  Widget w(options, problem);\n}\n";
        let facts = parse_source(Language::Cpp, src);
        assert!(
            facts.calls.is_empty(),
            "an ambiguous most-vexing-parse form must not become a call fact"
        );
    }

    #[test]
    fn cpp_block_scope_type_argument_is_not_a_call() {
        // `Widget w(int);` inside a function body declares the function
        // `w` taking an int (MSVC: the type of `w` is `Widget(int)`), not
        // a variable: a function declaration is legal C++ at block scope,
        // so the declaration is not a constructor call.
        let src = b"void caller() {\n  Widget w(int);\n}\n";
        let facts = parse_source(Language::Cpp, src);
        assert!(
            facts.calls.is_empty(),
            "a block-scope function declaration is not a constructor call"
        );
    }

    #[test]
    fn cpp_block_scope_function_declaration_is_not_a_call() {
        // The same shape with a named return type is the canonical
        // block-scope function declaration, which C++ accepts.
        let src = b"void caller() {\n  Widget helper(int);\n}\n";
        let facts = parse_source(Language::Cpp, src);
        assert!(
            facts.calls.is_empty(),
            "a block-scope function declaration is not a constructor call"
        );
    }

    #[test]
    fn cpp_block_scope_empty_parentheses_is_not_a_call() {
        // `Widget w();` declares the function `w` (MSVC: the type of `w` is
        // `Widget(void)`), never a default-constructed variable.
        let src = b"void caller() {\n  Widget w();\n}\n";
        let facts = parse_source(Language::Cpp, src);
        assert!(
            facts.calls.is_empty(),
            "an empty argument list declares a function, not a call"
        );
    }

    #[test]
    fn cpp_block_scope_function_type_argument_is_not_a_call() {
        // `Widget w(T());` is the classic most-vexing parse: `T()` is a
        // function type, so the declaration is a function declaration
        // (MSVC: the type of `w` is `Widget(T(void))`).
        let src = b"struct T {};\nvoid caller() {\n  Widget w(T());\n}\n";
        let facts = parse_source(Language::Cpp, src);
        assert!(
            facts.calls.is_empty(),
            "a function-type argument declares a function, not a call"
        );
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
        // still local, so a direct initialization is a constructor call.
        let src = b"void caller() {\n#if defined(X)\n  Widget w(1);\n#endif\n}\n";
        let facts = parse_source(Language::Cpp, src);
        assert_eq!(facts.calls.len(), 1);
        assert_eq!(facts.calls[0].name.as_deref(), Some("Widget"));
    }

    #[test]
    fn cpp_direct_initialization_records_the_first_declarator_only() {
        // Known limit: a declaration with several declarators yields the
        // fact for the first one only. The omission is conservative - a
        // missing fact, never a wrong one.
        let src = b"void caller() {\n  Widget a(1), b(2);\n}\n";
        let facts = parse_source(Language::Cpp, src);
        assert_eq!(
            facts.calls.len(),
            1,
            "only the first declarator of a multi-declarator declaration is recorded"
        );
    }

    #[test]
    fn cpp_template_type_direct_initialization_is_a_call() {
        // A template type is a qualified_identifier in the type field, so
        // the call is recorded. The callee stays as written; the terminal is
        // the template name without its arguments.
        let src = b"void caller() {\n  std::vector<int> v(10);\n}\n";
        let facts = parse_source(Language::Cpp, src);
        assert_eq!(facts.calls.len(), 1);
        let call = &facts.calls[0];
        assert_eq!(call.name.as_deref(), Some("std::vector<int>"));
        let callee = call.callee.as_ref().expect("callee span recorded");
        assert_eq!(
            &src[callee.byte_start..callee.byte_end],
            b"std::vector<int>"
        );
        assert_eq!(terminal_text(src, call), "vector");
    }

    #[test]
    fn cpp_template_type_direct_initialization_terminal_ignores_argument_separators() {
        let src = b"void caller() {\n  std::vector<ns::Item> items(10);\n}\n";
        let facts = parse_source(Language::Cpp, src);
        assert_eq!(facts.calls.len(), 1);
        assert_eq!(terminal_text(src, &facts.calls[0]), "vector");
    }

    /// Text covered by the terminal span of a call fact.
    fn terminal_text<'a>(src: &'a [u8], call: &super::Fact) -> &'a str {
        let terminal = call.tcallee.as_ref().expect("terminal span recorded");
        std::str::from_utf8(&src[terminal.byte_start..terminal.byte_end]).unwrap()
    }

    /// Terminal of the single call in `void caller() { <expr>; }`.
    fn cpp_terminal_of(expr: &str) -> String {
        let src = format!("void caller() {{\n  {expr};\n}}\n");
        let facts = parse_source(Language::Cpp, src.as_bytes());
        assert_eq!(facts.calls.len(), 1, "one call in {expr:?}");
        terminal_text(src.as_bytes(), &facts.calls[0]).to_string()
    }

    #[test]
    fn cpp_template_call_terminal_is_the_function_not_the_type_argument() {
        // `mvs::PatchMatchController` is a template type argument of the
        // called function `make_shared`, not part of the called name. The
        // callee stays as written; the terminal is the function name.
        let src = b"void caller() {\n  std::make_shared<mvs::PatchMatchController>(options);\n}\n";
        let facts = parse_source(Language::Cpp, src);
        assert_eq!(facts.calls.len(), 1);
        let call = &facts.calls[0];
        assert_eq!(
            call.name.as_deref(),
            Some("std::make_shared<mvs::PatchMatchController>")
        );
        let callee = call.callee.as_ref().expect("callee span recorded");
        assert_eq!(
            &src[callee.byte_start..callee.byte_end],
            b"std::make_shared<mvs::PatchMatchController>"
        );
        assert_eq!(terminal_text(src, call), "make_shared");
        // The line/column pair agrees with the byte range (1-based).
        let terminal = call.tcallee.as_ref().expect("terminal span recorded");
        assert_eq!(
            (
                terminal.start_line,
                terminal.start_col,
                terminal.end_line,
                terminal.end_col
            ),
            (2, 8, 2, 19)
        );
    }

    #[test]
    fn cpp_template_call_terminal_excludes_every_template_argument_list() {
        // The terminal is the called name for every shape that carries a
        // template argument list, whatever the arguments contain.
        let cases = [
            (
                "std::make_unique<std::vector<std::string>>(n)",
                "make_unique",
            ),
            ("make<ns::Thing>(x)", "make"),
            ("std::get<0>(t)", "get"),
            ("obj.get<ns::Thing>()", "get"),
            ("ptr->get<ns::Thing>()", "get"),
            ("ptr->template get<ns::Thing>()", "get"),
            ("Foo<ns::T>::template create<ns::U>(x)", "create"),
        ];
        for (expr, terminal) in cases {
            assert_eq!(cpp_terminal_of(expr), terminal, "terminal of {expr:?}");
        }
    }

    #[test]
    fn cpp_call_terminal_is_unchanged_without_a_template_argument_list() {
        // Calls that were already identified correctly keep their terminal,
        // including template arguments that only appear in a scope.
        let cases = [
            ("foo(1)", "foo"),
            ("ns::inner::run(x)", "run"),
            ("a.b.c(x)", "c"),
            ("ptr->run(x)", "run"),
            ("Foo<ns::T>::create(x)", "create"),
            ("ns::Foo<ns::Bar>::Baz<ns::Qux>::run(x)", "run"),
        ];
        for (expr, terminal) in cases {
            assert_eq!(cpp_terminal_of(expr), terminal, "terminal of {expr:?}");
        }
    }

    #[test]
    fn cpp_unqualified_template_call_has_no_receiver() {
        // Nothing precedes the called name, so the terminal starts where the
        // callee starts (the graph reads this as "no receiver").
        let src = b"void caller() {\n  make<ns::Thing>(x);\n}\n";
        let facts = parse_source(Language::Cpp, src);
        let call = &facts.calls[0];
        let callee = call.callee.as_ref().expect("callee span recorded");
        let terminal = call.tcallee.as_ref().expect("terminal span recorded");
        assert_eq!(terminal.byte_start, callee.byte_start);
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
