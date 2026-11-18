//! Shared by the tests: compile the fixture schemas with capnp-compiler.
#![allow(dead_code)]

use std::path::PathBuf;

use capnp_compiler::CompileCommand;
use capnpc_cli::model::Model;

/// The folder of the fixture schemas.
pub fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The serialized request for the given fixture files (the first is the requested one).
pub fn request(files: &[&str]) -> Vec<u8> {
    let dir = fixtures();
    let mut cmd = CompileCommand::new();
    cmd.src_prefix(&dir);
    cmd.import_path(env!("CARGO_MANIFEST_DIR"));
    for f in files {
        // the annotation file lives in the crate, the schemas under test in tests/fixtures
        let base = if *f == "capnpc_cli.capnp" {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        } else {
            dir.clone()
        };
        cmd.file(base.join(f));
    }
    cmd.run_bytes().expect("fixture compiles")
}

/// The model of the given fixture files.
pub fn model(files: &[&str]) -> Model {
    Model::from_request(&request(files)).expect("request reads")
}

/// The generated text of one file.
pub fn generated(files: &[&str], name: &str) -> String {
    let out = capnpc_cli::GenerationCommand::new()
        .generate(&request(files))
        .expect("generates");
    out.into_iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no file {name}"))
        .content
}
