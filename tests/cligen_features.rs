//! T7: one fixture case per gap of the probe, as text of the generated file (the run-time round trip is in
//! `capnpc-cli-fixtures`). Each assertion names the mutant it kills.

#[path = "cligen_common/mod.rs"]
mod common;

fn text() -> String {
    common::generated(&["features.capnp", "capnpc_cli.capnp"], "features_cli.rs")
}

fn row(t: &str, path: &str) -> String {
    let needle = format!("Param {{ path: {path:?},");
    t.lines()
        .find(|l| l.contains(&needle))
        .unwrap_or_else(|| panic!("no row for {path}"))
        .to_owned()
}

#[test]
fn secret_is_marked_and_only_that_one() {
    // M9: `secret` handled as a normal parameter
    let t = text();
    assert!(row(&t, "password").contains("secret: true"));
    assert!(row(&t, "user").contains("secret: false"));
}

#[test]
fn union_members_know_their_union() {
    // M3: two members of a union set silently
    let t = text();
    assert!(row(&t, "shape.circle").contains("union_of: Some(\"shape\")"));
    assert!(row(&t, "shape.nothing").contains("ParamTy::Void"));
    assert!(row(&t, "shape.label").contains("union_of: None"));
    assert!(t.contains("CallError::union_twice(path)"));
}

#[test]
fn groups_lists_and_defaults_are_rows_and_typed_setters() {
    // M6: group written flat; M5: list written with one element only; M4: write only if not 0
    let t = text();
    assert!(row(&t, "line.from").contains("ParamTy::Struct"));
    assert!(row(&t, "line.from.x").contains("ParamTy::Int { signed: true, bits: 32 }"));
    assert!(row(&t, "line.width").contains("ParamTy::Int { signed: false, bits: 16 }"));
    assert!(t.contains("b.reborrow().init_from()"));
    assert!(t.contains("for i0 in 0..n {"));
    assert!(t.contains("l.set(i0, u8::try_from(v)"));
    assert!(t.contains("b.set_width(u16::try_from(v)"));
    assert!(
        !t.contains("!= 0"),
        "a value of 0 is written like any other"
    );
    assert!(row(&t, "grid").contains("ParamTy::List"));
    assert!(t.contains("l1.set(i1, i32::try_from(v)"));
}

#[test]
fn any_pointer_is_listed_as_unreadable() {
    // M12: AnyPointer dropped silently
    let t = text();
    let r = row(&t, "value");
    assert!(r.contains("ParamTy::Unreadable") && r.contains("ty_name: \"any-pointer\""));
}

#[test]
fn recursion_ends_and_is_listed() {
    let t = text();
    assert!(row(&t, "root.children").contains("ParamTy::List"));
    assert!(t.contains("fn set_features_capnp_node("));
}

#[test]
fn listener_server_answers_after_the_sink_and_never_with_an_error() {
    // M7: answer before the sink took the line; M8: answer with an error
    let t = text();
    let line = t
        .find("self.sink.borrow_mut().line(\"event\"")
        .expect("event line");
    let ready = t[line..]
        .find("::core::future::ready(Ok(()))")
        .expect("answer");
    assert!(ready > 0);
    assert!(!t.contains("ready(Err("));
    assert!(t.contains("pub struct SinkLines<S: Sink>"));
    assert!(t.contains("watch: true"));
}

#[test]
fn risk_annotation_is_passed_on() {
    let t = text();
    assert!(t.contains("annotations: &[(0xf0a9fc9d3d3d3fb5, \"destructive\")],"));
}

#[test]
fn reserved_word_keeps_the_contract_name_and_gets_a_prefix() {
    // A2: the generator must not break on `version`, and must not hide that the word changed
    let t = common::generated(&["reserved.capnp"], "reserved_cli.rs");
    assert!(t.contains("words: &[\"method-version\"],"), "{t}");
    assert!(t.contains("key: \"reserved/Bad.version\","));
    assert!(t.contains("renamed: true,"));
    assert!(t.contains("[\"method-version\"] => Some("));
    // the real case: `Control.version` of common.capnp, together with the second root `Blob`
    let c = common::generated(&["common.capnp"], "common_cli.rs");
    assert!(c.contains("key: \"common/Control.version\","));
    assert!(c.contains("pub mod control {") && c.contains("pub mod blob {"));
    assert!(c.contains("renamed: true,") && c.contains("renamed: false,"));
}

#[test]
fn several_roots_give_one_module_each_in_a_fixed_order() {
    let t = common::generated(&["multi.capnp"], "multi_cli.rs");
    for m in ["alpha", "beta", "gamma"] {
        assert!(t.contains(&format!("pub mod {m} {{")), "no module {m}");
    }
}

