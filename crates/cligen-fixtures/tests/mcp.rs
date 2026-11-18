//! The tool tables of the fixture schemas, compiled with the rest: what is a tool, what is left out and why, the
//! schemas as JSON (parsed here), the switch that is off, the label of the AI actor, the way to the command code.

#[path = "mcp_json/mod.rs"]
mod support_mcp;

use capnp_compiler::mcpgen::rt::{self, Danger, Never, Off, Switch, ToolSpec};
use capnpc_cli_fixtures::{features_mcp, mcp_mcp, multi_mcp, tasks_mcp, words_mcp};
use support_mcp::{On, V, input, output, parse};

fn every_table() -> Vec<(&'static str, &'static [ToolSpec])> {
    vec![
        ("tasks", tasks_mcp::TOOLS),
        ("features", features_mcp::TOOLS),
        ("words", words_mcp::TOOLS),
        ("mcp", mcp_mcp::TOOLS),
    ]
}

#[test]
fn every_tool_is_well_formed() {
    let mut count = 0;
    for (file, tools) in every_table() {
        let mut names = std::collections::BTreeSet::new();
        for t in tools {
            count += 1;
            assert!(names.insert(t.name), "{file}: name twice: {}", t.name);
            assert!(
                (1..=128).contains(&t.name.len())
                    && t.name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || "_-.".contains(c)),
                "{}",
                t.name
            );
            let (i, o) = (input(t), output(t));
            assert_eq!(i.get("type").and_then(V::str), Some("object"), "{}", t.name);
            assert_eq!(o.get("type").and_then(V::str), Some("object"), "{}", t.name);
            assert_eq!(
                i.get("additionalProperties"),
                Some(&V::Bool(false)),
                "{}",
                t.name
            );
            assert!(!t.title.is_empty() && !t.description.is_empty());
            // the label of the AI actor is in every tool, and the risk in the `_meta` is the one of the table
            let meta = parse(t.meta).unwrap();
            let ours = meta.get(rt::META_KEY).expect("our key in _meta");
            assert_eq!(ours.get("actor").and_then(V::str), Some("ai"), "{}", t.name);
            assert_eq!(
                ours.get("risk").and_then(V::str),
                Some(t.risk.map_or("unknown", Danger::as_str)),
                "{}",
                t.name
            );
            assert_ne!(t.risk, Some(Danger::Excluded));
            // the words reach the command code
            assert!(!t.words.is_empty() && !t.key.is_empty());
        }
    }
    assert!(count >= 30, "only {count} tools");
}

#[test]
fn the_way_to_the_command_code_exists_for_every_tool() {
    for t in tasks_mcp::TOOLS {
        assert!(tasks_mcp::runner(t).is_some(), "{}", t.name);
    }
    for t in features_mcp::TOOLS {
        assert!(features_mcp::runner(t).is_some(), "{}", t.name);
    }
    for t in mcp_mcp::TOOLS {
        assert!(mcp_mcp::runner(t).is_some(), "{}", t.name);
    }
    for t in words_mcp::TOOLS {
        assert!(words_mcp::runner(t).is_some(), "{}", t.name);
    }
    for t in multi_mcp::alpha::TOOLS {
        assert!(multi_mcp::alpha::runner(t).is_some(), "{}", t.name);
    }
}

#[test]
fn off_by_default_and_never_for_the_vault() {
    let defaults = [tasks_mcp::ENABLED_BY_DEFAULT, mcp_mcp::ENABLED_BY_DEFAULT];
    assert_eq!(defaults, [false, false]);
    for (file, tools) in every_table() {
        assert!(!tools.is_empty(), "{file} has tools");
    }
    assert_eq!(tasks_mcp::tools(&Off).len(), 0);
    assert_eq!(tasks_mcp::tools(&Never).len(), 0);
    assert_eq!(mcp_mcp::tools(&Never).len(), 0);
    assert_eq!(tasks_mcp::tools(&On).len(), tasks_mcp::TOOLS.len());
    assert!(!Off.is_on() && !Never.is_on());
}

#[test]
fn the_specification_date_is_the_one_we_target() {
    assert_eq!(rt::PROTOCOL_VERSION, "2026-07-28");
    assert_eq!(tasks_mcp::PROTOCOL_HINT, "2026-07-28");
    assert_eq!(mcp_mcp::PROTOCOL_HINT, rt::PROTOCOL_VERSION);
}

#[test]
fn the_actor_comes_from_the_client_name_and_is_always_an_ai() {
    assert_eq!(rt::actor(Some("Zed")), "ai:Zed");
    assert_eq!(rt::actor(None), "ai:unknown");
    assert_eq!(rt::actor(Some("  ")), "ai:unknown");
    assert_eq!(rt::actor(Some("a\nb")), "ai:ab");
    let meta = parse(&rt::result_meta(&rt::actor(Some("x\"y")))).unwrap();
    let ours = meta.get(rt::META_KEY).unwrap();
    assert_eq!(ours.get("actor").and_then(V::str), Some("ai:x\"y"));
    assert_eq!(ours.get("ai"), Some(&V::Bool(true)));
}
