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
    println!("cargo:rerun-if-changed=../../tests/cligen/fixtures");
    println!("cargo:rerun-if-changed=../../src/cligen/capnpc_cli.capnp");
    println!("cargo:rerun-if-changed=../../src/cligen");
}
