//! The helpers that write parameters into a builder, with the typed setters of capnpc-rust.
//!
//! Pattern: Template — one function per struct and group of the parameters. Only typed setters are used (capnpc-rust
//! applies the default of a slot itself); a value is written whenever the input has it, also if it is `0`.

use super::ctx::Ctx;
use super::text::Text;
use crate::cligen::model::{Field, FieldKind, Ty, capitalize, kebab, snake};

/// Can the generator write a value of this type?
#[must_use]
pub fn writable(ty: &Ty) -> bool {
    match ty {
        Ty::Interface(_) | Ty::Unreadable(_) | Ty::Void => false,
        Ty::List(e) => writable(e),
        _ => true,
    }
}

/// `set_<struct>(b, inp, path)`.
pub fn set_fn(ctx: &mut Ctx<'_>, id: u64) -> String {
    let st = ctx.st(id);
    let path = ctx.m.path(id);
    let name = ctx.ident("set", id);
    let mut w = Text::default();
    w.line("#[allow(unused_variables, unused_mut)]");
    w.open(format!(
        "fn {name}(mut b: {path}::Builder<'_>, inp: &mut dyn Input, path: &str) -> Result<(), CallError> {{"
    ));
    let members: Vec<&Field> = st
        .fields
        .iter()
        .filter(|f| f.union_member.is_some() && usable(f))
        .collect();
    if members.len() > 1 {
        let has: Vec<String> = members
            .iter()
            .map(|f| format!("inp.has(&join(path, {:?}))", kebab(&f.name)))
            .collect();
        w.line(format!(
            "if [{}].iter().filter(|x| **x).count() > 1 {{ return Err(CallError::union_twice(path)); }}",
            has.join(", ")
        ));
    }
    for f in st.fields.iter().filter(|f| usable(f)) {
        w.line(format!("let p = join(path, {:?});", kebab(&f.name)));
        field(ctx, &mut w, f);
    }
    w.line("Ok(())");
    w.close("}");
    w.finish()
}

fn usable(f: &Field) -> bool {
    match &f.kind {
        FieldKind::Group(_) => true,
        FieldKind::Slot(Ty::Void) => f.union_member.is_some(),
        FieldKind::Slot(t) => writable(t),
    }
}

fn field(ctx: &mut Ctx<'_>, w: &mut Text, f: &Field) {
    let sn = snake(&f.name);
    match &f.kind {
        FieldKind::Group(g) => {
            let s = ctx.setter(*g);
            w.line(format!(
                "if inp.has(&p) {{ {s}(b.reborrow().init_{sn}(), inp, &p)?; }}"
            ));
        }
        FieldKind::Slot(Ty::Void) => w.line(format!("if inp.has(&p) {{ b.set_{sn}(()); }}")),
        FieldKind::Slot(Ty::Struct(id)) => {
            let s = ctx.setter(*id);
            w.line(format!(
                "if inp.has(&p) {{ {s}(b.reborrow().init_{sn}(), inp, &p)?; }}"
            ));
        }
        FieldKind::Slot(Ty::List(el)) => {
            w.open("if let Some(n) = inp.count(&p)? {");
            w.line(format!("let mut l = b.reborrow().init_{sn}(n);"));
            elements(ctx, w, el, 0);
            w.close("}");
        }
        FieldKind::Slot(t) => scalar(ctx, w, t, "&p", &|v| format!("b.set_{sn}({v});")),
    }
}

/// The loop over the elements of a list that was just initialised as `l` (`l1` ... for inner lists).
fn elements(ctx: &mut Ctx<'_>, w: &mut Text, el: &Ty, depth: usize) {
    let suffix = if depth == 0 {
        String::new()
    } else {
        depth.to_string()
    };
    let (i, e, l, n) = (
        format!("i{depth}"),
        format!("e{depth}"),
        format!("l{suffix}"),
        format!("n{suffix}"),
    );
    let parent = if depth == 0 {
        "&p".to_owned()
    } else {
        format!("&e{}", depth - 1)
    };
    w.open(format!("for {i} in 0..{n} {{"));
    w.line(format!("let {e} = join({parent}, &{i}.to_string());"));
    match el {
        Ty::Struct(id) => {
            let s = ctx.setter(*id);
            w.line(format!("{s}({l}.reborrow().get({i}), inp, &{e})?;"));
        }
        Ty::List(inner) => {
            let d = depth + 1;
            w.open(format!("if let Some(n{d}) = inp.count(&{e})? {{"));
            w.line(format!("let mut l{d} = {l}.reborrow().init({i}, n{d});"));
            elements(ctx, w, inner, d);
            w.close("}");
        }
        t => scalar(ctx, w, t, &format!("&{e}"), &|v| {
            format!("{l}.set({i}, {v});")
        }),
    }
    w.close("}");
}

/// Read a scalar (also an enum) from the input and hand it to `set`, which gets the converted value as an expression.
fn scalar(ctx: &mut Ctx<'_>, w: &mut Text, t: &Ty, p: &str, set: &dyn Fn(&str) -> String) {
    match t {
        Ty::Enum(id) => {
            let names: Vec<String> = ctx.m.enums[id]
                .names
                .iter()
                .map(|n| format!("{:?}", kebab(n)))
                .collect();
            w.open(format!(
                "if let Some(ix) = inp.enumerant({p}, &[{}])? {{",
                names.join(", ")
            ));
            w.line(set(&enum_match(ctx, *id, p)));
        }
        Ty::Int(s, b) => {
            let rt = format!("{}{b}", if *s { 'i' } else { 'u' });
            w.open(format!("if let Some(v) = inp.int({p})? {{"));
            w.line(set(&format!(
                "{rt}::try_from(v).map_err(|_| CallError::out_of_range({p}, {rt:?}))?"
            )));
        }
        Ty::Bool => {
            w.open(format!("if let Some(v) = inp.boolean({p})? {{"));
            w.line(set("v"));
        }
        Ty::Float(b) => {
            w.open(format!("if let Some(v) = inp.float({p})? {{"));
            w.line(set(if *b == 32 { "v as f32" } else { "v" }));
        }
        Ty::Text => {
            w.open(format!("if let Some(v) = inp.text({p})? {{"));
            w.line(set("v.as_str()"));
        }
        Ty::Data => {
            w.open(format!("if let Some(v) = inp.data({p})? {{"));
            w.line(set("&v"));
        }
        _ => unreachable!("not a scalar: {t:?}"),
    }
    w.close("}");
}

/// `match ix { 0 => E::A, ... }` for an enum read as an index.
fn enum_match(ctx: &Ctx<'_>, id: u64, p: &str) -> String {
    let path = ctx.m.path(id);
    let arms: Vec<String> = ctx.m.enums[&id]
        .names
        .iter()
        .enumerate()
        .map(|(i, n)| format!("{i} => {path}::{},", capitalize(n)))
        .collect();
    format!(
        "match ix {{ {} _ => return Err(CallError::at({p}, ErrorKind::BadValue(\"enumerant\".into()))) }}",
        arms.join(" ")
    )
}
