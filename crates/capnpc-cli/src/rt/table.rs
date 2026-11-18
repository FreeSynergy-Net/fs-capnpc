//! The table of commands and its rows.
//!
//! Pattern: Value Object — plain static data; the program walks it for help, completion and coverage.

use core::any::Any;

/// The type of a parameter, as far as the run time needs it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamTy {
    /// A flag without a value (a `Void` union member).
    Void,
    /// `Bool`.
    Bool,
    /// An integer.
    Int {
        /// Signed?
        signed: bool,
        /// Width in bits.
        bits: u8,
    },
    /// A float of this width.
    Float(u8),
    /// `Text`.
    Text,
    /// `Data`.
    Data,
    /// An enum with these enumerants, in schema order.
    Enum(&'static [&'static str]),
    /// A list; the elements are `path.0`, `path.1` (in the table `path.*`).
    List,
    /// A struct or group; its members follow with `path.` in front.
    Struct,
    /// The generator cannot read it (the reason is in `ty_name`).
    Unreadable,
}

/// One parameter of a command.
#[derive(Clone, Copy, Debug)]
pub struct Param {
    /// Path, `filter.due-from`.
    pub path: &'static str,
    /// Type.
    pub ty: ParamTy,
    /// Name of the type in the schema (`Uuid`), the element type of a list, or the reason of `Unreadable`.
    pub ty_name: &'static str,
    /// The union this parameter is a member of, as the path of the struct that holds it (`""` for the parameters
    /// themselves); `None` if it is no union member.
    pub union_of: Option<&'static str>,
    /// A hint for the basic type from the mark `kind` (`uuid`, `timestamp`); empty if none.
    pub hint: &'static str,
    /// Marked secret: never an argument, never a tool description.
    pub secret: bool,
    /// The doc comment of the field.
    pub doc: &'static str,
}

/// One command: one method of the contract.
#[derive(Clone, Copy, Debug)]
pub struct Command {
    /// The words after the program name.
    pub words: &'static [&'static str],
    /// Coverage key `file/Interface.method`.
    pub key: &'static str,
    /// Interface id of the method.
    pub iface_id: u64,
    /// Ordinal of the method.
    pub ordinal: u16,
    /// Parameters in declaration order.
    pub params: &'static [Param],
    /// The method takes a listener: the command watches.
    pub watch: bool,
    /// The doc comment of the method (the English help).
    pub doc: &'static str,
    /// Annotations of the method that are not known to the generator: (annotation id, value as text).
    pub annotations: &'static [(u64, &'static str)],
}

/// A word that only leads to the commands below it (a method that returns a capability).
#[derive(Clone, Copy, Debug)]
pub struct Group {
    /// The words after the program name.
    pub words: &'static [&'static str],
    /// Coverage key `file/Interface.method`.
    pub key: &'static str,
}

/// What a finished call keeps alive (the subscription of a watch) until the program drops it.
pub struct Held(Option<Box<dyn Any>>);

impl Held {
    /// Nothing to keep.
    #[must_use]
    pub fn none() -> Self {
        Self(None)
    }

    /// Keep `what` alive.
    #[must_use]
    pub fn new<T: Any>(what: T) -> Self {
        Self(Some(Box::new(what)))
    }

    /// Is something kept?
    #[must_use]
    pub fn is_some(&self) -> bool {
        self.0.is_some()
    }
}

/// `join("filter", "due-from")` is `filter.due-from`; an empty prefix gives the name alone.
#[must_use]
pub fn join(path: &str, name: &str) -> String {
    if path.is_empty() {
        name.to_owned()
    } else {
        format!("{path}.{name}")
    }
}
