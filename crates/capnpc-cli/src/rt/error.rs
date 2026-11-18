//! The error of a generated call.
//!
//! Pattern: Value Object — the kind and the path of the parameter say what went wrong; the program maps it to an exit code.

use core::fmt;

/// What went wrong.
#[derive(Debug)]
pub enum ErrorKind {
    /// The remote call or a reader failed.
    Rpc(capnp::Error),
    /// Two members of one union were given.
    UnionTwice,
    /// An integer does not fit its type (`u8`, `i32` ...).
    OutOfRange(&'static str),
    /// The value is not valid for the parameter.
    BadValue(String),
    /// The result carries a union member or enumerant this version does not know.
    NotInSchema(u16),
    /// The input cannot supply what the call needs (for example a listener without a sink).
    Unsupported(&'static str),
}

/// The error of a generated call, with the path of the parameter (empty for the call itself).
#[derive(Debug)]
pub struct CallError {
    /// What went wrong.
    pub kind: ErrorKind,
    /// Path of the parameter, `filter.due-from`; empty if none.
    pub path: String,
}

impl CallError {
    /// An error about the parameter `path`.
    #[must_use]
    pub fn at(path: &str, kind: ErrorKind) -> Self {
        Self {
            kind,
            path: path.to_owned(),
        }
    }

    /// Two members of the union at `path` were given.
    #[must_use]
    pub fn union_twice(path: &str) -> Self {
        Self::at(path, ErrorKind::UnionTwice)
    }

    /// The integer at `path` does not fit `ty`.
    #[must_use]
    pub fn out_of_range(path: &str, ty: &'static str) -> Self {
        Self::at(path, ErrorKind::OutOfRange(ty))
    }
}

impl From<capnp::Error> for CallError {
    fn from(e: capnp::Error) -> Self {
        Self::at("", ErrorKind::Rpc(e))
    }
}

impl From<capnp::NotInSchema> for CallError {
    fn from(e: capnp::NotInSchema) -> Self {
        Self::at("", ErrorKind::NotInSchema(e.0))
    }
}

impl From<core::str::Utf8Error> for CallError {
    fn from(e: core::str::Utf8Error) -> Self {
        Self::at("", ErrorKind::Rpc(e.into()))
    }
}

impl fmt::Display for CallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.path.is_empty() {
            write!(f, "{}: ", self.path)?;
        }
        match &self.kind {
            ErrorKind::Rpc(e) => write!(f, "{e}"),
            ErrorKind::UnionTwice => f.write_str("more than one member of the union given"),
            ErrorKind::OutOfRange(t) => write!(f, "value does not fit {t}"),
            ErrorKind::BadValue(v) => write!(f, "bad value: {v}"),
            ErrorKind::NotInSchema(n) => write!(f, "unknown variant {n}"),
            ErrorKind::Unsupported(w) => write!(f, "unsupported: {w}"),
        }
    }
}

impl std::error::Error for CallError {}
