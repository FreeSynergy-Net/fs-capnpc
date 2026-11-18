//! The static tables of one file: `COMMANDS`, `GROUPS` and `dispatch`.
//!
//! Pattern: Template — plain data rows; the program walks them for help, completion and the coverage check.

use super::ctx::{Ctx, lit, type_name};
use super::readers::elem_name;
use super::text::Text;
use crate::model::{Field, FieldKind, Model, Ty, kebab};
use crate::tree::{Command, Tree};

/// One row of the parameter table, as Rust text.
struct Row {
    path: String,
    ty: String,
    ty_name: String,
    union_of: Option<String>,
    hint: String,
    secret: bool,
    doc: String,
}

impl Row {
    fn text(&self) -> String {
        let union_of = self
            .union_of
            .as_ref()
            .map_or("None".to_owned(), |u| format!("Some({})", lit(u)));
        format!(
            "Param {{ path: {}, ty: {}, ty_name: {}, union_of: {union_of}, hint: {}, secret: {}, doc: {} }},",
            lit(&self.path),
            self.ty,
            lit(&self.ty_name),
            lit(&self.hint),
            self.secret,
            lit(&self.doc),
        )
    }
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{prefix}.{name}")
    }
}

/// The parameter rows of a struct (the params of a method, or a struct inside them).
fn rows(
    ctx: &Ctx<'_>,
    key: &str,
    sid: u64,
    prefix: &str,
    stack: &mut Vec<u64>,
    out: &mut Vec<Row>,
) {
    stack.push(sid);
    for f in &ctx.st(sid).fields {
        if matches!(f.kind, FieldKind::Slot(Ty::Void)) && f.union_member.is_none() {
            continue;
        }
        let path = join(prefix, &kebab(&f.name));
        let mut row = Row {
            path: path.clone(),
            ty: String::new(),
            ty_name: String::new(),
            union_of: f.union_member.map(|_| prefix.to_owned()),
            hint: f
                .annotations
                .iter()
                .find(|a| a.is_kind())
                .map(|a| a.value.clone())
                .or_else(|| ctx.marks.kind(key, &path))
                .unwrap_or_default(),
            secret: f
                .annotations
                .iter()
                .any(crate::model::Annotation::is_secret)
                || ctx.marks.secret(key, &path),
            doc: f.doc.trim().to_owned(),
        };
        match &f.kind {
            FieldKind::Group(g) => group(ctx, key, *g, &path, row, stack, out),
            FieldKind::Slot(ty) => slot(ctx, key, ty, &path, &mut row, stack, out),
        }
    }
    stack.pop();
}

fn group(
    ctx: &Ctx<'_>,
    key: &str,
    g: u64,
    path: &str,
    mut row: Row,
    stack: &mut Vec<u64>,
    out: &mut Vec<Row>,
) {
    row.ty = "ParamTy::Struct".into();
    out.push(row);
    rows(ctx, key, g, path, stack, out);
}

fn slot(
    ctx: &Ctx<'_>,
    key: &str,
    ty: &Ty,
    path: &str,
    row: &mut Row,
    stack: &mut Vec<u64>,
    out: &mut Vec<Row>,
) {
    fn done(out: &mut Vec<Row>, row: &Row, ty: String) {
        out.push(Row {
            path: row.path.clone(),
            ty,
            ty_name: row.ty_name.clone(),
            union_of: row.union_of.clone(),
            hint: row.hint.clone(),
            secret: row.secret,
            doc: row.doc.clone(),
        });
    }
    match ty {
        Ty::Void => done(out, row, "ParamTy::Void".into()),
        Ty::Bool => done(out, row, "ParamTy::Bool".into()),
        Ty::Int(s, b) => done(
            out,
            row,
            format!("ParamTy::Int {{ signed: {s}, bits: {b} }}"),
        ),
        Ty::Float(b) => done(out, row, format!("ParamTy::Float({b})")),
        Ty::Text => done(out, row, "ParamTy::Text".into()),
        Ty::Data => done(out, row, "ParamTy::Data".into()),
        Ty::Enum(id) => {
            row.ty_name = ctx.m.paths[id].last().cloned().unwrap_or_default();
            let names: Vec<String> = ctx.m.enums[id]
                .names
                .iter()
                .map(|n| lit(&kebab(n)))
                .collect();
            done(out, row, format!("ParamTy::Enum(&[{}])", names.join(", ")));
        }
        Ty::Unreadable(why) => {
            (*why).clone_into(&mut row.ty_name);
            done(out, row, "ParamTy::Unreadable".into());
        }
        Ty::Interface(_) => {
            row.ty_name = "capability".into();
            done(out, row, "ParamTy::Unreadable".into());
        }
        Ty::Struct(id) if stack.contains(id) => {
            row.ty_name = "recursive".into();
            done(out, row, "ParamTy::Unreadable".into());
        }
        Ty::Struct(id) => {
            type_name(ctx.st(*id)).clone_into(&mut row.ty_name);
            done(out, row, "ParamTy::Struct".into());
            rows(ctx, key, *id, path, stack, out);
        }
        Ty::List(el) => {
            row.ty_name = elem_name(ctx, el);
            done(out, row, "ParamTy::List".into());
            if let Ty::Struct(id) = &**el
                && !stack.contains(id)
            {
                rows(ctx, key, *id, &format!("{path}.*"), stack, out);
            }
        }
    }
}

