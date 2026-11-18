//! `capnpc-cli`: the capnp plugin binary (`capnp compile -ocli:<dir> ...`).
//!
//! Pattern: Facade — reads the `CodeGeneratorRequest` from standard input and writes the files into the current
//! directory, as every capnp plugin does.

fn main() {
    if let Err(e) = capnpc_cli::GenerationCommand::new().run(std::io::stdin().lock()) {
        eprintln!("capnpc-cli: {e}");
        std::process::exit(1);
    }
}
