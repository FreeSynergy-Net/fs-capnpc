//! T6/T7: the generated commands of the feature fixture, run against a fake server in the same process.

mod fake;
mod support;

use std::rc::Rc;

use capnp_compiler::cligen::rt::ErrorKind;
use capnpc_cli_fixtures::features_capnp::features;
use capnpc_cli_fixtures::features_cli as cli;
use fake::{Fake, Log};
use support::{In, Rec, run};

fn client() -> (features::Client, Log) {
    let log: Log = Rc::default();
    (capnp_rpc::new_client(Fake::new(log.clone())), log)
}

fn call(
    words: &[&str],
    pairs: &[(&str, &str)],
) -> Result<(Vec<String>, Log), capnp_compiler::cligen::rt::CallError> {
    let (c, log) = client();
    let run_it = cli::dispatch(words).expect("command");
    let mut inp = In::of(pairs);
    let mut out = Rec::default();
    run(run_it(&c, &mut inp, &mut out))?;
    Ok((out.lines, log))
}

#[test]
fn default_not_zero_is_kept_and_zero_is_written() {
    // M4: write only if not 0
    let (_, log) = call(&["draw"], &[("line.name", "x")]).unwrap();
    assert_eq!(
        log.borrow()[0],
        "draw width=1 opacity=0.5 name=x colour=1 from=(0,0) to=(0,0)"
    );
    let (_, log) = call(
        &["draw"],
        &[
            ("line.width", "0"),
            ("line.opacity", "0"),
            ("line.colour", "red"),
        ],
    )
    .unwrap();
    assert_eq!(
        log.borrow()[0],
        "draw width=0 opacity=0 name=line colour=0 from=(0,0) to=(0,0)"
    );
}

#[test]
fn groups_are_written_below_their_own_name() {
    let (_, log) = call(&["draw"], &[("line.from.x", "3"), ("line.to.y", "4")]).unwrap();
    assert!(
        log.borrow()[0].ends_with("from=(3,0) to=(0,4)"),
        "{:?}",
        log.borrow()
    );
}

#[test]
fn union_and_its_echo() {
    // M3: a second member is not refused
    let (lines, _) = call(&["place"], &[("shape.label", "a"), ("shape.circle", "5")]).unwrap();
    assert_eq!(
        lines,
        [
            "struct :",
            "struct shape:Shape",
            "label=Text(\"a\")",
            "circle=Uint(5)",
            "end",
            "end"
        ]
    );
    let err = call(&["place"], &[("shape.circle", "1"), ("shape.box.x", "1")])
        .map(|_| ())
        .expect_err("refused");
    assert!(
        matches!(err.kind, ErrorKind::UnionTwice) && err.path == "shape",
        "{err}"
    );
    let (lines, _) = call(&["place"], &[("shape.nothing", "")]).unwrap();
    assert!(lines.contains(&"nothing=Void".to_owned()), "{lines:?}");
}

#[test]
fn lists_of_every_kind_arrive_whole() {
    // M5: only the first element
    let p = [
        ("ints", "3"),
        ("ints.0", "1"),
        ("ints.1", "2"),
        ("ints.2", "255"),
        ("names", "2"),
        ("names.0", "a"),
        ("names.1", "b"),
        ("points", "2"),
        ("points.0.x", "1"),
        ("points.0.y", "2"),
        ("points.1.x", "3"),
        ("points.1.y", "4"),
        ("grid", "2"),
        ("grid.0", "2"),
        ("grid.0.0", "1"),
        ("grid.0.1", "2"),
        ("grid.1", "1"),
        ("grid.1.0", "3"),
        ("colours", "1"),
        ("colours.0", "deep-blue"),
    ];
    let (lines, log) = call(&["lists"], &p).unwrap();
    assert_eq!(
        log.borrow()[0],
        "lists ints=[1, 2, 255] names=[\"a\", \"b\"] points=[(1, 2), (3, 4)] grid=[[1, 2], [3]] colours=[1]"
    );
    assert_eq!(lines, ["struct :", "count=Uint(3)", "end"]);
    let err = call(&["lists"], &[("ints", "1"), ("ints.0", "256")])
        .map(|_| ())
        .expect_err("range");
    assert!(
        matches!(err.kind, ErrorKind::OutOfRange("u8")) && err.path == "ints.0",
        "{err}"
    );
}

