//! The `run_*` function of a command: navigate, set the parameters, call, walk the result.
//!
//! Pattern: Template — one text template per command; the call goes through the typed client capnpc-rust wrote
//! (`root.items_request()`), never through a run-time schema and never through `capnp::private`.

use super::ctx::Ctx;
use super::table::listener_fields;
use super::text::Text;
use crate::model::snake;
use crate::tree::Command;

/// The function of one command.
pub fn run_fn(ctx: &mut Ctx<'_>, w: &mut Text, c: &Command, root_client: &str) {
    let doc = c.method.doc.lines().next().unwrap_or("").trim();
    w.line(format!("/// `{}` {doc}", c.key).trim_end());
    w.open(format!(
        "pub async fn {}(root: &{root_client}, inp: &mut dyn Input, out: &mut dyn Output) -> Result<Held, CallError> {{",
        c.fn_name
    ));
    let mut client = "root".to_owned();
    for (i, st) in c.path.iter().enumerate() {
        w.line(format!(
            "let c{i} = {client}.{}_request().send().pipeline.get_{}();",
            snake(&st.method),
            snake(&st.field)
        ));
        client = format!("c{i}");
    }
    w.line(format!(
        "let mut req = {client}.{}_request();",
        snake(&c.method.name)
    ));
    let set = ctx.setter(c.method.params);
    if c.method.streaming {
        w.line(format!("{set}(req.get(), inp, \"\")?;"));
        w.line("req.send().await?;");
        w.line("let _ = out;");
        w.line("Ok(Held::none())");
        w.close("}");
        return;
    }
    w.line(format!("{set}(req.get(), inp, \"\")?;"));
    for (f, lid) in listener_fields(ctx.m, c) {
        let lines = super::listener::lines_name(ctx, lid);
        let lpath = ctx.m.path(lid);
        w.line("let sink = out.sink().ok_or_else(|| CallError::at(\"\", ErrorKind::Unsupported(\"watching needs a sink\")))?;");
        w.line(format!(
            "req.get().set_{}(::capnp_rpc::new_client::<{lpath}::Client, _>({lines}::new(sink)));",
            snake(&f.name)
        ));
    }
    w.line("let resp = req.send().promise.await?;");
    let o = ctx.out(c.method.results);
    w.line(format!("{o}(resp.get()?, \"\", out)?;"));
    if c.watch {
        w.line("Ok(Held::new(resp))");
    } else {
        w.line("Ok(Held::none())");
    }
    w.close("}");
}
