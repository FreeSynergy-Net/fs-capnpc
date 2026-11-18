//! The cases of the fixture schemas: what is a tool, what is left out and why, and the shape of the schemas.

#[path = "mcp_json/mod.rs"]
mod support_mcp;

use capnp_compiler::mcpgen::rt::{Danger, Off};
use capnpc_cli_fixtures::{features_mcp, mcp_mcp, multi_mcp, tasks_mcp};
use support_mcp::{V, input, left, output, tool};

#[test]
fn tasks_tools_follow_the_command_tree() {
    let t = tasks_mcp::TOOLS;
    let create = tool(t, "tasks.items.create");
    assert_eq!(create.key, "tasks/Items.create");
    assert_eq!(create.words, ["items", "create"]);
    let i = input(create);
    assert_eq!(
        i.at("properties/jscalendar/type").and_then(V::str),
        Some("string")
    );
    assert_eq!(i.get("required").map(|r| r.arr().len()), Some(1));
    // the result is the struct, by reference, with its definition below
    assert!(output(create).at("properties/task/$ref").is_some());
    assert!(output(create).at("$defs").is_some());
    // 64-bit integers are strings, as the capnp-JSON codec writes them, with their bounds in `x-fs-int`
    let edit_if = input(tool(t, "tasks.items.edit-if"));
    assert_eq!(
        edit_if.at("properties/revision/type").and_then(V::str),
        Some("string")
    );
    assert_eq!(
        edit_if.at("properties/revision/pattern").and_then(V::str),
        Some("^[0-9]+$")
    );
    assert_eq!(
        edit_if
            .at("properties/revision/x-fs-int/maximum")
            .and_then(V::str),
        Some("18446744073709551615")
    );
    assert_eq!(
        edit_if
            .at("properties/revision/x-fs-int/minimum")
            .and_then(V::str),
        Some("0")
    );
    // a result is lax: what the codec always writes is required (a Bool), a pointer is not (it may be left out)
    let get = output(tool(t, "tasks.items.get"));
    let required: Vec<&str> = get
        .get("required")
        .unwrap()
        .arr()
        .iter()
        .filter_map(V::str)
        .collect();
    assert_eq!(required, ["found"]);
    // an enum is an enum
    let rel = input(tool(t, "tasks.items.create-relation"));
    let names: Vec<&str> = rel
        .at("properties/relation/enum")
        .unwrap()
        .arr()
        .iter()
        .filter_map(V::str)
        .collect();
    assert_eq!(&names[..3], ["first", "next", "child"]);
    assert!(names.contains(&"dependsOn"));
}

#[test]
fn tasks_left_out_has_the_listener_and_no_tool() {
    assert_eq!(
        left(tasks_mcp::LEFT, "tasks/Tasks.subscribe").as_deref(),
        Some("listener")
    );
    assert!(
        tasks_mcp::TOOLS
            .iter()
            .all(|t| t.key != "tasks/Tasks.subscribe")
    );
}

#[test]
fn features_cases() {
    let l = features_mcp::LEFT;
    assert_eq!(
        left(l, "features/Features.login").as_deref(),
        Some("secret")
    );
    assert_eq!(
        left(l, "features/Features.rawValue").as_deref(),
        Some("any-pointer")
    );
    assert_eq!(
        left(l, "features/Features.watch").as_deref(),
        Some("listener")
    );
    assert_eq!(
        left(l, "features/Features.watchTwo").as_deref(),
        Some("listener")
    );
    assert_eq!(
        left(l, "features/Sink.event"),
        None,
        "a listener is no root"
    );
    let t = features_mcp::TOOLS;
    for gone in ["features.login", "features.raw-value", "features.watch"] {
        assert!(t.iter().all(|x| x.name != gone), "{gone}");
    }
    // a destructive mark travels in the table and in the `_meta`
    let danger = tool(t, "features.danger");
    assert_eq!(danger.risk, Some(Danger::Destructive));
    // a union inside a struct: exactly one member (oneOf), the other fields required
    let place = input(tool(t, "features.place"));
    let shape = place.at("$defs/features.Shape").expect("Shape in $defs");
    assert_eq!(shape.get("oneOf").map(|o| o.arr().len()), Some(4));
    assert_eq!(shape.get("required").map(|r| r.arr().len()), Some(1));
    // ... but not in a result: the codec leaves out a null pointer and the default member of a union
    let placed = output(tool(t, "features.place"));
    let shape_out = placed.at("$defs/features.Shape").expect("Shape in $defs");
    assert!(shape_out.get("oneOf").is_none() && shape_out.get("required").is_none());
    assert_eq!(
        shape.at("properties/nothing/type").and_then(V::str),
        Some("null")
    );
    // a struct that contains itself ends: one definition, a reference inside it
    let tree = input(tool(t, "features.tree"));
    assert_eq!(
        tree.at("$defs/features.Node/properties/children/items/$ref")
            .and_then(V::str),
        Some("#/$defs/features.Node")
    );
    // groups are nested objects
    let draw = input(tool(t, "features.draw"));
    let line = draw.at("$defs/features.Line").unwrap();
    assert_eq!(
        line.at("properties/from/type").and_then(V::str),
        Some("object")
    );
    assert!(line.at("properties/from/properties/x").is_some());
    // widths: Int8 and UInt16 have their own bounds
    let at = input(tool(t, "features.create-at"));
    assert_eq!(
        at.at("properties/small/minimum"),
        Some(&V::Num("-128".into()))
    );
    assert_eq!(
        at.at("properties/small/maximum"),
        Some(&V::Num("127".into()))
    );
    // a float is a number, or one of the three strings the codec writes for infinity and not-a-number
    let any_of = at.at("properties/f/anyOf").unwrap().arr();
    assert_eq!(any_of.len(), 2);
    assert_eq!(any_of[0].get("type").and_then(V::str), Some("number"));
    assert_eq!(any_of[1].get("enum").unwrap().arr().len(), 3);
    assert_eq!(
        at.at("properties/when/pattern").and_then(V::str),
        Some("^-?[0-9]+$")
    );
    let blob = input(tool(t, "features.blob"));
    assert_eq!(
        blob.at("properties/data/contentEncoding").and_then(V::str),
        Some("base64")
    );
}

