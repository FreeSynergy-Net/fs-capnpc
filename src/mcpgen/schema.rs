//! Contract types to JSON Schema (draft 2020-12, the dialect MCP uses), as text.
//!
//! Pattern: Visitor — one arm per [`Ty`] variant; the walk collects the `$defs` of every named struct once, so a struct
//! that contains itself ends. Property names are the names of the contract (`jscalendar`, `otherUid`); the command
//! line code of `cligen` spells the same names in kebab case (`other-uid`), and the run time maps between the two.

use std::collections::BTreeMap;

use super::json::{Json, obj, s};
use crate::cligen::MarkSource;
use crate::cligen::model::{Annotation, Field, FieldKind, Model, Ty, kebab};

/// The `format` of a text that carries a `kind` hint; any other hint is passed on as `x-fs-kind`.
fn format_of(kind: &str) -> Option<&'static str> {
    match kind {
        "uuid" => Some("uuid"),
        "timestamp" | "date-time" => Some("date-time"),
        "date" => Some("date"),
        "email" => Some("email"),
        "uri" | "url" => Some("uri"),
        _ => None,
    }
}

fn int(signed: bool, bits: u8) -> Json {
    let (min, max) = if signed {
        (
            format!("-{}", 1_u128 << (bits - 1)),
            ((1_u128 << (bits - 1)) - 1).to_string(),
        )
    } else {
        ("0".to_owned(), ((1_u128 << bits) - 1).to_string())
    };
    if bits == 64 {
        // the capnp-JSON codec writes 64-bit integers as strings (a number loses digits above 2^53)
        let pattern = if signed { "^-?[0-9]+$" } else { "^[0-9]+$" };
        let range = obj([
            ("signed", Json::Bool(signed)),
            ("bits", Json::Num("64".to_owned())),
            ("minimum", Json::Str(min)),
            ("maximum", Json::Str(max)),
        ]);
        return obj([
            ("type", s("string")),
            ("pattern", s(pattern)),
            ("x-fs-int", range),
        ]);
    }
    obj([
        ("type", s("integer")),
        ("minimum", Json::Num(min)),
        ("maximum", Json::Num(max)),
    ])
}

/// A float as the capnp-JSON codec writes it: a number, or one of three strings for infinity and not-a-number.
fn float() -> Json {
    let special = obj([
        ("type", s("string")),
        (
            "enum",
            Json::Arr(vec![s("Infinity"), s("-Infinity"), s("NaN")]),
        ),
    ]);
    obj([(
        "anyOf",
        Json::Arr(vec![obj([("type", s("number"))]), special]),
    )])
}

/// Does the capnp-JSON codec always write this field? Numbers, booleans and enums live in the data section; a pointer
/// that is null is left out.
fn always_written(kind: &FieldKind) -> bool {
    matches!(
        kind,
        FieldKind::Slot(Ty::Bool | Ty::Int(..) | Ty::Float(_) | Ty::Enum(_))
    )
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{prefix}.{name}")
    }
}

/// The key of a struct in `$defs`: `common.Cursor` (file stem, the name with its scopes).
fn def_key(display: &str) -> String {
    let (file, name) = display.split_once(':').unwrap_or(("", display));
    let stem = file.rsplit('/').next().unwrap_or(file);
    let stem = stem.trim_end_matches(".capnp").replace('-', "_");
    if stem.is_empty() {
        name.to_owned()
    } else {
        format!("{stem}.{name}")
    }
}

/// Builds one schema document.
pub struct Schema<'a> {
    m: &'a Model,
    marks: &'a dyn MarkSource,
    key: &'a str,
    /// An input schema is strict (closed, every field required, a union is `oneOf`); a result is lax, because the
    /// capnp-JSON codec leaves out null pointers and the default member of a union.
    closed: bool,
    defs: BTreeMap<String, Json>,
}

impl<'a> Schema<'a> {
    /// A builder for the parameters of the command `key` (the marks apply), or for its results.
    #[must_use]
    pub fn new(m: &'a Model, marks: &'a dyn MarkSource, key: &'a str, closed: bool) -> Self {
        Self {
            m,
            marks,
            key,
            closed,
            defs: BTreeMap::new(),
        }
    }

    /// The schema of the struct `sid` (the parameters or results of a method). With `only`, a union struct is cut down
    /// to that one member (the tool of one member). The `$defs` of what it names are attached.
    #[must_use]
    pub fn root(mut self, sid: u64, only: Option<&str>) -> Json {
        let path = self.closed.then_some("");
        let mut root = self.object(sid, only, path, "");
        if !self.defs.is_empty() {
            root.put("$defs", Json::Obj(self.defs.into_iter().collect()));
        }
        root
    }

