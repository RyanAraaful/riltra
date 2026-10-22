// SPDX-License-Identifier: Apache-2.0
// E7 Slice 2 evidence probe. Run outside the Riltra repository; archived with its output.
//
// Reads one C++ file, parses it with the SAME tree-sitter C++ grammar the
// product uses (tree-sitter-cpp 0.23.4, tree-sitter 0.25) and reports every
// occurrence of a given identifier with its AST node type, its syntactic role,
// and its enclosing function/class. Read-only: it never writes to the tree.
//
// Usage: e7-slice2-probe <file> <identifier>

use std::env;
use std::fs;
use tree_sitter::{Node, Parser};

fn text<'a>(node: Node, src: &'a [u8]) -> &'a str {
    std::str::from_utf8(&src[node.byte_range()]).unwrap_or("")
}

/// The nearest enclosing named node of one of `kinds`.
fn enclosing<'a>(node: Node<'a>, kinds: &[&str]) -> Option<Node<'a>> {
    let mut cur = node.parent();
    while let Some(p) = cur {
        if kinds.contains(&p.kind()) {
            return Some(p);
        }
        cur = p.parent();
    }
    None
}

/// The enclosing callable and class of a node, as far as the syntax proves.
fn owner<'a>(node: Node<'a>, src: &'a [u8]) -> (Option<String>, Option<String>) {
    let func = enclosing(node, &["function_definition", "function_declaration"])
        .and_then(|f| {
            let mut cur = f.child_by_field_name("declarator")?;
            for _ in 0..6 {
                match cur.kind() {
                    "identifier" | "field_identifier" | "qualified_identifier" => {
                        return Some(text(cur, src).to_string())
                    }
                    "function_declarator"
                    | "parenthesized_declarator"
                    | "pointer_declarator"
                    | "reference_declarator"
                    | "array_declarator" => {
                        cur = cur.child_by_field_name("declarator")?
                    }
                    _ => return None,
                }
            }
            None
        });
    // The class of a member function is not an enclosing node (an
    // out-of-class definition is not nested in its class), so it comes from
    // the qualified name: `PatchMatchOptions::Print` -> `PatchMatchOptions`.
    let class = func.as_ref().and_then(|f| {
        let mut parts = f.rsplit("::");
        let last = parts.next();
        let scope = parts.next();
        match (last, scope) {
            (Some(_), Some(s)) if s != "mvs" && !s.is_empty() => Some(s.to_string()),
            _ => None,
        }
    });
    (func, class)
}

/// The identifier of the declarator chain of a `field_declaration`.
fn declarator_field_id(node: Node) -> Option<Node> {
    let mut cur = node.child_by_field_name("declarator")?;
    for _ in 0..6 {
        match cur.kind() {
            "field_identifier" => return Some(cur),
            "function_declarator" => return None,
            "pointer_declarator"
            | "reference_declarator"
            | "array_declarator"
            | "parenthesized_declarator"
            | "attributed_declarator" => {
                cur = cur.child_by_field_name("declarator")?;
            }
            _ => return None,
        }
    }
    None
}

/// The macro names defined in this file (a `preproc_function_def`).
fn local_macro_names(root: Node, src: &[u8]) -> Vec<String> {
    let mut names = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.kind() == "preproc_function_def" {
            if let Some(n) = node.child_by_field_name("name") {
                names.push(text(n, src).to_string());
            }
        }
        for i in 0..node.child_count() {
            if let Some(c) = node.child(i) {
                stack.push(c);
            }
        }
    }
    names
}

