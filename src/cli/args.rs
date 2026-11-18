//! Argument parsing.
//!
//! Pattern: Command — a tiny parser from `argv` to a typed [`Command`], no external dependency.

use std::path::PathBuf;

/// Options of the `compile` sub-command.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompileArgs {
    /// Directory receiving the generated Rust files (`-o`).
    pub output_dir: Option<PathBuf>,
    /// Directory receiving the command line code (`<name>_cli.rs`, `--cli`).
    pub cli_dir: Option<PathBuf>,
    /// Directory receiving the tool tables (`<name>_mcp.rs`, `--mcp`).
    pub mcp_dir: Option<PathBuf>,
    /// Also write the raw `CodeGeneratorRequest` to this file (`--request`).
    pub request_file: Option<PathBuf>,
    /// Import search directories (`-I`).
    pub import_paths: Vec<PathBuf>,
    /// Source prefixes (`--src-prefix`).
    pub src_prefixes: Vec<PathBuf>,
    /// Disable the embedded standard schemas (`--no-standard-import`).
    pub no_standard_import: bool,
    /// Schema files to compile.
    pub files: Vec<PathBuf>,
}

/// A parsed command line.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(
    clippy::large_enum_variant,
    reason = "one value per process run; boxing would only change every pattern that reads the options"
)]
pub enum Command {
    /// Print the usage text.
    Help,
    /// Print the version.
    Version,
    /// Compile schemas.
    Compile(CompileArgs),
}

/// Usage text.
pub const USAGE: &str = "\
capnp-compiler - a Cap'n Proto schema compiler in pure Rust

USAGE:
    capnp-compiler compile [OPTIONS] <FILE>...

OPTIONS:
    -o, --output <DIR>       write generated Rust (<name>_capnp.rs) into DIR
    -I, --import-path <DIR>  search DIR for absolute imports (repeatable)
        --cli <DIR>          write command line code (<name>_cli.rs, capnpc-cli) into DIR
        --mcp <DIR>          write MCP tool tables (<name>_mcp.rs, capnpc-mcp) into DIR; they call <name>_cli.rs, so give --cli too
        --src-prefix <DIR>   strip DIR from file names (repeatable)
        --no-standard-import do not search the embedded /capnp/*.capnp files
        --request <FILE>     also write the raw CodeGeneratorRequest to FILE
    -h, --help               print this help
    -V, --version            print the version
";

fn value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<PathBuf, String> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{flag} needs a value"))
}

/// Parse `argv` (without the program name).
///
/// # Errors
/// Returns a message for unknown options, missing values or missing files.
pub fn parse_args(argv: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let mut args = argv.into_iter();
    match args.next().as_deref() {
        None | Some("-h" | "--help" | "help") => return Ok(Command::Help),
        Some("-V" | "--version") => return Ok(Command::Version),
        Some("compile") => {}
        Some(other) => return Err(format!("unknown command '{other}'")),
    }
    let mut out = CompileArgs::default();
    while let Some(a) = args.next() {
        match a.as_str() {
            "-o" | "--output" => out.output_dir = Some(value(&mut args, &a)?),
            "-I" | "--import-path" => out.import_paths.push(value(&mut args, &a)?),
            "--src-prefix" => out.src_prefixes.push(value(&mut args, &a)?),
            "--cli" => out.cli_dir = Some(value(&mut args, &a)?),
            "--mcp" => out.mcp_dir = Some(value(&mut args, &a)?),
            "--request" => out.request_file = Some(value(&mut args, &a)?),
            "--no-standard-import" => out.no_standard_import = true,
            "-h" | "--help" => return Ok(Command::Help),
            f if f.starts_with('-') && f != "-" => return Err(format!("unknown option '{f}'")),
            f => out.files.push(PathBuf::from(f)),
        }
    }
    if out.files.is_empty() {
        return Err("no schema files given".into());
    }
    if out.output_dir.is_none()
        && out.cli_dir.is_none()
        && out.mcp_dir.is_none()
        && out.request_file.is_none()
    {
        return Err(
            "nothing to do: give -o <dir>, --cli <dir>, --mcp <dir> and/or --request <file>".into(),
        );
    }
    Ok(Command::Compile(out))
}
