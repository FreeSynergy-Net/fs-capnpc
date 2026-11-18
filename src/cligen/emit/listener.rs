//! The server that receives the calls of a listener and hands each one to a `Sink`.
//!
//! Pattern: Adapter — the typed `Server` trait capnpc-rust wrote on one side, the one-line-per-call `Sink` of the
//! program on the other. The answer goes out after the sink took the line (back pressure) and never is an error:
//! a stream that failed once stays failed. A call that cannot be read is counted by the listener itself, before the
//! sink sees the line (the arguments are walked once into `Discard`), so no sink can hide it, not even one that never
//! calls `render`.

use super::command::RT_PATH;
use super::ctx::Ctx;
use super::text::Text;
use crate::cligen::model::{capitalize, module_name};

/// `ChangeListenerLines` for the interface `id`.
#[must_use]
pub fn lines_name(ctx: &Ctx<'_>, id: u64) -> String {
    format!("{}Lines", ctx.m.interfaces[&id].name.replace('.', ""))
}

/// The struct and the `Server` impl for the interface `id`.
pub fn emit(ctx: &mut Ctx<'_>, w: &mut Text, id: u64) {
    let name = lines_name(ctx, id);
    let path = ctx.m.path(id);
    w.line(format!(
        "/// Receives the calls of `{}` and hands each to the sink as one line.",
        ctx.m.interfaces[&id].name
    ));
    w.open(format!("pub struct {name}<S: Sink> {{"));
    w.line("sink: ::core::cell::RefCell<S>,");
    w.line("errors: ::std::rc::Rc<::core::cell::Cell<u32>>,");
    w.close("}");
    w.blank();
    w.open(format!("impl<S: Sink> {name}<S> {{"));
    w.open("pub fn new(sink: S) -> Self {");
    w.line("Self { sink: ::core::cell::RefCell::new(sink), errors: ::std::rc::Rc::default() }");
    w.close("}");
    w.blank();
    w.line(
        "/// How many calls could not be read: counted here, whatever the sink does with the line.",
    );
    w.open("pub fn errors(&self) -> ::std::rc::Rc<::core::cell::Cell<u32>> {");
    w.line("self.errors.clone()");
    w.close("}");
    w.close("}");
    w.blank();
    w.open(format!("impl<S: Sink> {path}::Server for {name}<S> {{"));
    for me in ctx.m.interfaces[&id].methods.clone() {
        let cap = capitalize(&me.name);
        let out = ctx.out(me.params);
        let results = if me.streaming {
            String::new()
        } else {
            format!(", _results: {path}::{cap}Results")
        };
        w.open(format!(
            "fn {}(self: ::capnp::capability::Rc<Self>, params: {path}::{cap}Params{results}) -> impl ::core::future::Future<Output = Result<(), ::capnp::Error>> + 'static {{",
            module_name(&me.name)
        ));
        let kind = format!("{:?}", crate::cligen::model::kebab(&me.name));
        w.open("match params.get() {");
        w.line(format!(
            "Ok(p) => {{ if {out}(p, \"\", &mut {RT_PATH}::Discard).is_err() {{ self.errors.set(self.errors.get() + 1); }} self.sink.borrow_mut().line({kind}, &mut |out| {out}(p, \"\", out)); }}"
        ));
        w.line(format!(
            "Err(e) => {{ self.errors.set(self.errors.get() + 1); let why = e.to_string(); self.sink.borrow_mut().line({kind}, &mut |_| Err(CallError::at(\"\", ErrorKind::BadValue(why.clone())))); }}"
        ));
        w.close("}");
        w.line("::core::future::ready(Ok(()))");
        w.close("}");
    }
    w.close("}");
}
