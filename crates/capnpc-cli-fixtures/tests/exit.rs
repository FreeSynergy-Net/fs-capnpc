//! A7: the exit code comes from the generated entry point and from what the generated listener counted, not from
//! the sink and not from the program: a sink that swallows errors cannot make a broken stream look fine.

mod fake;
mod support;

use std::cell::Cell;
use std::rc::Rc;

use capnpc_cli_fixtures::features_capnp::features;
use capnpc_cli_fixtures::features_cli as cli;
use capnpc_cli_fixtures::ids;
use fake::{Fake, Log, Mode};
use support::{In, Rec, run};

/// The exit code of `words` against a fake in `mode`; `swallow` hands the command a sink that drops every error.
fn code(mode: Mode, words: &[&str], pairs: &[(&str, &str)], swallow: bool) -> i32 {
    let mut fake = Fake::new(Log::default());
    fake.mode = mode;
    let c: features::Client = capnp_rpc::new_client(fake);
    let mut out = Rec {
        sink: Some(Rc::default()),
        swallow,
        ..Rec::default()
    };
    run(cli::run_to_exit(
        &c,
        words,
        &mut In::of(pairs),
        &mut out,
        || async {},
    ))
}

#[test]
fn a_sink_that_swallows_errors_cannot_hide_unreadable_calls() {
    // X1: the error arm of the listener does not count; X2: a failing render is not counted
    for round in 0..10 {
        assert_eq!(
            code(Mode::Malformed, &["watch"], &[], true),
            3,
            "swallowing sink, run {round}"
        );
        assert_eq!(
            code(Mode::Malformed, &["watch"], &[], false),
            3,
            "recording sink, run {round}"
        );
    }
}

#[test]
fn a_clean_watch_and_a_plain_call_exit_with_0() {
    assert_eq!(code(Mode::Burst(5), &["watch"], &[], true), 0);
    assert_eq!(code(Mode::Normal, &["reserved-ok"], &[], false), 0);
}

#[test]
fn the_counters_of_all_listeners_add_up_and_a_lost_connection_is_3() {
    // X4: only the first listener counted; X9: a lost connection is a plain error (1)
    assert_eq!(code(Mode::Normal, &["watch-two"], &[], true), 3);
    assert_eq!(code(Mode::Normal, &["raw-value"], &[], false), 3);
}

#[test]
fn the_other_sides_error_wrong_words_and_wrong_values_have_their_own_codes() {
    // `danger` is not implemented by the fake: the contract side answers with an error
    assert_eq!(code(Mode::Normal, &["danger"], &[], false), 1);
    assert_eq!(code(Mode::Normal, &["nope"], &[], false), 2);
    assert_eq!(
        code(
            Mode::Normal,
            &["lists"],
            &[("ints", "1"), ("ints.0", "256")],
            false
        ),
        2
    );
}

#[test]
fn the_code_is_made_after_the_wait_from_what_arrived_while_waiting() {
    // X6: the code made before the wait; X7: the wait not run. The unreadable call comes only while the program waits.
    let mut fake = Fake::new(Log::default());
    fake.mode = Mode::Stash;
    let stash = fake.stash.clone();
    let waited = Rc::new(Cell::new(false));
    let w = waited.clone();
    let c: features::Client = capnp_rpc::new_client(fake);
    let wait = move || async move {
        w.set(true);
        let sink = stash.borrow_mut().take().expect("the fake kept the sink");
        let mut ev =
            sink.client
                .new_streaming_call::<capnp::any_pointer::Owned>(ids::SINK, 0, None);
        ev.get()
            .set_as::<capnp::text::Owned>("not a struct")
            .unwrap();
        ev.send().await.unwrap();
        let mut d = sink.done_request();
        d.get().set_why("end");
        d.send().promise.await.unwrap();
    };
    let mut out = Rec {
        sink: Some(Rc::default()),
        swallow: true,
        ..Rec::default()
    };
    let code = run(cli::run_to_exit(
        &c,
        &["watch"],
        &mut In::default(),
        &mut out,
        wait,
    ));
    assert!(waited.get(), "the program waited");
    assert_eq!(code, 3);
    // a command that does not watch is not waited for
    let waited = Rc::new(Cell::new(false));
    let w = waited.clone();
    let c: features::Client = capnp_rpc::new_client(Fake::new(Log::default()));
    let c0 = run(cli::run_to_exit(
        &c,
        &["reserved-ok"],
        &mut In::default(),
        &mut Rec::default(),
        move || async move {
            w.set(true);
        },
    ));
    assert_eq!((c0, waited.get()), (0, false));
}
