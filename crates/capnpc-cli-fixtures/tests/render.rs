//! A7 (B1/B2): a call that is readable at the top but fails only while it is rendered (a `Page` where the `Shape`
//! belongs: its label is a list of structs) is counted by the generated listener, whatever the sink does: it shows
//! the line, swallows the error, or never calls `render` at all.

mod support;

use std::future::Future;
use std::rc::Rc;

use capnpc_cli::rt::{CallError, Output, Sink, Value};
use capnpc_cli_fixtures::features_capnp::features;
use capnpc_cli_fixtures::features_cli as cli;
use capnpc_cli_fixtures::ids;
use support::{In, Rec, run};

/// What the fake sends to the sink.
#[derive(Clone, Copy)]
enum Send {
    /// A good event.
    Good,
    /// An event that is readable at the top and fails while rendering.
    Deep,
    /// An event whose arguments are a text where a struct belongs (fails at once).
    Top,
}

struct Server(Send);

impl features::Server for Server {
    fn watch(
        self: capnp::capability::Rc<Self>,
        params: features::WatchParams,
        _: features::WatchResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        let sink = params
            .get()
            .and_then(features::watch_params::Reader::get_sink);
        let what = self.0;
        async move {
            let sink = sink?;
            match what {
                Send::Good => {
                    let mut ev = sink.event_request();
                    ev.get().init_shape().set_label("fine");
                    ev.send().await?;
                }
                Send::Deep => {
                    let mut ev = sink.client.new_streaming_call::<capnp::any_pointer::Owned>(
                        ids::SINK,
                        0,
                        None,
                    );
                    // params.get() is fine; the label of the shape is a struct list, not a text
                    ev.get()
                        .init_as::<features::page_results::Builder<'_>>()
                        .init_page()
                        .init_shapes(1);
                    ev.send().await?;
                }
                Send::Top => {
                    let mut ev = sink.client.new_streaming_call::<capnp::any_pointer::Owned>(
                        ids::SINK,
                        0,
                        None,
                    );
                    ev.get().set_as::<capnp::text::Owned>("not a struct")?;
                    ev.send().await?;
                }
            }
            Ok(())
        }
    }
}

/// A sink that drops the line without ever calling `render`.
struct NoRender;

impl Sink for NoRender {
    fn line(&mut self, _: &str, _: &mut dyn FnMut(&mut dyn Output) -> Result<(), CallError>) {}
}

/// Output that hands out one given sink.
struct Hands(Option<Box<dyn Sink>>);

impl Output for Hands {
    fn begin_struct(&mut self, _: &str, _: &str) {}
    fn begin_list(&mut self, _: &str, _: &str) {}
    fn field(&mut self, _: &str, _: Value<'_>) {}
    fn end(&mut self) {}
    fn sink(&mut self) -> Option<Box<dyn Sink>> {
        self.0.take()
    }
}

fn code(what: Send, out: &mut dyn Output) -> i32 {
    let c: features::Client = capnp_rpc::new_client(Server(what));
    run(cli::run_to_exit(
        &c,
        &["watch"],
        &mut In::default(),
        out,
        || async { Ok(()) },
    ))
}

#[test]
fn a_call_that_fails_only_while_rendering_is_counted_with_a_recording_sink() {
    for round in 0..10 {
        let log = Rc::default();
        let mut out = Rec {
            sink: Some(Rc::clone(&log)),
            ..Rec::default()
        };
        assert_eq!(code(Send::Deep, &mut out), 3, "run {round}");
        let lines = log.borrow();
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].starts_with("event: ERR "), "{}", lines[0]);
    }
}

#[test]
fn a_call_that_fails_only_while_rendering_is_counted_with_a_swallowing_sink() {
    for round in 0..10 {
        let mut out = Rec {
            sink: Some(Rc::default()),
            swallow: true,
            ..Rec::default()
        };
        assert_eq!(code(Send::Deep, &mut out), 3, "run {round}");
    }
}

#[test]
fn a_call_that_fails_only_while_rendering_is_counted_with_a_sink_that_never_renders() {
    for round in 0..10 {
        let mut out = Hands(Some(Box::new(NoRender)));
        assert_eq!(code(Send::Deep, &mut out), 3, "run {round}");
    }
}

#[test]
fn an_unreadable_call_at_the_top_is_counted_with_a_sink_that_never_renders() {
    for round in 0..10 {
        let mut out = Hands(Some(Box::new(NoRender)));
        assert_eq!(code(Send::Top, &mut out), 3, "run {round}");
    }
}

#[test]
fn a_good_call_is_0_whatever_the_sink_does() {
    for round in 0..10 {
        assert_eq!(
            code(Send::Good, &mut Hands(Some(Box::new(NoRender)))),
            0,
            "run {round}"
        );
        let mut out = Rec {
            sink: Some(Rc::default()),
            swallow: true,
            ..Rec::default()
        };
        assert_eq!(code(Send::Good, &mut out), 0, "run {round}");
    }
}
