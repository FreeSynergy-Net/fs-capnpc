//! Read a `CodeGeneratorRequest` into the model.
//!
//! Pattern: Adapter — the only place that touches `capnp::schema_capnp`.

use std::collections::HashMap;

use capnp::schema_capnp::{code_generator_request, field, node, type_};

use super::annotation::read_all;
use super::scope::Scopes;
use super::{
    EnumNode, Field, FieldKind, InterfaceNode, Method, Model, RequestedFile, STREAM_RESULT_ID,
    StructNode, Ty,
};

fn text(r: capnp::Result<capnp::text::Reader<'_>>) -> capnp::Result<String> {
    Ok(r?.to_str()?.to_owned())
}

fn is_generic_node(nodes: &HashMap<u64, node::Reader<'_>>, id: u64) -> bool {
    nodes.get(&id).is_some_and(|n| n.get_is_generic())
}

fn ty(t: type_::Reader<'_>, nodes: &HashMap<u64, node::Reader<'_>>) -> capnp::Result<Ty> {
    use type_::Which as W;
    Ok(match t.which()? {
        W::Void(()) => Ty::Void,
        W::Bool(()) => Ty::Bool,
        W::Int8(()) => Ty::Int(true, 8),
        W::Int16(()) => Ty::Int(true, 16),
        W::Int32(()) => Ty::Int(true, 32),
        W::Int64(()) => Ty::Int(true, 64),
        W::Uint8(()) => Ty::Int(false, 8),
        W::Uint16(()) => Ty::Int(false, 16),
        W::Uint32(()) => Ty::Int(false, 32),
        W::Uint64(()) => Ty::Int(false, 64),
        W::Float32(()) => Ty::Float(32),
        W::Float64(()) => Ty::Float(64),
        W::Text(()) => Ty::Text,
        W::Data(()) => Ty::Data,
        W::List(l) => Ty::List(Box::new(ty(l.get_element_type()?, nodes)?)),
        W::Enum(e) => Ty::Enum(e.get_type_id()),
        W::Struct(s) if is_generic_node(nodes, s.get_type_id()) => Ty::Unreadable("generic"),
        W::Struct(s) => Ty::Struct(s.get_type_id()),
        W::Interface(i) if is_generic_node(nodes, i.get_type_id()) => Ty::Unreadable("generic"),
        W::Interface(i) => Ty::Interface(i.get_type_id()),
        W::AnyPointer(_) => Ty::Unreadable("any-pointer"),
    })
}

impl Model {
    /// Read a serialized `CodeGeneratorRequest` (what `capnp-compiler` or `capnp compile -o-` emits).
    ///
    /// # Errors
    /// The bytes are not a request, or a node is malformed.
    pub fn from_request(bytes: &[u8]) -> capnp::Result<Self> {
        let opts = capnp::message::ReaderOptions {
            traversal_limit_in_words: None,
            nesting_limit: 128,
        };
        let msg = capnp::serialize::read_message(&mut &bytes[..], opts)?;
        Self::from_reader(msg.get_root()?)
    }

    fn from_reader(req: code_generator_request::Reader<'_>) -> capnp::Result<Self> {
        let mut docs: HashMap<u64, (String, Vec<String>)> = HashMap::new();
        for info in req.get_source_info()? {
            let members = info
                .get_members()?
                .iter()
                .map(|m| text(m.get_doc_comment()))
                .collect::<Result<_, _>>()?;
            docs.insert(info.get_id(), (text(info.get_doc_comment())?, members));
        }
        let nodes: HashMap<u64, node::Reader<'_>> =
            req.get_nodes()?.iter().map(|n| (n.get_id(), n)).collect();
        let mut m = Self::default();
        let mut scopes = Scopes::new(&nodes);
        for f in req.get_requested_files()? {
            let name = text(f.get_filename())?.replace('\\', "/");
            let stem = stem_of(&name);
            for imp in f.get_imports()? {
                scopes.add_file(imp.get_id(), &stem_of(&text(imp.get_name())?))?;
            }
            scopes.add_file(f.get_id(), &stem)?;
            m.files.push(RequestedFile {
                id: f.get_id(),
                name,
                stem,
            });
        }
        scopes.add_anonymous()?;
        m.paths = std::mem::take(&mut scopes.paths);
        for n in req.get_nodes()? {
            m.add_node(n, &nodes, &docs)?;
        }
        Ok(m)
    }

    fn add_node(
        &mut self,
        n: node::Reader<'_>,
        nodes: &HashMap<u64, node::Reader<'_>>,
        docs: &HashMap<u64, (String, Vec<String>)>,
    ) -> capnp::Result<()> {
        let id = n.get_id();
        let display = text(n.get_display_name())?.replace('\\', "/");
        let (file_part, nice) = display.split_once(':').unwrap_or(("", &display));
        let file = stem_of(file_part);
        // `sourceInfo.members` is indexed by the code order of the member
        let doc_of = |code_order: u16| {
            docs.get(&id)
                .and_then(|d| d.1.get(usize::from(code_order)).cloned())
                .unwrap_or_default()
        };
        match n.which()? {
            node::Which::Struct(s) => {
                let mut fields = Vec::new();
                for f in s.get_fields()? {
                    let kind = match f.which()? {
                        field::Which::Slot(sl) => FieldKind::Slot(ty(sl.get_type()?, nodes)?),
                        field::Which::Group(g) => FieldKind::Group(g.get_type_id()),
                    };
                    let disc = f.get_discriminant_value();
                    fields.push((
                        f.get_code_order(),
                        Field {
                            name: text(f.get_name())?,
                            doc: doc_of(f.get_code_order()),
                            union_member: (disc != 0xffff).then_some(disc),
                            kind,
                            annotations: read_all(f.get_annotations()?)?,
                        },
                    ));
                }
                fields.sort_by_key(|f| f.0);
                self.structs.insert(
                    id,
                    StructNode {
                        name: display.clone(),
                        is_group: s.get_is_group(),
                        has_union: s.get_discriminant_count() > 0,
                        generic: n.get_is_generic(),
                        fields: fields.into_iter().map(|f| f.1).collect(),
                    },
                );
            }
            node::Which::Enum(e) => {
                let names = e
                    .get_enumerants()?
                    .iter()
                    .map(|x| text(x.get_name()))
                    .collect::<Result<_, _>>()?;
                self.enums.insert(id, EnumNode { names });
            }
            node::Which::Interface(i) => {
                let mut methods = Vec::new();
                for (ix, me) in i.get_methods()?.iter().enumerate() {
                    methods.push(Method {
                        ordinal: u16::try_from(ix)
                            .map_err(|_| capnp::Error::failed("too many methods".into()))?,
                        name: text(me.get_name())?,
                        doc: doc_of(me.get_code_order()),
                        params: me.get_param_struct_type(),
                        results: me.get_result_struct_type(),
                        streaming: me.get_result_struct_type() == STREAM_RESULT_ID,
                        annotations: read_all(me.get_annotations()?)?,
                    });
                }
                self.interfaces.insert(
                    id,
                    InterfaceNode {
                        file,
                        name: nice.to_owned(),
                        doc: docs.get(&id).map(|d| d.0.clone()).unwrap_or_default(),
                        methods,
                        annotations: read_all(n.get_annotations()?)?,
                    },
                );
            }
            _ => {}
        }
        Ok(())
    }
}

/// `schema/my-file.capnp` -> `my_file`.
fn stem_of(path: &str) -> String {
    let last = path.rsplit('/').next().unwrap_or(path);
    last.trim_end_matches(".capnp").replace('-', "_")
}
