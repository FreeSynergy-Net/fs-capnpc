//! The tables of the generator at the level of the text: the safe defaults, the labels, the marks of a program.
//! (The compiled tables and their JSON are checked in the fixtures crate; the exact text by the golden files.)

#[path = "cligen_common/mod.rs"]
mod common;

use capnp_compiler::mcpgen::{GenerationCommand, MarkSource};

fn text(files: &[&str]) -> String {
    GenerationCommand::new()
        .generate(&common::request(files))
        .unwrap()
        .into_iter()
        .map(|f| f.content)
        .collect()
}

fn mcp() -> String {
    text(&["mcp.capnp", "capnpc_cli.capnp"])
}

/// The text of the field `name` of every tool.
fn values<'a>(text: &'a str, name: &str) -> Vec<&'a str> {
    let prefix = format!("        {name}: ");
    text.lines()
        .filter_map(|l| l.strip_prefix(prefix.as_str()))
        .map(|v| v.strip_suffix(',').unwrap_or(v))
        .collect()
}

#[test]
fn the_default_is_off() {
    for f in [&["tasks.capnp"][..], &["mcp.capnp", "capnpc_cli.capnp"]] {
        let t = text(f);
        assert!(t.contains("pub const ENABLED_BY_DEFAULT: bool = false;"));
        assert!(!t.contains("ENABLED_BY_DEFAULT: bool = true"));
        assert!(t.contains("capnp_compiler::mcpgen::rt::visible(switch, TOOLS)"));
    }
}

#[test]
fn every_tool_is_labelled_as_an_ai_and_has_both_schemas() {
    let t = mcp();
    let (metas, ins, outs) = (
        values(&t, "meta"),
        values(&t, "input_schema"),
        values(&t, "output_schema"),
    );
    assert_eq!(metas.len(), 6);
    assert!(
        metas.iter().all(|m| m.contains("\"actor\":\"ai\"")),
        "{metas:?}"
    );
    assert!(ins.iter().all(|s| s.contains("\"type\":\"object\"")));
    assert!(outs.iter().all(|s| s.contains("\"type\":\"object\"")));
    assert_eq!((ins.len(), outs.len()), (6, 6));
}

#[test]
fn the_revision_is_written_once_and_it_is_the_right_one() {
    let t = mcp();
    assert!(t.contains("pub const PROTOCOL_HINT: &str = \"2026-07-28\";"));
    assert!(
        t.lines()
            .next()
            .unwrap()
            .contains("MCP revision 2026-07-28")
    );
    assert!(!t.contains("2025-"));
}

#[test]
fn what_is_left_out_is_not_in_the_table() {
    let t = mcp();
    for gone in ["mcp.login", "mcp.raw", "mcp.other", "mcp.hidden", "vault"] {
        assert!(
            !values(&t, "name").iter().any(|n| n.contains(gone)),
            "{gone}"
        );
    }
    assert!(t.contains("(\"mcp/Notes.login\", \"secret\")"));
}

struct Marks;

impl MarkSource for Marks {
    fn secret(&self, key: &str, path: &str) -> bool {
        key == "mcp/Notes.add" && path == "tags"
    }
    fn kind(&self, key: &str, path: &str) -> Option<String> {
        (key == "mcp/Notes.add" && path == "text").then(|| "email".to_owned())
    }
}

struct KindOnly;

impl MarkSource for KindOnly {
    fn kind(&self, key: &str, path: &str) -> Option<String> {
        (key == "mcp/Notes.add" && path == "text").then(|| "email".to_owned())
    }
}

#[test]
fn the_marks_of_a_program_add_to_the_annotations() {
    let plain = mcp();
    assert!(values(&plain, "name").contains(&"\"mcp.add\""));
    let mut cmd = GenerationCommand::new();
    cmd.marks(Box::new(Marks));
    let marked: String = cmd
        .generate(&common::request(&["mcp.capnp", "capnpc_cli.capnp"]))
        .unwrap()
        .into_iter()
        .map(|f| f.content)
        .collect();
    // a secret mark takes the tool out, with the reason
    assert!(!values(&marked, "name").contains(&"\"mcp.add\""));
    assert!(marked.contains("(\"mcp/Notes.add\", \"secret\")"));
    // a kind mark makes a format
    let mut kinds = GenerationCommand::new();
    kinds.marks(Box::new(KindOnly));
    let t: String = kinds
        .generate(&common::request(&["mcp.capnp", "capnpc_cli.capnp"]))
        .unwrap()
        .into_iter()
        .map(|f| f.content)
        .collect();
    assert!(t.contains("\"text\":{\"type\":\"string\",\"format\":\"email\"}"));
}

struct ProgramMarks;

impl MarkSource for ProgramMarks {
    fn risk(&self, key: &str) -> Option<String> {
        (key == "mcp/Notes.count").then(|| "read".to_owned())
    }
    fn excluded(&self, key: &str) -> bool {
        key == "mcp/Notes.wipe"
    }
    fn tool_name(&self, key: &str) -> Option<String> {
        (key == "mcp/Notes.add").then(|| "note.add".to_owned())
    }
}

#[test]
fn a_program_can_mark_risk_exclude_and_name() {
    let mut cmd = GenerationCommand::new();
    cmd.marks(Box::new(ProgramMarks));
    let t: String = cmd
        .generate(&common::request(&["mcp.capnp", "capnpc_cli.capnp"]))
        .unwrap()
        .into_iter()
        .map(|f| f.content)
        .collect();
    let names = values(&t, "name");
    assert!(names.contains(&"\"note.add\"") && !names.contains(&"\"mcp.add\""));
    assert!(!names.contains(&"\"mcp.wipe\""));
    assert!(t.contains("(\"mcp/Notes.wipe\", \"excluded\")"));
    // the mark of the program is the risk where the schema says nothing, the schema wins where it speaks
    let risks = values(&t, "risk");
    assert!(risks.contains(&"Some(Danger::Read)"));
    assert!(risks.contains(&"Some(Danger::Add)"));
}