#[test]
fn a_call_that_cannot_be_read_becomes_an_error_line_not_a_gap() {
    // A4: the listener dropped such a call silently
    let t = text();
    assert!(
        t.contains("Err(e) => { self.errors.set(self.errors.get() + 1); let why = e.to_string(); self.sink.borrow_mut().line(\"event\"")
    );
    assert!(
        t.contains("Err(e) => { self.errors.set(self.errors.get() + 1); let why = e.to_string(); self.sink.borrow_mut().line(\"done\"")
    );
    assert!(!t.contains("if let Ok(p) = params.get()"));
}

#[test]
fn helper_names_are_snake_case() {
    // the gate found `enum_features_capnp_Colour`: a non-snake-case function is a lint error in the user's crate
    let t = text();
    assert!(t.contains("fn enum_features_capnp_colour("));
    for l in t.lines().filter(|l| l.starts_with("fn ")) {
        let name = l[3..].split('(').next().unwrap();
        assert!(!name.chars().any(char::is_uppercase), "{name}");
    }
}

/// The text of the `Command` row (or `Group` line) that starts with these words.
fn row_of<'a>(t: &'a str, words: &str) -> &'a str {
    let start = t
        .find(&format!("words: &[{words}],"))
        .unwrap_or_else(|| panic!("no words {words}"));
    let end = t[start..].find("},").map_or(t.len(), |e| start + e);
    &t[start..end]
}

#[test]
fn every_reserved_word_gets_the_prefix_on_the_top_level_only() {
    // R1: only `version` escaped; R4: escaped below the top level too; R2: `renamed` not handed down;
    // R7 (empty prefix) and MA2 (no escape) are killed here as well
    let t = common::generated(&["words.capnp"], "words_cli.rs");
    for w in ["help", "mcp"] {
        let r = row_of(&t, &format!("\"method-{w}\""));
        assert!(r.contains("renamed: true,"), "{w}: {r}");
        assert!(
            r.contains(&format!("key: \"words/Words.{w}\"")),
            "the contract name stays: {r}"
        );
    }
    // `serve` is a navigation word here: the group is renamed, the command below it is marked
    assert!(t.contains("Group { words: &[\"method-serve\"], key: \"words/Words.serve\" }"));
    let below = row_of(&t, "\"method-serve\", \"version\"");
    assert!(
        below.contains("renamed: true,") && below.contains("key: \"words/Sub.version\""),
        "{below}"
    );
    // V7: the mark goes down on every level, not on the first one only
    let deep = row_of(&t, "\"method-serve\", \"down\", \"ping\"");
    assert!(
        deep.contains("renamed: true,") && deep.contains("key: \"words/Leaf.ping\""),
        "{deep}"
    );
    assert!(row_of(&t, "\"nav\", \"down\", \"ping\"").contains("renamed: false,"));
    // the same interface below `nav`: no reserved word on the top level, nothing is renamed
    let plain = row_of(&t, "\"nav\", \"version\"");
    assert!(plain.contains("renamed: false,"), "{plain}");
    assert!(
        !t.contains("\"nav\", \"method-version\""),
        "no prefix below the top level"
    );
    assert!(!t.contains("\"method-nav\""));
}

#[test]
fn two_methods_with_the_same_words_are_an_error_not_an_overwrite() {
    // R5: the check for equal words switched off
    let err = capnp_compiler::cligen::GenerationCommand::new()
        .generate(&common::request(&["collide.capnp"]))
        .map(|_| ())
        .expect_err("collision");
    let msg = err.to_string();
    // the message names the words and BOTH methods
    assert!(msg.contains("get the words `method-version`"), "{msg}");
    assert!(
        msg.contains("`collide/Collide.version` (Collide, command)")
            && msg.contains("`collide/Collide.methodVersion` (Collide, command)"),
        "{msg}"
    );
}

#[test]
fn a_navigation_word_and_a_command_with_the_same_words_are_an_error() {
    let err = capnp_compiler::cligen::GenerationCommand::new()
        .generate(&common::request(&["collide2.capnp"]))
        .map(|_| ())
        .expect_err("collision");
    let msg = err.to_string();
    assert!(msg.contains("get the words `method-serve`"), "{msg}");
    assert!(
        msg.contains("`collide2/Collide2.serve` (Collide2, navigation)")
            && msg.contains("`collide2/Collide2.methodServe` (Collide2, command)"),
        "{msg}"
    );
}
