//! The exit code of a call.
//!
//! Pattern: Strategy — one function maps what a call ended with to the exit values of the proposal (0 ok, 1 the other
//! side answered with an error, 2 a wrong word or value, 3 not reachable or the stream was broken); the generated entry
//! point uses it, so a program cannot forget it.

use super::{CallError, ErrorKind, Outcome};

/// 0 ok · 1 the contract answered with an error (also a result that cannot be read) · 2 a wrong word or value ·
/// 3 the program is not reachable, or a call of a watch could not be read (the stream is broken, lines are missing).
#[must_use]
pub fn exit_code(result: &Result<Outcome, CallError>) -> i32 {
    match result {
        Ok(o) if o.errors() > 0 => 3,
        Ok(_) => 0,
        Err(e) => match &e.kind {
            ErrorKind::Rpc(c) if matches!(c.kind, capnp::ErrorKind::Disconnected) => 3,
            ErrorKind::Rpc(_) | ErrorKind::NotInSchema(_) => 1,
            ErrorKind::UnionTwice
            | ErrorKind::OutOfRange(_)
            | ErrorKind::BadValue(_)
            | ErrorKind::Unsupported(_) => 2,
        },
    }
}