#[test]
fn mcp_fixture_cases() {
    let t = mcp_mcp::TOOLS;
    let names: Vec<&str> = t.iter().map(|x| x.name).collect();
    assert_eq!(
        names,
        [
            "mcp.find.by-id",
            "mcp.find.by-name",
            "mcp.find.everything",
            "mcp.add",
            "mcp.wipe",
            "mcp.count",
            "mcp.peeker.peek",
            "mcp.voids",
            "mcp.ratio"
        ]
    );
    // a union only: one tool for each member, each asks for its member and nothing else
    let by_id = tool(t, "mcp.find.by-id");
    assert_eq!(by_id.member, Some("byId"));
    assert_eq!(by_id.key, "mcp/Notes.find");
    let i = input(by_id);
    assert_eq!(i.at("properties").unwrap().keys(), ["byId"]);
    assert_eq!(i.get("required").unwrap().arr(), [V::Str("byId".into())]);
    assert_eq!(
        i.at("properties/byId/description").and_then(V::str),
        Some("The id of the note."),
        "{} / {}",
        by_id.description,
        by_id.input_schema
    );
    assert!(by_id.description.starts_with("Find notes."));
    let every = input(tool(t, "mcp.find.everything"));
    assert_eq!(
        every.at("properties/everything/type").and_then(V::str),
        Some("null")
    );
    // hints: a text kind is a format, any other field keeps the hint
    let add = tool(t, "mcp.add");
    assert_eq!(add.risk, Some(Danger::Add));
    assert_eq!(add.title, "Add a note.");
    let ai = input(add);
    assert_eq!(
        ai.at("properties/at/x-fs-kind").and_then(V::str),
        Some("timestamp")
    );
    assert_eq!(ai.at("properties/mood/enum").unwrap().arr().len(), 2);
    let note = output(add);
    assert_eq!(
        note.at("$defs/mcp.Note/properties/id/format")
            .and_then(V::str),
        Some("uuid")
    );
    assert_eq!(
        note.at("$defs/mcp.Note/description").and_then(V::str),
        Some("A note somebody wrote.")
    );
    // a method without a doc comment says so
    assert_eq!(
        tool(t, "mcp.count").description,
        "No description in the contract."
    );
    assert_eq!(tool(t, "mcp.count").title, "mcp.count");
    // no parameters: an object that takes nothing
    assert_eq!(input(tool(t, "mcp.count")).at("properties"), None);
    // left out, with the reason
    let l = mcp_mcp::LEFT;
    assert_eq!(left(l, "mcp/Vault.open").as_deref(), Some("excluded"));
    assert_eq!(left(l, "mcp/Notes.hidden").as_deref(), Some("excluded"));
    assert_eq!(left(l, "mcp/Notes.login").as_deref(), Some("secret"));
    assert_eq!(left(l, "mcp/Notes.raw").as_deref(), Some("any-pointer"));
    assert_eq!(left(l, "mcp/Notes.other").as_deref(), Some("capability"));
    // a way (a getter) that is excluded, or an interface on the way that is excluded: nothing below is a tool
    assert_eq!(left(l, "mcp/Inner.deep").as_deref(), Some("excluded"));
    assert_eq!(left(l, "mcp/Child.leaf").as_deref(), Some("excluded"));
    // no mark on the method: the mark of the interface counts
    assert_eq!(tool(t, "mcp.peeker.peek").risk, Some(Danger::Read));
    // a secret on a result field (the annotation) keeps the tool out, as on an argument
    assert_eq!(left(l, "mcp/Notes.token").as_deref(), Some("secret"));
    assert!(t.iter().all(|x| x.key != "mcp/Notes.token"));
    // a result: a Float is always written by the codec (required), a Text may be left out; a text in the result is
    // not free content of the arguments
    let ratio = tool(t, "mcp.ratio");
    let required: Vec<&str> = output(ratio)
        .get("required")
        .unwrap()
        .arr()
        .iter()
        .filter_map(V::str)
        .collect();
    assert_eq!(required, ["value"]);
    assert!(!ratio.free_content);
    // a Void field outside a union says nothing, in the arguments and in the result
    let voids = tool(t, "mcp.voids");
    assert_eq!(input(voids).at("properties").unwrap().keys(), ["n"]);
    assert_eq!(output(voids).at("properties").unwrap().keys(), ["ok"]);
    // free text is marked: `add` takes a text without a kind
    assert!(add.free_content && !tool(t, "mcp.count").free_content);
}

#[test]
fn several_roots_and_reserved_words() {
    let all = [
        multi_mcp::alpha::TOOLS,
        multi_mcp::beta::TOOLS,
        multi_mcp::gamma::TOOLS,
    ];
    let names: Vec<&str> = all.iter().flat_map(|t| t.iter().map(|x| x.name)).collect();
    assert_eq!(
        names,
        [
            "multi.alpha.first",
            "multi.beta.second",
            "multi.gamma.third"
        ]
    );
    assert!(multi_mcp::alpha::tools(&Off).is_empty() && multi_mcp::gamma::tools(&Off).is_empty());
    assert_ne!(multi_mcp::beta::TOOLS[0].words.len(), 0);
}
