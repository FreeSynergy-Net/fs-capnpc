//! The comment of a member is the comment behind it, also when the ordinals are not in the order written in the file
//! (`sourceInfo.members` follows the list of the node, not the `codeOrder`). Before 0.2.1 such a schema got the comment of
//! another member: wrong tool descriptions and wrong help of the command line.

#[path = "cligen_common/mod.rs"]
mod common;

#[test]
fn the_docs_of_fields_and_methods_are_their_own_whatever_the_order_of_the_ordinals() {
    let m = common::model(&["order.capnp"]);
    let odd = m
        .structs
        .values()
        .find(|s| s.name.ends_with("Odd"))
        .expect("Odd");
    let doc = |n: &str| {
        odd.fields
            .iter()
            .find(|f| f.name == n)
            .map(|f| f.doc.trim().to_owned())
    };
    assert_eq!(doc("first").as_deref(), Some("the first one"));
    assert_eq!(doc("second").as_deref(), Some("the second one"));
    assert_eq!(doc("third").as_deref(), Some("the third one"));
    let out = m
        .interfaces
        .values()
        .find(|i| i.name.ends_with("Out"))
        .expect("Out");
    let mdoc = |n: &str| {
        out.methods
            .iter()
            .find(|x| x.name == n)
            .map(|x| x.doc.trim().to_owned())
    };
    assert_eq!(mdoc("sooner").as_deref(), Some("runs sooner"));
    assert_eq!(mdoc("later").as_deref(), Some("runs later"));
}
