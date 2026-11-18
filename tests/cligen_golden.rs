//! T4: the generated files equal the checked-in ones byte for byte. `CAPNPC_CLI_BLESS=1` rewrites them (then look
//! at the diff).

#[path = "cligen_common/mod.rs"]
mod common;

use std::path::PathBuf;

fn golden(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/cligen/generated")
        .join(name)
}

fn check(files: &[&str], name: &str) {
    let text = common::generated(files, name);
    let path = golden(name);
    if std::env::var_os("CAPNPC_CLI_BLESS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &text).unwrap();
    }
    let want = std::fs::read_to_string(&path).expect("golden file (CAPNPC_CLI_BLESS=1 writes it)");
    assert!(
        want == text,
        "{name} differs from tests/cligen/generated/{name}"
    );
}

#[test]
fn t4_tasks_golden() {
    check(&["tasks.capnp"], "tasks_cli.rs");
}

#[test]
fn t4_multi_root_and_reserved_word_golden() {
    check(&["multi.capnp"], "multi_cli.rs");
    check(&["common.capnp"], "common_cli.rs");
    check(&["reserved.capnp"], "reserved_cli.rs");
    check(&["words.capnp"], "words_cli.rs");
}

#[test]
fn t4_features_golden() {
    check(&["features.capnp", "capnpc_cli.capnp"], "features_cli.rs");
}
