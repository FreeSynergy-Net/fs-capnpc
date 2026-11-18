//! A9: the program's `wait` ends with an error that is not a lost connection (the rpc system failed, a call was
//! refused, the protocol was broken). That is a plain error of the other side, exit code 1: only the connection-lost
//! kinds make 3, and no error of `wait` is swallowed. A watchdog turns a hang into a red test.

mod fake;
mod support;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use capnp_compiler::cligen::rt::CallError;
use capnpc_cli_fixtures::features_capnp::features;
use capnpc_cli_fixtures::features_cli as cli;
use fake::{Fake, Log, Mode};
use support::{In, Rec, run};

/// Ends the whole process with a message if the test does not finish in time (a wait error must end the program).
struct Watchdog(Arc<AtomicBool>);

impl Watchdog {
    fn start(name: &'static str) -> Self {
        let done = Arc::new(AtomicBool::new(false));
        let d = done.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(60));
            if !d.load(Ordering::SeqCst) {
                eprintln!("TIMEOUT: `{name}` hangs");
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

/// The exit code of a `watch` whose `wait` ends with `error` (or with Ok when `None`).
fn code_of_wait(name: &'static str, error: Option<capnp::Error>) -> i32 {
    let _dog = Watchdog::start(name);
    let mut fake = Fake::new(Log::default());
    fake.mode = Mode::Burst(2);
    let c: features::Client = capnp_rpc::new_client(fake);
    let mut out = Rec {
        sink: Some(std::rc::Rc::default()),
        ..Rec::default()
    };
    run(cli::run_to_exit(
        &c,
        &["watch"],
        &mut In::default(),
        &mut out,
        move || async move { error.map_or(Ok(()), |e| Err(CallError::from(e))) },
    ))
}

#[test]
fn a_wait_that_ends_with_another_error_than_a_lost_connection_is_exit_1() {
    for round in 0..10 {
        let failed = capnp::Error::failed("the rpc system broke".to_owned());
        assert_eq!(
            code_of_wait("failed", Some(failed)),
            1,
            "failed, run {round}"
        );
        let unimplemented = capnp::Error::unimplemented("not there".to_owned());
        assert_eq!(
            code_of_wait("unimplemented", Some(unimplemented)),
            1,
            "unimplemented, run {round}"
        );
        let overloaded = capnp::Error::overloaded("too much".to_owned());
        assert_eq!(
            code_of_wait("overloaded", Some(overloaded)),
            1,
            "overloaded, run {round}"
        );
    }
}

#[test]
fn a_wait_that_ends_with_a_lost_connection_is_exit_3_and_a_clean_wait_is_exit_0() {
    for round in 0..10 {
        let lost = capnp::Error::disconnected("gone".to_owned());
        assert_eq!(code_of_wait("lost", Some(lost)), 3, "lost, run {round}");
        assert_eq!(code_of_wait("clean", None), 0, "clean, run {round}");
    }
}
