//! The fake server of the feature fixture: it logs what it received and echoes what the tests ask for.
#![allow(dead_code)] // shared by several test files, each uses a part

use std::cell::RefCell;
use std::future::{Future, ready};
use std::rc::Rc;

use capnpc_cli_fixtures::features_capnp::{Colour, features};

pub type Log = Rc<RefCell<Vec<String>>>;

/// What the fake does when it is asked to `watch`.
#[derive(Clone, Copy)]
pub enum Mode {
    /// One event and one `done`.
    Normal,
    /// This many events, then `done`; the fake notes how many lines the sink had when `done` was answered.
    Burst(u32),
    /// An event and a `done` whose arguments are not what the method takes (a text where a struct belongs).
    Malformed,
}

pub struct Fake {
    pub log: Log,
    pub mode: Mode,
    /// The lines of the sink, to check that the answer came after the sink took the line.
    pub probe: Log,
}

impl Fake {
    pub fn new(log: Log) -> Self {
        Self {
            log,
            mode: Mode::Normal,
            probe: Rc::default(),
        }
    }
}

fn done() -> impl Future<Output = Result<(), capnp::Error>> + 'static {
    ready(Ok(()))
}

impl features::Server for Fake {
    fn draw(
        self: capnp::capability::Rc<Self>,
        params: features::DrawParams,
        _: features::DrawResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        let r = (|| {
            let l = params.get()?.get_line()?;
            self.log.borrow_mut().push(format!(
                "draw width={} opacity={} name={} colour={:?} from=({},{}) to=({},{})",
                l.get_width(),
                l.get_opacity(),
                l.get_name()?.to_str()?,
                l.get_colour().map_err(capnp::Error::from)? as u16,
                l.reborrow().get_from().get_x(),
                l.reborrow().get_from().get_y(),
                l.reborrow().get_to().get_x(),
                l.reborrow().get_to().get_y(),
            ));
            Ok(())
        })();
        ready(r)
    }

    fn place(
        self: capnp::capability::Rc<Self>,
        params: features::PlaceParams,
        mut results: features::PlaceResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        let r = (|| {
            results.get().set_shape(params.get()?.get_shape()?)?;
            Ok(())
        })();
        ready(r)
    }

    fn lists(
        self: capnp::capability::Rc<Self>,
        params: features::ListsParams,
        mut results: features::ListsResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        let r = (|| {
            let p = params.get()?;
            let ints: Vec<u8> = p.get_ints()?.iter().collect();
            let names: Vec<String> = p
                .get_names()?
                .iter()
                .map(|n| Ok(n?.to_str()?.to_owned()))
                .collect::<capnp::Result<_>>()?;
            let points: Vec<(i32, i32)> = p
                .get_points()?
                .iter()
                .map(|q| (q.get_x(), q.get_y()))
                .collect();
            let mut grid = Vec::new();
            for row in p.get_grid()? {
                grid.push(row?.iter().collect::<Vec<i32>>());
            }
            let colours: Vec<u16> = p
                .get_colours()?
                .iter()
                .map(|c| c.map(|c| c as u16))
                .collect::<Result<_, _>>()?;
            self.log
                .borrow_mut()
                .push(format!("lists ints={ints:?} names={names:?} points={points:?} grid={grid:?} colours={colours:?}"));
            results
                .get()
                .set_count(u32::try_from(ints.len()).unwrap_or(0));
            Ok(())
        })();
        ready(r)
    }

    fn blob(
        self: capnp::capability::Rc<Self>,
        params: features::BlobParams,
        mut results: features::BlobResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        let r = (|| {
            let p = params.get()?;
            self.log.borrow_mut().push(format!(
                "blob data={:?} first={}",
                p.get_data()?,
                p.get_first()
            ));
            results.get().set_data(p.get_data()?);
            Ok(())
        })();
        ready(r)
    }

    fn tree(
        self: capnp::capability::Rc<Self>,
        params: features::TreeParams,
        mut results: features::TreeResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        let r = (|| {
            results.get().set_root(params.get()?.get_root()?)?;
            Ok(())
        })();
        ready(r)
    }

    fn login(
        self: capnp::capability::Rc<Self>,
        params: features::LoginParams,
        _: features::LoginResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        let r = (|| {
            let p = params.get()?;
            self.log.borrow_mut().push(format!(
                "login {}:{}",
                p.get_user()?.to_str()?,
                p.get_password()?.to_str()?
            ));
            Ok(())
        })();
        ready(r)
    }

    fn watch(
        self: capnp::capability::Rc<Self>,
        params: features::WatchParams,
        _: features::WatchResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        let sink = params
            .get()
            .and_then(features::watch_params::Reader::get_sink);
        async move {
            let sink = sink?;
            match self.mode {
                Mode::Normal | Mode::Burst(_) => {
                    let n = if let Mode::Burst(n) = self.mode { n } else { 1 };
                    for i in 0..n {
                        let mut ev = sink.event_request();
                        let label = if n == 1 {
                            "l".to_owned()
                        } else {
                            format!("e{i}")
                        };
                        ev.get().init_shape().set_label(label.as_str());
                        ev.send().await?;
                    }
                    let mut d = sink.done_request();
                    d.get().set_why(if n == 1 { "bye" } else { "end" });
                    d.send().promise.await?;
                    self.log.borrow_mut().push(format!(
                        "after done: sink lines = {}",
                        self.probe.borrow().len()
                    ));
                }
                Mode::Malformed => {
                    let mut ev = sink.client.new_streaming_call::<capnp::any_pointer::Owned>(
                        capnpc_cli_fixtures::ids::SINK,
                        0,
                        None,
                    );
                    ev.get().set_as::<capnp::text::Owned>("not a struct")?;
                    ev.send().await?;
                    let mut d = sink
                        .client
                        .new_call::<capnp::any_pointer::Owned, capnp::any_pointer::Owned>(
                            capnpc_cli_fixtures::ids::SINK,
                            1,
                            None,
                        );
                    d.get().set_as::<capnp::text::Owned>("not a struct")?;
                    d.send().promise.await?;
                    self.log
                        .borrow_mut()
                        .push("watch ended without an error answer".to_owned());
                }
            }
            Ok(())
        }
    }

    fn reserved_ok(
        self: capnp::capability::Rc<Self>,
        _: features::ReservedOkParams,
        mut results: features::ReservedOkResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        results.get().set_version(3);
        results.get().set_kinds(Colour::DeepBlue);
        done()
    }
}
