//! The run-time side of the generated code: the small trait set the generated functions call.
//!
//! Pattern: Strategy — the generated code only types and calls; everything that carries meaning (reading values,
//! printing results, help) is implemented by the program that uses it behind these traits. The exit code is the
//! exception: the generated `run_to_exit` makes it with [`exit_code`], so a program cannot forget it.

mod error;
mod exit;
mod io;
mod table;

pub use error::{CallError, ErrorKind};
pub use exit::exit_code;
pub use io::{Discard, Hooks, Input, Output, Sink, Value};
pub use table::{Command, Group, Held, Outcome, Param, ParamTy, join};

/// Version of the contract between generated code and this module. Generated code asserts it at compile time.
pub const ABI: u32 = 3;
