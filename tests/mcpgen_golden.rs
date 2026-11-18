//! The generated tool tables equal the checked-in ones byte for byte. `MCPGEN_BLESS=1` rewrites them (then look at
//! the diff). A missing golden file fails with the generated text in the message, so a CI log can supply it.

#[path = "cligen_common/mod.rs"]
mod common;

use std::path::PathBuf;

fn golden(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/mcpgen/generated")
        .join(name)
}

/// The generated text of one file.
fn generated(files: &[&str], name: &str) -> String {
    let out = capnp_compiler::mcpgen::GenerationCommand::new()
        .generate(&common::request(files))
        .expect("generates");
    out.into_iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no file {name}"))
        .content
}

fn check(files: &[&str], name: &str) {
    let text = generated(files, name);
    let path = golden(name);
    if std::env::var_os("MCPGEN_BLESS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &text).unwrap();
    }
    let Ok(want) = std::fs::read_to_string(&path) else {
        panic!("GOLDEN-MISSING {name}\n<<<BEGIN {name}\n{text}<<<END {name}");
    };
    assert!(
        want == text,
        "{name} differs from tests/mcpgen/generated/{name}\n<<<BEGIN {name}\n{text}<<<END {name}"
    );
}

#[test]
fn golden_tasks() {
    check(&["tasks.capnp"], "tasks_mcp.rs");
}

#[test]
fn golden_multi_root_and_reserved_words() {
    check(&["multi.capnp"], "multi_mcp.rs");
    check(&["common.capnp"], "common_mcp.rs");
    check(&["reserved.capnp"], "reserved_mcp.rs");
    check(&["words.capnp"], "words_mcp.rs");
}

#[test]
fn golden_features() {
    check(&["features.capnp", "capnpc_cli.capnp"], "features_mcp.rs");
}

#[test]
fn golden_mcp_cases() {
    check(&["mcp.capnp", "capnpc_cli.capnp"], "mcp_mcp.rs");
}