/// The interfaces a command takes as parameters (listeners), in field order.
#[must_use]
pub fn listener_fields<'a>(m: &'a Model, c: &Command) -> Vec<(&'a Field, u64)> {
    m.structs[&c.method.params]
        .fields
        .iter()
        .filter_map(|f| match &f.kind {
            FieldKind::Slot(Ty::Interface(id)) => Some((f, *id)),
            _ => None,
        })
        .collect()
}

/// `COMMANDS`, `GROUPS` and `dispatch` for a tree, appended to `w`.
pub fn emit(ctx: &Ctx<'_>, w: &mut Text, t: &Tree, root_client: &str) {
    w.open("pub const COMMANDS: &[Command] = &[");
    for c in &t.commands {
        let mut rs = Vec::new();
        rows(ctx, &c.key, c.method.params, "", &mut Vec::new(), &mut rs);
        w.open("Command {");
        w.line(format!(
            "words: &[{}],",
            c.words
                .iter()
                .map(|x| lit(x))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        w.line(format!("key: {},", lit(&c.key)));
        w.line(format!("iface_id: {:#x},", c.iface));
        w.line(format!("ordinal: {},", c.method.ordinal));
        if rs.is_empty() {
            w.line("params: &[],");
        } else {
            w.open("params: &[");
            for r in &rs {
                w.line(r.text());
            }
            w.close("],");
        }
        w.line(format!("watch: {},", c.watch));
        w.line(format!("doc: {},", lit(c.method.doc.trim())));
        let ann: Vec<String> = c
            .method
            .annotations
            .iter()
            .map(|a| format!("({:#x}, {})", a.id, lit(&a.value)))
            .collect();
        w.line(format!("annotations: &[{}],", ann.join(", ")));
        w.close("},");
    }
    w.close("];");
    w.blank();
    w.open("pub const GROUPS: &[Group] = &[");
    for g in &t.groups {
        w.line(format!(
            "Group {{ words: &[{}], key: {} }},",
            g.words
                .iter()
                .map(|x| lit(x))
                .collect::<Vec<_>>()
                .join(", "),
            lit(&g.key)
        ));
    }
    w.close("];");
    w.blank();
    w.line(format!(
        "pub type Runner = for<'a> fn(&'a {root_client}, &'a mut dyn Input, &'a mut dyn Output) -> ::core::pin::Pin<Box<dyn ::core::future::Future<Output = Result<Held, CallError>> + 'a>>;"
    ));
    w.blank();
    w.line("/// The function of a command, by its words; `None` if there is no such command.");
    w.open("pub fn dispatch(words: &[&str]) -> Option<Runner> {");
    w.open("match words {");
    for c in &t.commands {
        let ws: Vec<String> = c.words.iter().map(|x| lit(x)).collect();
        w.line(format!(
            "[{}] => Some(|r, i, o| Box::pin({}(r, i, o))),",
            ws.join(", "),
            c.fn_name
        ));
    }
    w.line("_ => None,");
    w.close("}");
    w.close("}");
}
