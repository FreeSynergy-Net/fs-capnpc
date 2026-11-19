//! The annotation `sensitive` (and the directory `SCHEMA_DIR` that carries it): the generator of tools writes it into the
//! schema and into the `_meta` of the tool, it never makes a tool disappear (a secret does), the marks of a program can add to it.

#[path = "cligen_common/mod.rs"]
mod common;

use capnp_compiler::cligen::MarkSource;
use capnp_compiler::mcpgen::GenerationCommand;

const FILES: [&str; 2] = ["sensitive.capnp", "capnpc_cli.capnp"];

fn text(marks: Option<Box<dyn MarkSource>>) -> String {
    let mut g = GenerationCommand::new();
    if let Some(m) = marks {
        g.marks(m);
    }
    g.generate(&common::request(&FILES))
        .unwrap()
        .into_iter()
        .map(|f| f.content)
        .collect()
}

/// The generated text of the tool `name`: from its `name:` line to the next tool.
fn tool<'a>(t: &'a str, name: &str) -> &'a str {
    let start = t
        .find(&format!("name: \"{name}\""))
        .unwrap_or_else(|| panic!("no tool {name}"));
    let rest = &t[start..];
    let end = rest[10..].find("ToolSpec {").map_or(rest.len(), |e| e + 10);
    &rest[..end]
}

#[test]
fn the_directory_of_the_annotations_is_exported_and_enough_to_compile_a_contract() {
    let dir = std::path::Path::new(capnp_compiler::SCHEMA_DIR);
    let file = dir.join("capnpc_cli.capnp");
    let body = std::fs::read_to_string(&file).expect("the annotation file is there");
    assert!(body.contains("annotation sensitive @0x8d8306a2e2dc31b6 (field, param) :Void;"));
    assert!(body.contains("annotation secret @0xefac3d5046acfeb0"));
    // a contract that imports it compiles with this directory as the ONLY import path
    let mut cmd = capnp_compiler::CompileCommand::new();
    cmd.src_prefix(common::fixtures());
    cmd.import_path(dir);
    cmd.file(common::fixtures().join("sensitive.capnp"));
    let request = cmd.run_bytes().expect("compiles with SCHEMA_DIR alone");
    let model = capnp_compiler::cligen::model::Model::from_request(&request).expect("reads");
    let ids: Vec<u64> = model
        .structs
        .values()
        .flat_map(|s| {
            s.fields
                .iter()
                .flat_map(|f| f.annotations.iter().map(|a| a.id))
        })
        .collect();
    assert!(
        ids.contains(&capnp_compiler::cligen::model::SENSITIVE_ANNOTATION),
        "{ids:x?}"
    );
    assert_eq!(
        capnp_compiler::cligen::model::SENSITIVE_ANNOTATION,
        0x8d83_06a2_e2dc_31b6
    );
}

#[test]
fn a_sensitive_result_and_argument_are_marked_in_the_schema_and_in_the_meta_and_stay_tools() {
    let t = text(None);
    let diagnose = tool(&t, "sensitive.diagnose");
    assert!(diagnose.contains("\"sensitive\":true"), "{diagnose}");
    // in the result: the field of the struct and the field of the method, nowhere else
    assert_eq!(
        diagnose.matches("\"x-fs-sensitive\":true").count(),
        2,
        "{diagnose}"
    );
    assert!(
        diagnose.contains("\"diagnosis\":{\"type\":\"string\",\"x-fs-sensitive\":true"),
        "{diagnose}"
    );
    let share = tool(&t, "sensitive.share");
    assert!(share.contains("\"sensitive\":true"), "{share}");
    assert_eq!(
        share.matches("\"x-fs-sensitive\":true").count(),
        1,
        "only `detail`: {share}"
    );
    assert!(
        share.contains("\"detail\":{\"type\":\"string\",\"x-fs-sensitive\":true}"),
        "{share}"
    );
    assert!(share.contains("\"with\":{\"type\":\"string\"}"), "{share}");
}

#[test]
fn a_tool_without_the_mark_has_none_and_the_meta_has_no_sensitive_key() {
    let plain = text(None);
    let plain = tool(&plain, "sensitive.plain");
    assert!(
        !plain.contains("\"sensitive\":") && !plain.contains("x-fs-sensitive"),
        "{plain}"
    );
}

#[test]
fn a_secret_still_wins_it_is_no_tool_even_if_another_field_is_sensitive() {
    let t = text(None);
    assert!(!t.contains("name: \"sensitive.secret\""));
    assert!(
        t.contains("(\"sensitive/Records.secret\", \"secret\")"),
        "{t}"
    );
}

struct Marks;
impl MarkSource for Marks {
    fn sensitive(&self, key: &str, path: &str) -> bool {
        key == "sensitive/Records.plain" && path == "name"
    }
}

#[test]
fn the_marks_of_a_program_add_a_sensitive_argument_and_only_that_one() {
    let t = text(Some(Box::new(Marks)));
    let plain = tool(&t, "sensitive.plain");
    assert!(plain.contains("\"sensitive\":true"), "{plain}");
    assert!(
        plain.contains("\"name\":{\"type\":\"string\",\"x-fs-sensitive\":true}"),
        "{plain}"
    );
    // the other tools are as they were
    assert_eq!(
        tool(&t, "sensitive.share"),
        tool(&text(None), "sensitive.share")
    );
}

#[test]
fn the_command_code_is_not_changed_by_the_mark() {
    // `sensitive` is for the tools: the command line code has no `secret` for it and writes the same table with or without it
    let cli = common::generated(&FILES[..1], "sensitive_cli.rs");
    assert!(
        !cli.contains("x-fs-sensitive") && !cli.contains("8d8306a2"),
        "the command code does not carry the mark"
    );
    let secrets: Vec<&str> = cli.lines().filter(|l| l.contains("secret: true")).collect();
    assert_eq!(
        secrets.len(),
        0,
        "no argument is a secret and `sensitive` is no secret: {secrets:?}"
    );
}

struct ResultMarks;
impl MarkSource for ResultMarks {
    fn sensitive(&self, key: &str, path: &str) -> bool {
        // a path that only a RESULT has (`count`) and one that both have (`name`): the marks speak of the arguments only
        key == "sensitive/Records.plain" && (path == "count" || path == "name.x")
    }
}

#[test]
fn the_marks_of_a_program_do_not_reach_the_results() {
    let t = text(Some(Box::new(ResultMarks)));
    let plain = tool(&t, "sensitive.plain");
    assert!(
        !plain.contains("\"sensitive\":") && !plain.contains("x-fs-sensitive"),
        "{plain}"
    );
}
