//! A1: a file whose `capnpc` code lives in another crate (`crates_provide_map`, as `fs-interfaces` does for `fs-types`):
//! its types are named `::fs_types::types_capnp::...`, everything else stays as it was.

#[path = "cligen_common/mod.rs"]
mod common;

use std::collections::HashMap;

use capnp_compiler::cligen::GenerationCommand;

/// The file id of `types.capnp` (`@0x916fad50f1c14440`), the same as in `fs-types`.
const TYPES_FILE_ID: u64 = 0x916f_ad50_f1c1_4440;

fn provided(name: &str) -> String {
    let mut cmd = GenerationCommand::new();
    cmd.crates_provide_map(HashMap::from([(TYPES_FILE_ID, name.to_owned())]));
    let files = cmd.generate(&common::request(&["tasks.capnp"])).unwrap();
    files
        .into_iter()
        .find(|f| f.name == "tasks_cli.rs")
        .expect("tasks_cli.rs")
        .content
}

#[test]
fn a1_types_of_the_other_crate_are_named_with_it() {
    let plain = common::generated(&["tasks.capnp"], "tasks_cli.rs");
    let text = provided("fs_types");
    assert!(
        plain.contains("crate::types_capnp::"),
        "the fixture uses types.capnp"
    );
    assert!(text.contains("::fs_types::types_capnp::"));
    assert!(!text.contains("crate::types_capnp"));
    // nothing else changes: the same text, with the path put back
    assert_eq!(
        text.replace("::fs_types::types_capnp", "crate::types_capnp"),
        plain
    );
    // the types of tasks.capnp itself stay below `crate`
    assert!(text.contains("crate::tasks_capnp::"));
}

#[test]
fn a1_the_crate_name_is_used_as_given() {
    assert!(provided("other_name").contains("::other_name::types_capnp::"));
}

#[test]
fn a1_golden() {
    let text = provided("fs_types");
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/cligen/generated/tasks_fs_types_cli.rs");
    if std::env::var_os("CAPNPC_CLI_BLESS").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    let want = std::fs::read_to_string(&path).expect("golden file (CAPNPC_CLI_BLESS=1 writes it)");
    assert!(
        want == text,
        "differs from tests/cligen/generated/tasks_fs_types_cli.rs"
    );
}

#[test]
fn a1_an_unknown_file_id_changes_nothing() {
    let mut cmd = GenerationCommand::new();
    cmd.crates_provide_map(HashMap::from([(1, "nothing".to_owned())]));
    let files = cmd.generate(&common::request(&["tasks.capnp"])).unwrap();
    let text = &files
        .iter()
        .find(|f| f.name == "tasks_cli.rs")
        .unwrap()
        .content;
    assert_eq!(*text, common::generated(&["tasks.capnp"], "tasks_cli.rs"));
}

#[test]
fn a1_a_requested_file_can_be_provided_too() {
    let src = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/cligen/fixtures/tasks.capnp"),
    )
    .unwrap();
    let id = src
        .lines()
        .find_map(|l| l.strip_prefix("@0x"))
        .map(|h| u64::from_str_radix(h.trim_end_matches(';'), 16).unwrap())
        .expect("file id");
    let mut cmd = GenerationCommand::new();
    cmd.crates_provide_map(HashMap::from([(id, "fs_tasks".to_owned())]));
    let files = cmd.generate(&common::request(&["tasks.capnp"])).unwrap();
    let text = &files
        .iter()
        .find(|f| f.name == "tasks_cli.rs")
        .unwrap()
        .content;
    assert!(text.contains("::fs_tasks::tasks_capnp::"));
    assert!(!text.contains("crate::tasks_capnp"));
}
