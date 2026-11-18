//! Names as `capnpc` writes them, and as the command line spells them.
//!
//! Pattern: Strategy — pure functions; the Rust names must be exactly those of capnpc-rust, or the generated code
//! does not compile (the compile test in `capnpc-cli-fixtures` is the proof).

/// Words Rust reserves; `capnpc` appends `_` to a module named like one.
const RUST_KEYWORDS: &[&str] = &[
    "abstract", "alignof", "as", "be", "become", "box", "break", "const", "continue", "crate",
    "do", "else", "enum", "extern", "false", "final", "fn", "for", "if", "impl", "in", "let",
    "loop", "macro", "match", "mod", "move", "mut", "offsetof", "once", "override", "priv", "proc",
    "pub", "pure", "ref", "return", "self", "sizeof", "static", "struct", "super", "trait", "true",
    "type", "typeof", "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// `createRelation` -> `create_relation` (capnpc: an upper case letter after the first gets a `_` before it).
#[must_use]
pub fn snake(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

/// A module name: snake case, `_` appended if it is a Rust keyword.
#[must_use]
pub fn module_name(name: &str) -> String {
    let mut n = snake(name);
    if RUST_KEYWORDS.contains(&n.as_str()) {
        n.push('_');
    }
    n
}

/// `dependsOn` -> `DependsOn` (the Rust name of an enumerant or a union variant).
#[must_use]
pub fn capitalize(name: &str) -> String {
    let mut cs = name.chars();
    cs.next()
        .map(|c| c.to_ascii_uppercase().to_string() + cs.as_str())
        .unwrap_or_default()
}

/// `createRelationWithGap` -> `create-relation-with-gap` (the word on the command line).
#[must_use]
pub fn kebab(name: &str) -> String {
    snake(name).replace('_', "-")
}
