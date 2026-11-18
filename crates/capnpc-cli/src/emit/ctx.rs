//! What the emitters share: the model, the marks, and the list of helper functions still to write.
//!
//! Pattern: Visitor with a work list — writing one function may need another (a struct inside a struct), so every
//! need is registered once and written later; the output is sorted by name, so discovery order never shows.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{Model, StructNode};

/// Generation-time marks that are not in the schema (the strategy behind the `secret` and `kind` annotations).
///
/// Pattern: Strategy — a program that keeps its marks elsewhere (a table file) answers here; the annotations of the
/// schema always count as well.
pub trait MarkSource {
    /// Is the parameter `path` of the command `key` secret?
    fn secret(&self, _key: &str, _path: &str) -> bool {
        false
    }
    /// A hint for the basic type of the parameter `path` of the command `key`.
    fn kind(&self, _key: &str, _path: &str) -> Option<String> {
        None
    }
}

/// No marks beyond the annotations.
pub struct NoMarks;

impl MarkSource for NoMarks {}

/// A helper function the generated file needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Work {
    /// `set_<struct>`: write parameters into a builder.
    Setter(u64),
    /// `out_<struct>`: walk a reader into the output.
    Out(u64),
    /// `enum_<enum>`: the name of an enumerant.
    Enum(u64),
}

/// The state of one file's generation.
pub struct Ctx<'a> {
    /// The contract.
    pub m: &'a Model,
    /// Extra marks.
    pub marks: &'a dyn MarkSource,
    seen: BTreeSet<Work>,
    todo: Vec<Work>,
    /// Finished helpers by (kind, name).
    pub pieces: BTreeMap<(u8, String), String>,
}

impl<'a> Ctx<'a> {
    /// A new, empty state.
    #[must_use]
    pub fn new(m: &'a Model, marks: &'a dyn MarkSource) -> Self {
        Self {
            m,
            marks,
            seen: BTreeSet::new(),
            todo: Vec::new(),
            pieces: BTreeMap::new(),
        }
    }

    /// `set_tasks_capnp_task_filter` for the node `id` (registered for writing).
    pub fn setter(&mut self, id: u64) -> String {
        self.need(Work::Setter(id));
        self.ident("set", id)
    }

    /// `out_tasks_capnp_task` for the node `id`.
    pub fn out(&mut self, id: u64) -> String {
        self.need(Work::Out(id));
        self.ident("out", id)
    }

    /// `enum_tasks_capnp_relation` for the enum `id`.
    pub fn enum_fn(&mut self, id: u64) -> String {
        self.need(Work::Enum(id));
        self.ident("enum", id)
    }

    fn need(&mut self, w: Work) {
        if self.seen.insert(w) {
            self.todo.push(w);
        }
    }

    /// The next helper to write, if any.
    pub fn next(&mut self) -> Option<Work> {
        self.todo.pop()
    }

    /// The name of a helper function: its prefix and the module path of the node without `crate`, in snake case (an
    /// enum is named `Readiness` by capnpc, a function may not be).
    #[must_use]
    pub fn ident(&self, prefix: &str, id: u64) -> String {
        let mut s = prefix.to_owned();
        for p in self.m.paths[&id].iter().skip(1) {
            s.push('_');
            s.push_str(&crate::model::snake(p));
        }
        s
    }

    /// The struct node `id`.
    ///
    /// # Panics
    /// The id is not a struct of the model.
    #[must_use]
    pub fn st(&self, id: u64) -> &'a StructNode {
        &self.m.structs[&id]
    }
}

/// The name of the type of a struct node as a human wrote it (`Uuid`); empty for groups and anonymous structs.
#[must_use]
pub fn type_name(s: &StructNode) -> &str {
    if s.is_group || s.name.contains('$') {
        return "";
    }
    s.name.rsplit(['.', ':']).next().unwrap_or("")
}

/// A Rust string literal for `s` (escaped the way `{:?}` does it; stable).
#[must_use]
pub fn lit(s: &str) -> String {
    format!("{s:?}")
}
