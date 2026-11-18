//! Compile the fixture schemas with capnp-compiler, then write the Rust types (capnpc) and the command code
//! (capnpc-cli) for the same request. The test crate includes the result and compiles it.

use std::path::PathBuf;

use capnp_compiler::CompileCommand;

fn main() {
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cli = root.join("src/cligen");
    let fixtures = root.join("tests/cligen/fixtures");
    // (types: every file of the set; commands: only the files that are meant to have a command line, because
    // every interface that nothing leads to is a root, and `common.capnp` has a method named `version`, which gets the word `method-version`)
    for (types, commands) in [
        (
            &["tasks.capnp", "common.capnp", "types.capnp"][..],
            &["tasks.capnp", "common.capnp"][..],
        ),
        (
            &["features.capnp", "capnpc_cli.capnp"][..],
            &["features.capnp"][..],
        ),
        (&["multi.capnp"][..], &["multi.capnp"][..]),
        (&["words.capnp"][..], &["words.capnp"][..]),
    ] {
        let request = |set: &[&str]| {
            let mut cmd = CompileCommand::new();
            cmd.src_prefix(&fixtures);
            cmd.import_path(&cli);
            for f in set {
                cmd.file(if *f == "capnpc_cli.capnp" {
                    cli.join(f)
                } else {
                    fixtures.join(f)
                });
            }
            cmd.run_bytes().expect("fixtures compile")
        };
        capnpc::codegen::CodeGenerationCommand::new()
            .output_directory(&out)
            .run(&request(types)[..])
            .expect("capnpc");
        capnp_compiler::cligen::GenerationCommand::new()
            .output_directory(&out)
            .run(&request(commands)[..])
            .expect("capnpc-cli");
    }
    provided(&root, &out);
    println!("cargo:rerun-if-changed=../../tests/cligen/fixtures");
    println!("cargo:rerun-if-changed=../../src/cligen/capnpc_cli.capnp");
    println!("cargo:rerun-if-changed=../../src/cligen");
}

/// `tasks.capnp` again, as `fs-interfaces` generates it: the code of `types.capnp` is in another crate (`fs_types`,
/// here the test crate itself, see `extern crate self`), so the generated code must name `::fs_types::types_capnp`.
fn provided(root: &std::path::Path, out: &std::path::Path) {
    let fixtures = root.join("tests/cligen/fixtures");
    let mut cmd = CompileCommand::new();
    cmd.src_prefix(&fixtures);
    cmd.file(fixtures.join("tasks.capnp"));
    let request = cmd.run_bytes().expect("tasks compiles");
    let dir = out.join("provided");
    std::fs::create_dir_all(&dir).expect("create dir");
    capnp_compiler::cligen::GenerationCommand::new()
        .crates_provide_map(std::collections::HashMap::from([(
            0x916f_ad50_f1c1_4440_u64,
            "fs_types".to_owned(),
        )]))
        .output_directory(&dir)
        .run(&request[..])
        .expect("capnpc-cli with a provided crate");
}
