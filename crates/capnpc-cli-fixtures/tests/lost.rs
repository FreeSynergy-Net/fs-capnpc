//! A7 (B3): the connection is cut in the middle of a stream. The program's `wait` ends with the error of the lost
//! connection, and the generated `run_to_exit` makes exit code 3 of it. The connection is real (two-party rpc over an
//! in-memory cable the test can cut); a watchdog turns a hang into a red test.

mod fake;
mod support;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use capnp_rpc::rpc_twoparty_capnp::Side;
use capnp_rpc::{RpcSystem, twoparty};
use capnpc_cli::rt::CallError;
use capnpc_cli_fixtures::features_capnp::features;
use capnpc_cli_fixtures::features_cli as cli;
use fake::{Fake, Log, Mode};
use futures::executor::LocalPool;
use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use futures::task::LocalSpawnExt;
use support::{In, Rec};

#[derive(Default)]
struct Pipe {
    buf: VecDeque<u8>,
    closed: bool,
    waker: Option<Waker>,
}

type Shared = Rc<RefCell<Pipe>>;

fn close(p: &Shared) {
    let mut b = p.borrow_mut();
    b.closed = true;
    if let Some(w) = b.waker.take() {
        w.wake();
    }
}

/// One end of the cable.
struct End {
    rx: Shared,
    tx: Shared,
}

impl Drop for End {
    fn drop(&mut self) {
        close(&self.tx);
    }
}

impl AsyncRead for End {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        out: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let mut p = self.rx.borrow_mut();
        if !p.buf.is_empty() {
            let n = out.len().min(p.buf.len());
            for (slot, byte) in out.iter_mut().zip(p.buf.drain(..n)) {
                *slot = byte;
            }
            Poll::Ready(Ok(n))
        } else if p.closed {
            Poll::Ready(Ok(0))
        } else {
            p.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl AsyncWrite for End {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        data: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut p = self.tx.borrow_mut();
        if p.closed {
            return Poll::Ready(Err(io::ErrorKind::BrokenPipe.into()));
        }
        p.buf.extend(data);
        if let Some(w) = p.waker.take() {
            w.wake();
        }
        Poll::Ready(Ok(data.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        close(&self.tx);
        Poll::Ready(Ok(()))
    }
}

/// Turns a hang into a red test: the whole process ends with a message if the test does not finish in time.
struct Watchdog(Arc<AtomicBool>);

impl Watchdog {
    fn start(name: &'static str) -> Self {
        let done = Arc::new(AtomicBool::new(false));
        let d = done.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(60));
            if !d.load(Ordering::SeqCst) {
                eprintln!(
                    "TIMEOUT: `{name}` hangs (a lost connection must end the program, not freeze it)"
                );
                std::process::exit(101);
            }
        });
        Self(done)
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

/// Give the other tasks of the pool a turn.
async fn yield_once() {
    let mut first = true;
    futures::future::poll_fn(|cx| {
        if first {
            first = false;
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    })
    .await;
}

/// What the "program" does while the watch runs: one event arrives, then (maybe) the cable is cut, then it waits for
/// its connection to end, as a real program waits for the rpc system.
fn watch_over_a_cable(cut: bool) -> (i32, Vec<String>) {
    let name = if cut { "cable cut" } else { "clean end" };
    let _dog = Watchdog::start(name);
    let a: Shared = Rc::default();
    let b: Shared = Rc::default();
    let (cable_a, cable_b) = (a.clone(), b.clone());
    let client_end = End {
        rx: b.clone(),
        tx: a.clone(),
    };
    let server_end = End { rx: a, tx: b };

    let mut fake = Fake::new(Log::default());
    fake.mode = Mode::Stash;
    let stash = fake.stash.clone();
    let mut pool = LocalPool::new();
    let spawn = pool.spawner();

    let (sr, sw) = server_end.split();
    let net = twoparty::VatNetwork::new(
        sr,
        sw,
        Side::Server,
        capnp::message::ReaderOptions::default(),
    );
    let boot = capnp_rpc::new_client::<features::Client, _>(fake).client;
    let server = RpcSystem::new(Box::new(net), Some(boot));
    spawn
        .spawn_local(async move {
            let _ = server.await;
        })
        .unwrap();

    let (cr, cw) = client_end.split();
    let net = twoparty::VatNetwork::new(
        cr,
        cw,
        Side::Client,
        capnp::message::ReaderOptions::default(),
    );
    let mut rpc = RpcSystem::new(Box::new(net), None);
    let client: features::Client = rpc.bootstrap(Side::Server);
    let (ended, ended_rx) = futures::channel::oneshot::channel();
    spawn
        .spawn_local(async move {
            let _ = ended.send(rpc.await);
        })
        .unwrap();

    let lines = Rc::new(RefCell::new(Vec::new()));
    let mut out = Rec {
        sink: Some(lines.clone()),
        ..Rec::default()
    };
    let seen = lines.clone();
    let wait = move || async move {
        let sink = stash.borrow_mut().take().expect("the fake kept the sink");
        let mut ev = sink.event_request();
        ev.get().init_shape().set_label("before");
        ev.send().await.unwrap();
        // the stream is in flight: go on only when the program's sink has the line (the watchdog guards the loop)
        while seen.borrow().is_empty() {
            yield_once().await;
        }
        if !cut {
            return Ok(());
        }
        close(&cable_a);
        close(&cable_b);
        // the program waits for its connection; when that is over, the stream is broken
        let _ = ended_rx.await;
        Err(CallError::from(capnp::Error::disconnected(
            "connection lost".to_owned(),
        )))
    };
    let code = pool.run_until(cli::run_to_exit(
        &client,
        &["watch"],
        &mut In::default(),
        &mut out,
        wait,
    ));
    let lines = lines.borrow().clone();
    (code, lines)
}

#[test]
fn a_connection_lost_in_the_middle_of_a_stream_is_exit_3() {
    for round in 0..10 {
        let (code, lines) = watch_over_a_cable(true);
        assert_eq!(code, 3, "run {round}");
        assert_eq!(
            lines.len(),
            1,
            "the event before the cut arrived: {lines:?}"
        );
    }
}

#[test]
fn a_stream_that_ends_cleanly_over_the_same_connection_is_exit_0() {
    for round in 0..10 {
        let (code, lines) = watch_over_a_cable(false);
        assert_eq!(code, 0, "run {round}");
        assert_eq!(lines.len(), 1, "{lines:?}");
    }
}
