//! AP3: the listener server against a fake that calls it: order, back pressure, separate watchers, and a call that
//! cannot be read (it must reach the sink as an error line, and the answer must still not be an error).

mod fake;
mod support;

use std::rc::Rc;

use capnpc_cli_fixtures::features_capnp::features;
use capnpc_cli_fixtures::features_cli as cli;
use fake::{Fake, Log, Mode};
use support::{In, Rec, exit_code, run};

/// Run `watch` against a fake in `mode`: (result is ok, lines of the sink, log of the fake).
fn watch(mode: Mode) -> (bool, Vec<String>, Vec<String>) {
    let log: Log = Rc::default();
    let sink_log: Log = Rc::default();
    let mut fake = Fake::new(log.clone());
    fake.mode = mode;
    fake.probe = sink_log.clone();
    let c: features::Client = capnp_rpc::new_client(fake);
    let mut out = Rec {
        sink: Some(sink_log.clone()),
        ..Rec::default()
    };
    let r = run(cli::dispatch(&["watch"]).expect("watch")(
        &c,
        &mut In::default(),
        &mut out,
    ));
    let (ok, lines, calls) = (r.is_ok(), sink_log.borrow().clone(), log.borrow().clone());
    (ok, lines, calls)
}

#[test]
fn a_burst_arrives_whole_in_order_and_done_comes_last() {
    // M7: a line the sink never got; the answer of `done` came after the sink took every line before it
    for round in 0..10 {
        let (ok, lines, calls) = watch(Mode::Burst(50));
        assert!(ok, "run {round}");
        assert_eq!(lines.len(), 51, "run {round}");
        for (i, l) in lines[..50].iter().enumerate() {
            assert!(
                l.starts_with("event: struct : struct shape:Shape label=Text(\"e"),
                "{l}"
            );
            assert!(
                l.contains(&format!("label=Text(\"e{i}\")")),
                "event {i} out of order: {l}"
            );
        }
        assert!(
            lines[50].starts_with("done: struct : why=Text(\"end\")"),
            "{}",
            lines[50]
        );
        assert_eq!(calls.last().unwrap(), "after done: sink lines = 51");
        assert_eq!(exit_code(&lines), 0);
    }
}

#[test]
fn two_watchers_do_not_mix_their_lines() {
    let (_, a, _) = watch(Mode::Burst(3));
    let (_, b, _) = watch(Mode::Burst(5));
    assert_eq!((a.len(), b.len()), (4, 6));
}

#[test]
fn a_call_that_cannot_be_read_is_an_error_line_and_the_exit_code_is_not_0() {
    // A4 at run time: R6 (error rendered as ok), MA4 (dropped), M8 (answered with an error)
    for round in 0..10 {
        let (ok, lines, calls) = watch(Mode::Malformed);
        assert!(
            ok,
            "the answer to the caller is no error (run {round}): {calls:?}"
        );
        assert_eq!(
            calls,
            ["watch ended without an error answer"],
            "run {round}"
        );
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(lines[0].starts_with("event: ERR "), "{}", lines[0]);
        assert!(lines[1].starts_with("done: ERR "), "{}", lines[1]);
        assert_eq!(exit_code(&lines), 3);
    }
}
