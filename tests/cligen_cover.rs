//! T1-T3: the command tree covers every method of the real tasks contract.

#[path = "cligen_common/mod.rs"]
mod common;

use capnp_compiler::cligen::tree::Tree;

/// The keys every method of the three interfaces would have, read straight from the model (independent of the tree).
fn required(m: &capnp_compiler::cligen::model::Model) -> Vec<String> {
    let mut keys = Vec::new();
    for name in ["Tasks", "Items", "Boards"] {
        let (_, i) = m.interface(name).expect("interface");
        keys.extend(
            i.methods
                .iter()
                .map(|me| format!("tasks/{name}.{}", me.name)),
        );
    }
    keys.sort();
    keys
}

fn tree(m: &capnp_compiler::cligen::model::Model) -> Tree {
    let (root, _) = m.interface("Tasks").expect("root");
    Tree::build(m, root).expect("tree")
}

fn offered_in_tasks(t: &Tree) -> Vec<String> {
    let mut v: Vec<String> = t
        .covered_keys()
        .into_iter()
        .filter(|k| k.starts_with("tasks/"))
        .map(str::to_owned)
        .collect();
    v.sort();
    v
}

#[test]
fn t1_tasks_methods_are_all_covered() {
    let m = common::model(&["tasks.capnp"]);
    let req = required(&m);
    assert_eq!(req.len(), 23, "6 on Tasks, 17 on Items, none on Boards");
    let t = tree(&m);
    assert_eq!(offered_in_tasks(&t), req);
    // 20 commands + 3 navigation words (`items`, `boards`, `types`) = the 23 methods
    let tasks_commands = t
        .commands
        .iter()
        .filter(|c| c.key.starts_with("tasks/"))
        .count();
    assert_eq!((tasks_commands, t.groups.len()), (20, 3));
}

#[test]
fn t2_mutant_items_delete_left_out_is_reported_as_gap() {
    let mut m = common::model(&["tasks.capnp"]);
    let req = required(&m);
    let (items, _) = m.interface("Items").expect("Items");
    m.interfaces
        .get_mut(&items)
        .expect("Items")
        .methods
        .retain(|me| me.name != "delete");
    let offered = offered_in_tasks(&tree(&m));
    let gaps: Vec<&String> = req.iter().filter(|k| !offered.contains(k)).collect();
    assert_eq!(gaps, ["tasks/Items.delete"]);
}

/// The commands of the hand-written fs-tasks-cli (`args.rs` at fs-tasks `9a2767f` has eight words) and the words that
/// replace them. `add` and `done` are conveniences: `add` builds the `JSCalendar` text, `done` is a workflow over
/// `items get` and `items edit-if`; the contract has only the plain methods.
const HAND_WRITTEN: &[(&str, &[&str])] = &[
    ("capabilities", &["capabilities"]),
    ("list", &["items", "list"]),
    ("show", &["items", "get"]),
    ("add", &["items", "create"]),
    ("done", &["items", "edit-if"]),
    ("delete", &["items", "delete"]),
    ("export", &["items", "export-ics"]),
    ("import", &["items", "import-ics"]),
];

#[test]
fn t3_hand_written_commands_have_a_generated_counterpart() {
    let t = tree(&common::model(&["tasks.capnp"]));
    for (old, words) in HAND_WRITTEN {
        assert!(
            t.commands.iter().any(|c| c.words == *words),
            "`{old}` has no generated command {words:?}"
        );
    }
}
