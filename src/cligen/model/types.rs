//! The types of the model.
//!
//! Pattern: Value Object — plain owned data, kept in sorted maps.

use super::Annotation;
use std::collections::BTreeMap;

/// Node id of `StreamResult` (`stream.capnp`): the result of a streaming method.
pub const STREAM_RESULT_ID: u64 = 0x995f_9a33_77c0_b16e;

/// The type of a field, by node id where it names something.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ty {
    /// `Void`.
    Void,
    /// `Bool`.
    Bool,
    /// An integer: signedness and width.
    Int(bool, u8),
    /// `Float32` / `Float64`.
    Float(u8),
    /// `Text`.
    Text,
    /// `Data`.
    Data,
    /// `List(T)`.
    List(Box<Ty>),
    /// An enum node.
    Enum(u64),
    /// A struct node.
    Struct(u64),
    /// An interface node.
    Interface(u64),
    /// Not readable by the generator (`AnyPointer`, generics): the reason.
    Unreadable(&'static str),
}

/// A field is a slot or a group.
#[derive(Clone, Debug)]
pub enum FieldKind {
    /// A slot of this type.
    Slot(Ty),
    /// A group; its fields live in the struct node with this id.
    Group(u64),
}

/// One field of a struct or group, in declaration order.
#[derive(Clone, Debug)]
pub struct Field {
    /// Name as declared.
    pub name: String,
    /// Doc comment (empty if none).
    pub doc: String,
    /// Discriminant value if the field is a member of the union of its struct.
    pub union_member: Option<u16>,
    /// Slot or group.
    pub kind: FieldKind,
    /// All annotations of the field.
    pub annotations: Vec<Annotation>,
}

/// A struct node, also a group.
#[derive(Clone, Debug)]
pub struct StructNode {
    /// Declared name with its scopes, `Items.create$Params`.
    pub name: String,
    /// Is it a group (a struct that only exists inside another)?
    pub is_group: bool,
    /// Has the struct a union?
    pub has_union: bool,
    /// Is it generic (a type parameter somewhere above it)?
    pub generic: bool,
    /// Fields in declaration order.
    pub fields: Vec<Field>,
    /// Doc comment (empty if none).
    pub doc: String,
    /// All annotations of the struct.
    pub annotations: Vec<Annotation>,
}

/// An enum node.
#[derive(Clone, Debug)]
pub struct EnumNode {
    /// Enumerant names in schema order.
    pub names: Vec<String>,
}

/// One method; its ordinal is its place in the interface.
#[derive(Clone, Debug)]
pub struct Method {
    /// Ordinal.
    pub ordinal: u16,
    /// Name as declared.
    pub name: String,
    /// Doc comment.
    pub doc: String,
    /// Parameter struct node id.
    pub params: u64,
    /// Result struct node id.
    pub results: u64,
    /// `-> stream`.
    pub streaming: bool,
    /// All annotations of the method.
    pub annotations: Vec<Annotation>,
}

/// An interface node.
#[derive(Clone, Debug)]
pub struct InterfaceNode {
    /// File stem, `tasks`.
    pub file: String,
    /// Name with its scopes, `Items`.
    pub name: String,
    /// Doc comment.
    pub doc: String,
    /// Methods in ordinal order.
    pub methods: Vec<Method>,
    /// All annotations of the interface.
    pub annotations: Vec<Annotation>,
}

/// A file the compiler was asked for.
#[derive(Clone, Debug)]
pub struct RequestedFile {
    /// Node id of the file.
    pub id: u64,
    /// Name as given to the compiler (with `/`, never `\`).
    pub name: String,
    /// File stem with `-` as `_`, `tasks`.
    pub stem: String,
}

/// Everything the generator needs to know of a request.
#[derive(Debug, Default)]
pub struct Model {
    /// Files in the order of the request.
    pub files: Vec<RequestedFile>,
    /// Struct nodes by id.
    pub structs: BTreeMap<u64, StructNode>,
    /// Enum nodes by id.
    pub enums: BTreeMap<u64, EnumNode>,
    /// Interface nodes by id.
    pub interfaces: BTreeMap<u64, InterfaceNode>,
    /// Rust module path of every node as `capnpc` writes it, `["crate", "tasks_capnp", "items"]`.
    pub paths: BTreeMap<u64, Vec<String>>,
}

impl Model {
    /// The Rust path of a node, `crate::tasks_capnp::items`.
    ///
    /// # Panics
    /// The node is not in the model (an id from another model).
    #[must_use]
    pub fn path(&self, id: u64) -> String {
        self.paths[&id].join("::")
    }

    /// An interface by its declared name (`Tasks`).
    #[must_use]
    pub fn interface(&self, name: &str) -> Option<(u64, &InterfaceNode)> {
        self.interfaces
            .iter()
            .find(|(_, i)| i.name == name)
            .map(|(id, i)| (*id, i))
    }
}
