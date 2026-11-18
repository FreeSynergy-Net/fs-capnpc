//! `cligen`: the generator of `capnpc-cli`; it turns an interface into command line code.
//!
//! Pattern: Facade — [`GenerationCommand`] is the entry point; the model, the command tree and the emitters behind
//! it are separate stages, and the generated code calls only the small trait set in [`rt`].
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::many_single_char_names,
    clippy::match_same_arms,
    reason = "internal stages share one error type; the emitters use short names for the Rust text they write"
)]

pub mod emit;
pub mod model;
pub mod rt;
pub mod tree;

pub use emit::{
    GENERATED_BY, GeneratedFile, GenerationCommand, MarkSource, NoMarks, RT_PATH, generate,
};
