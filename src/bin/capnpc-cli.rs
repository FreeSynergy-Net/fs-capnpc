//! `capnpc-cli`: the capnp plugin binary (`capnp compile -ocli:<dir> ...`).
//!
//! Pattern: Facade — reads the `CodeGeneratorRequest` from standard input and writes the files into the current
//! directory, as every capnp plugin does. `capnp-compiler compile --cli <dir>` runs the same generator without it.

fn main() {
    if let Some(a) = std::env::args().nth(1) {
        match a.as_str() {
            "-V" | "--version" => println!("capnpc-cli {}", env!("CARGO_PKG_VERSION")),
            _ => println!(
                "capnpc-cli {}: a capnp plugin; reads a CodeGeneratorRequest on standard input and writes <name>_cli.rs files\ninto the current directory (capnp compile -ocli:<dir> ...). Without the C++ tool: capnp-compiler compile --cli <dir> ...",
                env!("CARGO_PKG_VERSION")
            ),
        }
        return;
    }
    if let Err(e) = capnp_compiler::cligen::GenerationCommand::new().run(std::io::stdin().lock()) {
        eprintln!("capnpc-cli: {e}");
        std::process::exit(1);
    }
}
