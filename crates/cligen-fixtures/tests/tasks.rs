//! T6: every generated command of the real tasks contract calls exactly its own method (interface and ordinal) of a
//! fake server, and the values arrive.

mod fake_tasks;
mod support;

use std::rc::Rc;

use capnpc_cli_fixtures::tasks_capnp::tasks;
use capnpc_cli_fixtures::tasks_cli as cli;
use fake_tasks::{FakeTasks, Log};
use support::{In, Rec, run};

fn call(words: &[&str], pairs: &[(&str, &str)]) -> Vec<String> {
    let log: Log = Rc::default();
    let root: tasks::Client = capnp_rpc::new_client(FakeTasks(log.clone()));
    let mut out = Rec {
        sink: Some(Rc::default()),
        ..Rec::default()
    };
    let go = cli::dispatch(words).unwrap_or_else(|| panic!("no command {words:?}"));
    run(go(&root, &mut In::of(pairs), &mut out)).unwrap_or_else(|e| panic!("{words:?}: {e}"));
    std::mem::take(&mut *log.borrow_mut())
}

#[test]
fn t6_every_command_calls_exactly_its_own_method() {
    // M2 / O1: a wrong ordinal reaches another method of the fake, and the log shows it
    let mut seen = Vec::new();
    for c in cli::COMMANDS {
        let calls: Vec<String> = call(c.words, &[])
            .into_iter()
            .filter(|l| !l.starts_with("detail:"))
            .collect();
        assert_eq!(calls, [c.key], "{:?}", c.words);
        seen.push(c.key);
    }
    assert!(seen.len() >= 21, "{}", seen.len());
    let mut sorted = seen.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        seen.len(),
        "no two commands reach the same method"
    );
}

#[test]
fn t6_the_values_arrive() {
    let calls = call(
        &["items", "edit-if"],
        &[
            ("id.hi", "1"),
            ("id.lo", "2"),
            ("revision", "7"),
            ("jscalendar", "x"),
        ],
    );
    assert_eq!(
        calls,
        [
            "tasks/Items.editIf",
            "detail: id=1:2 revision=7 jscalendar=x"
        ]
    );
}

#[test]
fn t6_the_table_rows_reach_their_methods_by_interface_and_ordinal() {
    // O1 / M2: a row with a wrong ordinal or interface id: the call by (id, ordinal) lands on another method
    let mut n = 0;
    for c in cli::COMMANDS {
        let log: Log = Rc::default();
        let root: tasks::Client = capnp_rpc::new_client(FakeTasks(log.clone()));
        let client = if c.key.starts_with("tasks/Tasks.") {
            root.client.clone()
        } else if c.key.starts_with("tasks/Items.") {
            root.items_request().send().pipeline.get_items().client
        } else {
            root.types_request().send().pipeline.get_catalog().client
        };
        let req = client.new_call::<capnp::any_pointer::Owned, capnp::any_pointer::Owned>(
            c.iface_id, c.ordinal, None,
        );
        run(req.send().promise).unwrap_or_else(|e| panic!("{}: {e}", c.key));
        let calls: Vec<String> = log
            .borrow()
            .iter()
            .filter(|l| !l.starts_with("detail:"))
            .cloned()
            .collect();
        assert_eq!(
            calls,
            [c.key],
            "row {} ({:#x}, {})",
            c.key,
            c.iface_id,
            c.ordinal
        );
        n += 1;
    }
    assert_eq!(n, cli::COMMANDS.len());
}