    fn ty(&mut self, t: &Ty, path: Option<&str>) -> Json {
        match t {
            Ty::Void => obj([("type", s("null"))]),
            Ty::Bool => obj([("type", s("boolean"))]),
            Ty::Int(signed, bits) => int(*signed, *bits),
            Ty::Float(_) => float(),
            Ty::Text => obj([("type", s("string"))]),
            Ty::Data => obj([("type", s("string")), ("contentEncoding", s("base64"))]),
            Ty::List(e) => {
                let inner = path.map(|p| format!("{p}.*"));
                let items = self.ty(e, inner.as_deref());
                obj([("type", s("array")), ("items", items)])
            }
            Ty::Enum(id) => {
                let names = self.m.enums[id].names.iter().map(|n| s(n)).collect();
                obj([("type", s("string")), ("enum", Json::Arr(names))])
            }
            Ty::Struct(id) => self.struct_ref(*id),
            // a tool that has one of these is not generated (see `scan`); an open schema is the honest answer here
            Ty::Interface(_) | Ty::Unreadable(_) => Json::Obj(Vec::new()),
        }
    }

    fn struct_ref(&mut self, id: u64) -> Json {
        let m = self.m;
        let st = &m.structs[&id];
        let key = def_key(&st.name);
        if !self.defs.contains_key(&key) {
            // the placeholder ends the walk of a struct that contains itself
            self.defs.insert(key.clone(), Json::Obj(Vec::new()));
            let body = self.object(id, None, None, st.doc.trim());
            self.defs.insert(key.clone(), body);
        }
        obj([("$ref", Json::Str(format!("#/$defs/{key}")))])
    }

    fn field(&mut self, f: &Field, path: Option<&str>) -> Json {
        let fp = path.map(|p| join(p, &kebab(&f.name)));
        let mut j = match &f.kind {
            FieldKind::Slot(t) => self.ty(t, fp.as_deref()),
            FieldKind::Group(g) => self.object(*g, None, fp.as_deref(), ""),
        };
        let kind = f
            .annotations
            .iter()
            .find(|a| a.is_kind())
            .map(|a| a.value.clone())
            .or_else(|| fp.as_deref().and_then(|p| self.marks.kind(self.key, p)));
        if let Some(k) = kind {
            let is_text = matches!(&f.kind, FieldKind::Slot(Ty::Text));
            match format_of(&k).filter(|_| is_text) {
                Some(fmt) => j.put("format", s(fmt)),
                None => j.put("x-fs-kind", Json::Str(k)),
            }
        }
        // (the path is only there for the arguments, so the marks of a program never reach a result)
        // sensitive data of a person: the program that serves the tools lets it out only to a receiver with a release
        let marked = fp
            .as_deref()
            .is_some_and(|p| self.marks.sensitive(self.key, p));
        if marked || f.annotations.iter().any(Annotation::is_sensitive) {
            j.put("x-fs-sensitive", Json::Bool(true));
        }
        if !f.doc.trim().is_empty() {
            j.put("description", s(f.doc.trim()));
        }
        j
    }

    /// An object schema over a struct: the members of a union are alternatives (exactly one is set), a field that is
    /// not in a union is required, a `Void` field that is not in a union says nothing and is left out.
    fn object(&mut self, sid: u64, only: Option<&str>, path: Option<&str>, doc: &str) -> Json {
        let m = self.m;
        let st = &m.structs[&sid];
        let (mut props, mut required, mut alternatives) = (Vec::new(), Vec::new(), Vec::new());
        for f in &st.fields {
            let member = f.union_member.is_some();
            if only.is_some_and(|o| o != f.name) {
                continue;
            }
            if !member && matches!(f.kind, FieldKind::Slot(Ty::Void)) {
                continue;
            }
            props.push((f.name.clone(), self.field(f, path)));
            if only.is_some() || !member {
                // a result is lax: the codec leaves out a null pointer, so only what it always writes is required
                if self.closed || always_written(&f.kind) {
                    required.push(s(&f.name));
                }
            } else if self.closed {
                alternatives.push(obj([("required", Json::Arr(vec![s(&f.name)]))]));
            }
        }
        let mut o = obj([("type", s("object"))]);
        if !doc.is_empty() {
            o.put("description", s(doc));
        }
        if !props.is_empty() {
            o.put("properties", Json::Obj(props));
        }
        if !required.is_empty() {
            o.put("required", Json::Arr(required));
        }
        if self.closed {
            o.put("additionalProperties", Json::Bool(false));
        }
        if !alternatives.is_empty() {
            o.put("oneOf", Json::Arr(alternatives));
        }
        o
    }
}
