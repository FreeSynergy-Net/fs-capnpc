//! Test input, output and sink: plain recorders of what the generated code does.
#![allow(dead_code)] // shared by several test files, each uses a part

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use capnp_compiler::cligen::rt::{CallError, ErrorKind, Input, Output, Sink, Value};

/// Input from a map: path -> text. Lists: the path holds the count, the elements are `path.0`, `path.1`.
#[derive(Default)]
pub struct In(pub BTreeMap<String, String>);

impl In {
    #[must_use]
    pub fn of(pairs: &[(&str, &str)]) -> Self {
        Self(
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
        )
    }
}

fn bad(path: &str, what: &str) -> CallError {
    CallError::at(path, ErrorKind::BadValue(what.to_owned()))
}

impl Input for In {
    fn has(&self, path: &str) -> bool {
        let below = format!("{path}.");
        self.0.keys().any(|k| k == path || k.starts_with(&below))
    }
    fn text(&mut self, path: &str) -> Result<Option<String>, CallError> {
        Ok(self.0.get(path).cloned())
    }
    fn int(&mut self, path: &str) -> Result<Option<i128>, CallError> {
        self.0
            .get(path)
            .map(|v| v.parse().map_err(|_| bad(path, "int")))
            .transpose()
    }
    fn float(&mut self, path: &str) -> Result<Option<f64>, CallError> {
        self.0
            .get(path)
            .map(|v| v.parse().map_err(|_| bad(path, "float")))
            .transpose()
    }
    fn boolean(&mut self, path: &str) -> Result<Option<bool>, CallError> {
        self.0
            .get(path)
            .map(|v| v.parse().map_err(|_| bad(path, "bool")))
            .transpose()
    }
    fn enumerant(&mut self, path: &str, names: &[&str]) -> Result<Option<usize>, CallError> {
        self.0
            .get(path)
            .map(|v| {
                names
                    .iter()
                    .position(|n| n == v)
                    .ok_or_else(|| bad(path, "enumerant"))
            })
            .transpose()
    }
    fn count(&mut self, path: &str) -> Result<Option<u32>, CallError> {
        self.0
            .get(path)
            .map(|v| v.parse().map_err(|_| bad(path, "count")))
            .transpose()
    }
    fn data(&mut self, path: &str) -> Result<Option<Vec<u8>>, CallError> {
        Ok(self.0.get(path).map(|v| v.clone().into_bytes()))
    }
}

/// Output that writes one line per visit.
#[derive(Default)]
pub struct Rec {
    pub lines: Vec<String>,
    pub sink: Option<Rc<RefCell<Vec<String>>>>,
    /// Hand out a [`Swallow`] sink instead.
    pub swallow: bool,
}

impl Output for Rec {
    fn begin_struct(&mut self, name: &str, ty_name: &str) {
        self.lines.push(format!("struct {name}:{ty_name}"));
    }
    fn begin_list(&mut self, name: &str, ty_name: &str) {
        self.lines.push(format!("list {name}:{ty_name}"));
    }
    fn field(&mut self, name: &str, value: Value<'_>) {
        self.lines.push(format!("{name}={value:?}"));
    }
    fn end(&mut self) {
        self.lines.push("end".to_owned());
    }
    fn sink(&mut self) -> Option<Box<dyn Sink>> {
        if self.swallow {
            return Some(Box::new(Swallow));
        }
        self.sink
            .clone()
            .map(|s| Box::new(Lines(s)) as Box<dyn Sink>)
    }
}

/// A sink that records the lines it was given.
pub struct Lines(pub Rc<RefCell<Vec<String>>>);

impl Sink for Lines {
    fn line(
        &mut self,
        kind: &str,
        render: &mut dyn FnMut(&mut dyn Output) -> Result<(), CallError>,
    ) {
        let mut r = Rec::default();
        let line = match render(&mut r) {
            Ok(()) => format!("{kind}: {} ok", r.lines.join(" ")),
            Err(e) => format!("{kind}: ERR {e}"),
        };
        self.0.borrow_mut().push(line);
    }
}

/// Run a future on a local pool (the capnp-rpc servers of the tests are local).
pub fn run<F: std::future::Future>(f: F) -> F::Output {
    futures::executor::LocalPool::new().run_until(f)
}

/// A sink that swallows everything, also the errors: the generated code must still count them.
pub struct Swallow;

impl Sink for Swallow {
    fn line(&mut self, _: &str, render: &mut dyn FnMut(&mut dyn Output) -> Result<(), CallError>) {
        let _ = render(&mut Rec::default());
    }
}
