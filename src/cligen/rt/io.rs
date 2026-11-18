//! Input, output, sink and hooks of a generated call.
//!
//! Pattern: Strategy / Visitor — the generated code asks the `Input` for named values and walks a result into an
//! `Output`; the program decides where values come from and how they are shown.

use super::CallError;

/// One value of a result, handed to [`Output::field`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value<'a> {
    /// `Void`.
    Void,
    /// `Bool`.
    Bool(bool),
    /// A signed integer.
    Int(i64),
    /// An unsigned integer.
    Uint(u64),
    /// A float.
    Float(f64),
    /// `Text`.
    Text(&'a str),
    /// `Data`.
    Data(&'a [u8]),
    /// An enumerant, by its name in the schema.
    Enum(&'static str),
    /// An enumerant this version does not know (its number).
    UnknownEnum(u16),
    /// A capability (the name of its interface); it cannot be shown.
    Capability(&'static str),
    /// A value the generator cannot read (the reason); shown, never dropped.
    Unreadable(&'static str),
}

/// Named input values. `path` is the parameter path (`filter.due-from`, `ids.0`); `None` means "not given".
pub trait Input {
    /// Is the parameter, or anything below it, given?
    fn has(&self, path: &str) -> bool;
    /// A text value.
    ///
    /// # Errors
    /// The value is not valid.
    fn text(&mut self, path: &str) -> Result<Option<String>, CallError>;
    /// An integer value (the generated code checks the range of the type).
    ///
    /// # Errors
    /// The value is not valid.
    fn int(&mut self, path: &str) -> Result<Option<i128>, CallError>;
    /// A float value.
    ///
    /// # Errors
    /// The value is not valid.
    fn float(&mut self, path: &str) -> Result<Option<f64>, CallError>;
    /// A boolean value.
    ///
    /// # Errors
    /// The value is not valid.
    fn boolean(&mut self, path: &str) -> Result<Option<bool>, CallError>;
    /// An enumerant: the index into `names` (the enumerants in schema order).
    ///
    /// # Errors
    /// The value matches no name or more than one.
    fn enumerant(&mut self, path: &str, names: &[&str]) -> Result<Option<usize>, CallError>;
    /// The number of elements of a list parameter; the elements are `path.0`, `path.1` ...
    ///
    /// # Errors
    /// The value is not valid.
    fn count(&mut self, path: &str) -> Result<Option<u32>, CallError>;
    /// A data value.
    ///
    /// # Errors
    /// The value is not valid.
    fn data(&mut self, path: &str) -> Result<Option<Vec<u8>>, CallError>;
}

/// A result, visited from the root. The root struct has the name `""`.
pub trait Output {
    /// A struct starts (`ty_name` is the name of its type in the schema, for example `Uuid`).
    fn begin_struct(&mut self, name: &str, ty_name: &str);
    /// A list starts (`ty_name` names the element type).
    fn begin_list(&mut self, name: &str, ty_name: &str);
    /// A value.
    fn field(&mut self, name: &str, value: Value<'_>);
    /// The struct or list ends.
    fn end(&mut self);
    /// The sink for the calls of a listener, if the program supports watching.
    fn sink(&mut self) -> Option<Box<dyn Sink>> {
        None
    }
}

/// An [`Output`] that shows nothing: the generated listener walks every call into it once, to find out whether the
/// call can be read, independent of what the sink does with the line.
pub struct Discard;

impl Output for Discard {
    fn begin_struct(&mut self, _: &str, _: &str) {}
    fn begin_list(&mut self, _: &str, _: &str) {}
    fn field(&mut self, _: &str, _: Value<'_>) {}
    fn end(&mut self) {}
}

/// Takes the calls a listener receives, one line each. `line` returns when the line is written (back pressure):
/// the generated listener answers the call only afterwards, and never answers with an error. If `render` fails
/// (a call that cannot be read), the sink writes an error line: a line is never dropped silently. The exit code does
/// not depend on that: the generated listener counts every call it cannot read itself (see [`Outcome`](super::Outcome)),
/// also when the sink never calls `render`.
pub trait Sink: 'static {
    /// One call of the method `kind`; `render` walks the arguments into the output of the line.
    fn line(
        &mut self,
        kind: &str,
        render: &mut dyn FnMut(&mut dyn Output) -> Result<(), CallError>,
    );
}

impl Sink for Box<dyn Sink> {
    fn line(
        &mut self,
        kind: &str,
        render: &mut dyn FnMut(&mut dyn Output) -> Result<(), CallError>,
    ) {
        (**self).line(kind, render);
    }
}

/// What the program knows about a parameter beyond the schema (its own marks).
pub trait Hooks {
    /// A hint for the basic type of the value at `path` of the command `key` (`uuid`, `timestamp`).
    fn hint(&self, _key: &str, _path: &str) -> Option<String> {
        None
    }
    /// Must the value at `path` of the command `key` never come from an argument?
    fn secret(&self, _key: &str, _path: &str) -> bool {
        false
    }
}
