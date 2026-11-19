//! Annotations of the contract: read as (id, value) pairs, the known ones by name.
//!
//! Pattern: Adapter — every annotation is kept, so a mark of another tool is passed on and nothing is lost.

use capnp::schema_capnp::{annotation, value};

/// Annotation id of `secret` in `capnpc_cli.capnp`.
pub const SECRET_ANNOTATION: u64 = 0xefac_3d50_46ac_feb0;
/// Annotation id of `sensitive` in `capnpc_cli.capnp`.
pub const SENSITIVE_ANNOTATION: u64 = 0x8d83_06a2_e2dc_31b6;
/// Annotation id of `kind` in `capnpc_cli.capnp`.
pub const KIND_ANNOTATION: u64 = 0xe63f_c325_3809_b912;
/// Annotation id of `risk` in `capnpc_cli.capnp`.
pub const RISK_ANNOTATION: u64 = 0xf0a9_fc9d_3d3d_3fb5;

/// The names of the enumerants of `Danger` in `capnpc_cli.capnp`.
const RISK_NAMES: &[&str] = &[
    "read",
    "add",
    "change",
    "destructive",
    "actsInName",
    "excluded",
];

/// One annotation: its id and its value as text (empty for `Void`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Annotation {
    /// Annotation id.
    pub id: u64,
    /// The value: `true`/`false`, a number, the text, the name of a `Danger`; `?` for a struct or list.
    pub value: String,
}

fn text_of(v: value::Reader<'_>, id: u64) -> capnp::Result<String> {
    use value::Which as W;
    Ok(match v.which()? {
        W::Void(()) => String::new(),
        W::Bool(b) => b.to_string(),
        W::Int8(n) => n.to_string(),
        W::Int16(n) => n.to_string(),
        W::Int32(n) => n.to_string(),
        W::Int64(n) => n.to_string(),
        W::Uint8(n) => n.to_string(),
        W::Uint16(n) => n.to_string(),
        W::Uint32(n) => n.to_string(),
        W::Uint64(n) => n.to_string(),
        W::Float32(n) => n.to_string(),
        W::Float64(n) => n.to_string(),
        W::Text(t) => t?.to_str()?.to_owned(),
        W::Enum(n) if id == RISK_ANNOTATION => RISK_NAMES
            .get(usize::from(n))
            .map_or_else(|| n.to_string(), |s| (*s).to_owned()),
        W::Enum(n) => n.to_string(),
        _ => "?".to_owned(),
    })
}

/// Read a list of annotations.
///
/// # Errors
/// An annotation is malformed.
pub fn read_all(
    list: capnp::struct_list::Reader<'_, annotation::Owned>,
) -> capnp::Result<Vec<Annotation>> {
    let mut out = Vec::new();
    for a in list {
        out.push(Annotation {
            id: a.get_id(),
            value: text_of(a.get_value()?, a.get_id())?,
        });
    }
    out.sort_by_key(|a| a.id);
    Ok(out)
}

impl Annotation {
    /// Is it the `kind` hint?
    #[must_use]
    pub fn is_kind(&self) -> bool {
        self.id == KIND_ANNOTATION
    }

    /// Is it the `sensitive` mark?
    #[must_use]
    pub fn is_sensitive(&self) -> bool {
        self.id == SENSITIVE_ANNOTATION
    }

    /// Is it the `secret` mark?
    #[must_use]
    pub fn is_secret(&self) -> bool {
        self.id == SECRET_ANNOTATION
    }
}
