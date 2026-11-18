//! The Rust module path of every node, exactly as `capnpc` derives it.
//!
//! Pattern: Strategy — a port of the scope walk of capnpc-rust (file roots, nested nodes, groups, the anonymous
//! parameter and result structs of methods), because the generated calls name the types capnpc wrote.

use std::collections::{BTreeMap, HashMap};

use capnp::schema_capnp::{field, node};

use super::names::module_name;

/// Annotation id of `$Rust.name` in `rust.capnp`.
const NAME_ANNOTATION_ID: u64 = 0xc2fe_4c6d_1001_66d0;
/// Annotation id of `$Rust.parentModule` in `rust.capnp`.
const PARENT_MODULE_ANNOTATION_ID: u64 = 0xabee_386c_d145_0364;

/// How a node is named in Rust.
#[derive(Clone, Copy, PartialEq, Eq)]
enum NameKind {
    Verbatim,
    Module,
}

type Nodes<'a> = HashMap<u64, node::Reader<'a>>;

/// The scope map of one request.
pub struct Scopes<'a> {
    nodes: &'a Nodes<'a>,
    /// Path of every node.
    pub paths: BTreeMap<u64, Vec<String>>,
}

impl<'a> Scopes<'a> {
    /// A walk over `nodes`.
    pub fn new(nodes: &'a Nodes<'a>) -> Self {
        Self {
            nodes,
            paths: BTreeMap::new(),
        }
    }

    /// Add a file (requested or imported): its root module is `<stem>_capnp` below `crate`, or below `::<name>` when
    /// `provided_by` names the crate that holds the code `capnpc` wrote for this file (as `crates_provide_map` of `capnpc`).
    ///
    /// # Errors
    /// A node is malformed or uses an annotation the generator cannot follow.
    pub fn add_file(
        &mut self,
        id: u64,
        stem: &str,
        provided_by: Option<&str>,
    ) -> capnp::Result<()> {
        let root = format!("{}_capnp", stem.replace('-', "_"));
        let base = provided_by.map_or_else(|| "crate".to_owned(), |c| format!("::{c}"));
        self.walk(vec![base], root, NameKind::Verbatim, id)
    }

    fn walk(
        &mut self,
        mut ancestors: Vec<String>,
        mut name: String,
        kind: NameKind,
        id: u64,
    ) -> capnp::Result<()> {
        let Some(&n) = self.nodes.get(&id) else {
            return Ok(()); // an unused node of an imported file may be missing
        };
        for a in n.get_annotations()? {
            if a.get_id() == NAME_ANNOTATION_ID {
                if let capnp::schema_capnp::value::Which::Text(t) = a.get_value()?.which()? {
                    t?.to_str()?.clone_into(&mut name);
                }
            } else if a.get_id() == PARENT_MODULE_ANNOTATION_ID {
                return Err(capnp::Error::failed(
                    "the annotation $Rust.parentModule is not supported by capnpc-cli".into(),
                ));
            }
        }
        ancestors.push(match kind {
            NameKind::Verbatim => name.clone(),
            NameKind::Module => module_name(&name),
        });
        self.paths.insert(id, ancestors.clone());
        for nested in n.get_nested_nodes()? {
            let k = match self.nodes.get(&nested.get_id()).map(|x| x.which()) {
                Some(Ok(node::Which::Enum(_))) => NameKind::Verbatim,
                _ => NameKind::Module,
            };
            self.walk(
                ancestors.clone(),
                nested.get_name()?.to_str()?.to_owned(),
                k,
                nested.get_id(),
            )?;
        }
        if let Ok(node::Which::Struct(s)) = n.which() {
            for f in s.get_fields()? {
                if let Ok(field::Which::Group(g)) = f.which() {
                    self.walk(
                        ancestors.clone(),
                        f.get_name()?.to_str()?.to_owned(),
                        NameKind::Module,
                        g.get_type_id(),
                    )?;
                }
            }
        }
        Ok(())
    }

    /// The anonymous parameter and result structs of the methods of interface `id` live below it, as
    /// `<method>_params` and `<method>_results`.
    ///
    /// # Errors
    /// A node is malformed.
    pub fn add_anonymous(&mut self) -> capnp::Result<()> {
        let mut ids: Vec<u64> = self.nodes.keys().copied().collect();
        ids.sort_unstable();
        for id in ids {
            let n = self.nodes[&id];
            let node::Which::Interface(i) = n.which()? else {
                continue;
            };
            let Some(base) = self.paths.get(&id).cloned() else {
                continue;
            };
            for m in i.get_methods()? {
                let mname = m.get_name()?.to_str()?;
                for (sid, suffix) in [
                    (m.get_param_struct_type(), "Params"),
                    (m.get_result_struct_type(), "Results"),
                ] {
                    let anonymous = self.nodes.get(&sid).is_some_and(|s| s.get_scope_id() == 0);
                    if anonymous && !self.paths.contains_key(&sid) {
                        let mut p = base.clone();
                        p.push(module_name(&format!("{mname}{suffix}")));
                        self.paths.insert(sid, p);
                    }
                }
            }
        }
        Ok(())
    }
}
