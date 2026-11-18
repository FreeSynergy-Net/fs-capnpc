//! The server that receives the calls of a listener and hands each one to a `Sink`.
//!
//! Pattern: Adapter — the typed `Server` trait capnpc-rust wrote on one side, the one-line-per-call `Sink` of the
//! program on the other. The answer goes out after the sink took the line (back pressure) and never is an error:
//! a stream that failed once stays failed.

use super::ctx::Ctx;
use super::text::Text;
use crate::model::{capitalize, module_name};

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
    w.close("}");
    w.blank();
    w.open(format!("impl<S: Sink> {name}<S> {{"));
    w.open("pub fn new(sink: S) -> Self {");
    w.line("Self { sink: ::core::cell::RefCell::new(sink) }");
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
        w.open("if let Ok(p) = params.get() {");
        w.line(format!(
            "self.sink.borrow_mut().line({:?}, &mut |out| {out}(p, \"\", out));",
            crate::model::kebab(&me.name)
        ));
        w.close("}");
        w.line("::core::future::ready(Ok(()))");
        w.close("}");
    }
    w.close("}");
}
