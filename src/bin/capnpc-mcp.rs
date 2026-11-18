//! `capnpc-mcp`: the capnp plugin binary (`capnp compile -omcp:<dir> ...`).
//!
//! Pattern: Facade — reads the `CodeGeneratorRequest` from standard input and writes the files into the current
//! directory, as every capnp plugin does. `capnp-compiler compile --mcp <dir>` runs the same generator without it.
//! The tables it writes call the command code of `capnpc-cli` (`<name>_cli.rs`): run both on the same request.

fn main() {
    if let Some(a) = std::env::args().nth(1) {
        match a.as_str() {
            "-V" | "--version" => println!("capnpc-mcp {}", env!("CARGO_PKG_VERSION")),
            "-h" | "--help" => println!(
                "capnpc-mcp {}: a capnp plugin; reads a CodeGeneratorRequest on standard input and writes <name>_mcp.rs files\ninto the current directory (capnp compile -omcp:<dir> ...). Without the C++ tool: capnp-compiler compile --mcp <dir> ...\nThe tables call <name>_cli.rs: write that with capnpc-cli (compile --cli <dir>).",
                env!("CARGO_PKG_VERSION")
            ),
            other => {
                eprintln!(
                    "capnpc-mcp: unknown argument '{other}'; it reads a CodeGeneratorRequest on standard input (see --help)"
                );
                std::process::exit(2);
            }
        }
        return;
    }
    if let Err(e) = capnp_compiler::mcpgen::GenerationCommand::new().run(std::io::stdin().lock()) {
        eprintln!("capnpc-mcp: {e}");
        std::process::exit(1);
    }
}
