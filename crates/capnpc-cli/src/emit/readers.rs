//! The helpers that walk a result into an `Output`.
//!
//! Pattern: Template — one function per struct, enum and group of the results; the recursion of the contract is the
//! recursion of the functions, so a struct that contains itself needs no special case.

use super::ctx::{Ctx, lit, type_name};
use super::text::Text;
use crate::model::{FieldKind, Ty, capitalize, kebab, snake};

/// `out_<struct>(r, name, out)`: begin, one value per field, the active member of the union, end.
pub fn out_fn(ctx: &mut Ctx<'_>, id: u64) -> String {
    let st = ctx.st(id);
    let path = ctx.m.path(id);
    let name = ctx.ident("out", id);
    let mut w = Text::default();
    w.line("#[allow(unused_variables, unused_mut)]");
    w.open(format!(
        "fn {name}(r: {path}::Reader<'_>, name: &str, out: &mut dyn Output) -> Result<(), CallError> {{"
    ));
    w.line(format!("out.begin_struct(name, {});", lit(type_name(st))));
    for f in st.fields.iter().filter(|f| f.union_member.is_none()) {
        field(ctx, &mut w, f, &format!("r.get_{}()", snake(&f.name)));
    }
    if st.has_union {
        w.open("match r.which()? {");
        for f in st.fields.iter().filter(|f| f.union_member.is_some()) {
            w.open(format!("{path}::Which::{}(v) => {{", capitalize(&f.name)));
            match &f.kind {
                FieldKind::Group(g) => {
                    let o = ctx.out(*g);
                    w.line(format!("{o}(v, {}, out)?;", lit(&f.name)));
                }
                FieldKind::Slot(ty) => value(ctx, &mut w, ty, "v", &lit(&f.name), false),
            }
            w.close("}");
        }
        w.close("}");
    }
    w.line("out.end();");
    w.line("Ok(())");
    w.close("}");
    w.finish()
}

/// One field of a struct that is not a union member; `getter` is the call that reads it.
fn field(ctx: &mut Ctx<'_>, w: &mut Text, f: &crate::model::Field, getter: &str) {
    match &f.kind {
        FieldKind::Group(g) => {
            let o = ctx.out(*g);
            w.line(format!("{o}({getter}, {}, out)?;", lit(&f.name)));
        }
        FieldKind::Slot(ty) => value(ctx, w, ty, getter, &lit(&f.name), false),
    }
}

/// Write the statements that hand the value `e` of type `ty` to the output under the name `n` (a Rust expression).
/// `e` has the form the getters of capnpc-rust give: the value itself for numbers, a `Result` for pointers and enums.
/// `direct_struct`: a struct element of a list comes without the `Result`.
pub fn value(ctx: &mut Ctx<'_>, w: &mut Text, ty: &Ty, e: &str, n: &str, direct_struct: bool) {
    match ty {
        Ty::Void => w.line(format!("out.field({n}, Value::Void);")),
        Ty::Bool => w.line(format!("out.field({n}, Value::Bool({e}));")),
        Ty::Int(true, 64) => w.line(format!("out.field({n}, Value::Int({e}));")),
        Ty::Int(true, _) => w.line(format!("out.field({n}, Value::Int(i64::from({e})));")),
        Ty::Int(false, 64) => w.line(format!("out.field({n}, Value::Uint({e}));")),
        Ty::Int(false, _) => w.line(format!("out.field({n}, Value::Uint(u64::from({e})));")),
        Ty::Float(64) => w.line(format!("out.field({n}, Value::Float({e}));")),
        Ty::Float(_) => w.line(format!("out.field({n}, Value::Float(f64::from({e})));")),
        Ty::Text => w.line(format!("out.field({n}, Value::Text({e}?.to_str()?));")),
        Ty::Data => w.line(format!("out.field({n}, Value::Data({e}?));")),
        Ty::Enum(id) => {
            let f = ctx.enum_fn(*id);
            w.line(format!(
                "out.field({n}, match {e} {{ Ok(v) => Value::Enum({f}(v)), Err(x) => Value::UnknownEnum(x.0) }});"
            ));
        }
        Ty::Struct(id) => {
            let o = ctx.out(*id);
            let q = if direct_struct { "" } else { "?" };
            w.line(format!("{o}({e}{q}, {n}, out)?;"));
        }
        Ty::List(el) => list(ctx, w, el, e, n),
        Ty::Interface(id) => {
            let name = ctx.m.interfaces[id].name.clone();
            w.line(format!(
                "out.field({n}, Value::Capability({}));",
                lit(&name)
            ));
        }
        Ty::Unreadable(why) => w.line(format!("out.field({n}, Value::Unreadable({}));", lit(why))),
    }
}

fn list(ctx: &mut Ctx<'_>, w: &mut Text, el: &Ty, e: &str, n: &str) {
    w.open("{");
    w.line(format!("let l = {e}?;"));
    w.line(format!(
        "out.begin_list({n}, {});",
        lit(&elem_name(ctx, el))
    ));
    w.open("for item in l.iter() {");
    value(ctx, w, el, "item", "\"\"", true);
    w.close("}");
    w.line("out.end();");
    w.close("}");
}

/// The name of the element type of a list, as the table spells it.
#[must_use]
pub fn elem_name(ctx: &Ctx<'_>, el: &Ty) -> String {
    match el {
        Ty::Void => "Void".into(),
        Ty::Bool => "Bool".into(),
        Ty::Int(s, b) => format!("{}{b}", if *s { "Int" } else { "UInt" }),
        Ty::Float(b) => format!("Float{b}"),
        Ty::Text => "Text".into(),
        Ty::Data => "Data".into(),
        Ty::List(_) => "List".into(),
        Ty::Enum(id) => ctx.m.paths[id].last().cloned().unwrap_or_default(),
        Ty::Struct(id) => type_name(ctx.st(*id)).to_owned(),
        Ty::Interface(id) => ctx.m.interfaces[id].name.clone(),
        Ty::Unreadable(why) => (*why).to_owned(),
    }
}

/// `enum_<enum>(v) -> &'static str`: the enumerant as a command line word.
pub fn enum_fn(ctx: &Ctx<'_>, id: u64) -> String {
    let path = ctx.m.path(id);
    let name = ctx.ident("enum", id);
    let mut w = Text::default();
    w.open(format!("fn {name}(v: {path}) -> &'static str {{"));
    w.open("match v {");
    for n in &ctx.m.enums[&id].names {
        w.line(format!("{path}::{} => {},", capitalize(n), lit(&kebab(n))));
    }
    w.close("}");
    w.close("}");
    w.finish()
}
