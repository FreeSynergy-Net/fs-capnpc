//! The contract as plain owned values, read once from the `CodeGeneratorRequest`.
//!
//! Pattern: Adapter — the foreign form (public `capnp::schema_capnp`) is read in `read` and nowhere else; the tree and
//! the emitters work on these types. Everything is kept in sorted maps, so the output never depends on hash order.

mod annotation;
mod names;
mod read;
mod scope;
mod types;

pub use annotation::{Annotation, KIND_ANNOTATION, RISK_ANNOTATION, SECRET_ANNOTATION};
pub use names::{capitalize, kebab, module_name, snake};
pub use types::{
    EnumNode, Field, FieldKind, InterfaceNode, Method, Model, RequestedFile, STREAM_RESULT_ID,
    StructNode, Ty,
};