/// The classification of one occurrence.
fn classify(node: Node, src: &[u8], local_macros: &[String]) -> (String, String) {
    let parent = node.parent();
    let Some(parent) = parent else {
        return ("plain identifier".into(), String::new());
    };
    match parent.kind() {
        "field_declaration" => {
            if declarator_field_id(parent).map(|d| d.id()) == Some(node.id()) {
                ("declaration".into(), String::new())
            } else {
                ("field_declaration (other declarator)".into(), String::new())
            }
        }
        "field_expression" => {
            let op = parent
                .child_by_field_name("operator")
                .map(|o| text(o, src).to_string())
                .unwrap_or_default();
            let is_field = parent.child_by_field_name("field").map(|f| f.id()) == Some(node.id());
            if !is_field {
                return ("field expression (object)".into(), format!("operator {op}"));
            }
            // A member access that is the operand of an address-of
            // expression is an address-of binding.
            if let Some(grand) = parent.parent() {
                if grand.kind() == "pointer_expression" {
                    let gop = grand
                        .child_by_field_name("operator")
                        .map(|o| text(o, src).to_string())
                        .unwrap_or_default();
                    if gop == "&" {
                        return (
                            "address-of binding".into(),
                            format!("operator {gop} of a member access ({op})"),
                        );
                    }
                }
            }
            ("member access".into(), format!("operator {op}"))
        }
        "pointer_expression" => {
            let op = parent
                .child_by_field_name("operator")
                .map(|o| text(o, src).to_string())
                .unwrap_or_default();
            let inner = parent
                .child_by_field_name("argument")
                .map(|a| text(a, src).to_string())
                .unwrap_or_default();
            (
                "address-of binding".into(),
                format!("operator {op}, operand {inner}"),
            )
        }
        "argument_list" => {
            let call = parent.parent();
            let callee = call
                .and_then(|c| c.child_by_field_name("function"))
                .map(|f| text(f, src).to_string())
                .unwrap_or_default();
            let is_macro = local_macros.iter().any(|m| m == &callee);
            (
                if is_macro {
                    "macro argument".into()
                } else {
                    "ordinary call argument".into()
                },
                format!("callee {callee}"),
            )
        }
        "qualified_identifier" => ("qualified identifier".into(), String::new()),
        _ => {
            let kind = parent.kind();
            ("plain identifier".into(), format!("in {kind}"))
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = &args[1];
    let needle = &args[2];
    let src = fs::read(path).expect("read file");
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_cpp::LANGUAGE.into())
        .expect("cpp grammar");
    let tree = parser.parse(&src, None).expect("parse");
    let root = tree.root_node();
    let local_macros = local_macro_names(root, &src);

    println!(
        "FILE\t{}\tbytes={}\tparse_errors={}\tlocal_macros={:?}",
        path,
        src.len(),
        root.has_error(),
        local_macros
    );

    let mut stack = vec![root];
    let mut rows: Vec<(u32, u32, String, String, String, String, String, String)> = Vec::new();
    while let Some(node) = stack.pop() {
        if matches!(node.kind(), "identifier" | "field_identifier") && text(node, &src) == needle {
            let pos = node.start_position();
            let (role, detail) = classify(node, &src, &local_macros);
            let (func, class) = owner(node, &src);
            rows.push((
                pos.row as u32 + 1,
                pos.column as u32 + 1,
                node.kind().to_string(),
                role,
                detail,
                func.unwrap_or_else(|| "-".into()),
                class.unwrap_or_else(|| "-".into()),
                text(node, &src).to_string(),
            ));
        }
        if node.kind() == "string_literal" && text(node, &src).contains(needle) {
            let pos = node.start_position();
            rows.push((
                pos.row as u32 + 1,
                pos.column as u32 + 1,
                node.kind().to_string(),
                "string literal".into(),
                text(node, &src).to_string(),
                "-".into(),
                "-".into(),
                needle.clone(),
            ));
        }
        for i in 0..node.child_count() {
            if let Some(c) = node.child(i) {
                stack.push(c);
            }
        }
    }
    rows.sort();
    for (line, col, kind, role, detail, func, class, textv) in rows {
        println!(
            "OCC\t{line}\t{col}\t{kind}\t{role}\t{detail}\tfn={func}\tclass={class}\ttext={textv}"
        );
    }
}