#[test]
fn data_secret_and_enum_result() {
    let (_, log) = call(&["blob"], &[("data", "hi"), ("first", "true")]).unwrap();
    assert_eq!(log.borrow()[0], "blob data=[104, 105] first=true");
    let (_, log) = call(&["login"], &[("user", "u"), ("password", "p")]).unwrap();
    assert_eq!(log.borrow()[0], "login u:p");
    let (lines, _) = call(&["reserved-ok"], &[]).unwrap();
    assert_eq!(
        lines,
        [
            "struct :",
            "version=Uint(3)",
            "kinds=Enum(\"deep-blue\")",
            "end"
        ]
    );
}

#[test]
fn a_struct_that_contains_itself_round_trips() {
    let p = [
        ("root.name", "r"),
        ("root.children", "1"),
        ("root.children.0.name", "c"),
    ];
    let (lines, _) = call(&["tree"], &p).unwrap();
    assert_eq!(
        lines,
        [
            "struct :",
            "struct root:Node",
            "name=Text(\"r\")",
            "list children:Node",
            "struct :Node",
            "name=Text(\"c\")",
            "list children:Node",
            "end",
            "end",
            "end",
            "end",
            "end"
        ]
    );
}

#[test]
fn dispatch_knows_exactly_the_commands_of_the_table() {
    for c in cli::COMMANDS {
        assert!(cli::dispatch(c.words).is_some(), "{:?}", c.words);
    }
    assert!(cli::dispatch(&["nope"]).is_none());
}

#[test]
fn every_row_names_its_interface_and_ordinal() {
    // M2: ordinal + 1
    let row = |w: &[&str]| cli::COMMANDS.iter().find(|c| c.words == w).expect("row");
    let draw = row(&["draw"]);
    assert_eq!(
        (draw.iface_id, draw.ordinal),
        (capnpc_cli_fixtures::ids::FEATURES, 1)
    );
    let age = row(&["create-at"]);
    assert_eq!(
        (age.iface_id, age.ordinal),
        (capnpc_cli_fixtures::ids::FEATURES, 12)
    );
    let rename = row(&["details", "rename"]);
    assert_eq!(
        (rename.iface_id, rename.ordinal),
        (capnpc_cli_fixtures::ids::DETAILS, 0)
    );
}

#[test]
fn listener_gets_its_lines_in_order_and_the_call_returns_the_subscription() {
    // M7/M8: the answer goes out after the sink took the line, and it is never an error
    let (c, _) = client();
    let sink_log: Log = Rc::default();
    let mut out = Rec {
        sink: Some(sink_log.clone()),
        ..Rec::default()
    };
    let mut inp = In::default();
    let held = run(cli::dispatch(&["watch"]).expect("watch")(
        &c, &mut inp, &mut out,
    ))
    .unwrap();
    assert!(held.is_watching());
    let lines = sink_log.borrow();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_eq!(
        lines[0],
        "event: struct : struct shape:Shape label=Text(\"l\") circle=Uint(0) end end ok"
    );
    assert!(
        lines[1].starts_with("done: struct : why=Text(\"bye\")"),
        "{lines:?}"
    );
    // without a sink the command refuses instead of dropping the events
    let mut none = Rec::default();
    let err = run(cli::dispatch(&["watch"]).unwrap()(
        &c,
        &mut In::default(),
        &mut none,
    ))
    .map(|_| ())
    .expect_err("no sink");
    assert!(matches!(err.kind, ErrorKind::Unsupported(_)), "{err}");
}

#[test]
fn float32_value_that_is_not_zero_arrives() {
    // a Float32 written as 0 would only show in the golden file
    let (_, log) = call(&["draw"], &[("line.opacity", "0.25")]).unwrap();
    assert!(
        log.borrow()[0].contains("opacity=0.25"),
        "{:?}",
        log.borrow()
    );
}

#[test]
fn several_roots_each_have_their_own_dispatch() {
    use capnpc_cli_fixtures::multi_cli as m;
    assert!(m::alpha::dispatch(&["first"]).is_some() && m::alpha::dispatch(&["second"]).is_none());
    assert!(m::beta::dispatch(&["second"]).is_some());
    assert!(m::gamma::dispatch(&["third"]).is_some());
    assert_eq!(m::beta::COMMANDS[0].params.len(), 1);
}
