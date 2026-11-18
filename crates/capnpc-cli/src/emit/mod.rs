//! The generator: a `CodeGeneratorRequest` in, one Rust file per requested file out.
//!
//! Pattern: Facade — `GenerationCommand` is the entry point; the model, the tree and the emitters behind it are
//! separate stages. The output depends only on the request: sorted maps, fixed indentation, no time, no absolute path.

mod command;
mod ctx;
mod file;
mod listener;
mod readers;
mod runfn;
mod setters;
mod table;
mod text;

pub use command::{GENERATED_BY, GeneratedFile, GenerationCommand};
pub use ctx::{MarkSource, NoMarks};
pub use file::generate;
