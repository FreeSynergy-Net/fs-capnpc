//! `mcpgen`: the generator of `capnpc-mcp`; it turns an interface into a table of MCP tools.
//!
//! Pattern: Facade — [`GenerationCommand`] is the entry point; it reads the same `CodeGeneratorRequest` as `cligen`,
//! uses the command tree of `cligen` (one tool per command, no second reading of the contract), and writes
//! `<name>_mcp.rs`: the table `TOOLS` (JSON Schemas as text, no serde) and a function that finds the command code
//! of `<name>_cli.rs` for a tool. There is no protocol here and no run time of its own: that is the program's
//! (`fs-mcp`); the generated code is off until the program switches it on ([`rt::Switch`]).
//!
//! MCP revision: [`rt::PROTOCOL_VERSION`] only.
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::match_same_arms,
    reason = "internal stages share one error type; the emitters use short names for the Rust text they write"
)]

mod command;
mod emit;
mod json;
mod scan;
mod schema;
mod tools;

pub mod rt;

pub use crate::cligen::{GeneratedFile, MarkSource, NoMarks};
pub use command::{GENERATED_BY, GenerationCommand, generate};
