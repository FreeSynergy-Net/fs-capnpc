//! From the command tree to the tools: one tool per command (one per member for a method that takes a union only),
//! and the list of what was left out, with the reason.
//!
//! Pattern: Builder — one pass over the commands of a tree, in walk order, so the order of the tools never changes
//! between runs (the specification asks for a deterministic order).

use std::collections::BTreeSet;

use super::json::{Json, obj, s};
use super::rt::META_KEY;
use super::scan::scan;
use super::schema::Schema;
use crate::cligen::MarkSource;
use crate::cligen::model::{Annotation, Model, RISK_ANNOTATION, kebab};
use crate::cligen::tree::{Command, Tree};

/// The longest tool name the specification asks for.
const MAX_NAME: usize = 128;

/// What a method without a doc comment says.
pub const NO_DOC: &str = "No description in the contract.";

/// One tool, ready to be written.
#[derive(Debug)]
pub struct Tool {
    /// Unique name.
    pub name: String,
    /// Short human name.
    pub title: String,
    /// Description.
    pub description: String,
    /// Coverage key.
    pub key: String,
    /// The union member, if this is the tool of one.
    pub member: Option<String>,
    /// Words of the command in `<name>_cli.rs`.
    pub words: Vec<String>,
    /// Input schema text.
    pub input: String,
    /// Output schema text.
    pub output: String,
    /// `_meta` text.
    pub meta: String,
    /// The `Danger` variant, if the method is marked.
    pub risk: Option<&'static str>,
    /// Free text or bytes among the parameters.
    pub free: bool,
}

/// The tools of one tree and what was left out.
#[derive(Debug, Default)]
pub struct Table {
    /// Tools in walk order.
    pub tools: Vec<Tool>,
    /// (key, reason) of every command that is no tool.
    pub left: Vec<(String, &'static str)>,
}

fn is_excluded(a: &[Annotation]) -> bool {
    a.iter()
        .any(|x| x.id == RISK_ANNOTATION && x.value == "excluded")
}

/// The `Danger` variant of an annotation value.
fn variant(value: &str) -> Option<&'static str> {
    Some(match value {
        "read" => "Read",
        "add" => "Add",
        "change" => "Change",
        "destructive" => "Destructive",
        "actsInName" => "ActsInName",
        _ => return None,
    })
}

/// Is the command, the interface it belongs to, or a navigation method or interface on the way to it excluded?
fn excluded(m: &Model, marks: &dyn MarkSource, t: &Tree, c: &Command) -> bool {
    if is_excluded(&c.method.annotations)
        || is_excluded(&m.interfaces[&c.iface].annotations)
        || marks.excluded(&c.key)
    {
        return true;
    }
    t.groups
        .iter()
        .filter(|g| c.words.len() > g.words.len() && c.words[..g.words.len()] == g.words[..])
        .any(|g| {
            if marks.excluded(&g.key) {
                return true;
            }
            let (file, rest) = g.key.split_once('/').unwrap_or(("", &g.key));
            let (owner, method) = rest.rsplit_once('.').unwrap_or((rest, ""));
            m.interfaces
                .values()
                .filter(|i| i.file == file && i.name == owner)
                .any(|i| {
                    is_excluded(&i.annotations)
                        || i.methods
                            .iter()
                            .any(|me| me.name == method && is_excluded(&me.annotations))
                })
        })
}

/// The risk of a command: its own mark, else the mark of its interface, else the mark of the program.
fn risk_of(m: &Model, marks: &dyn MarkSource, c: &Command) -> Option<(&'static str, String)> {
    let find = |a: &[Annotation]| {
        a.iter()
            .find(|x| x.id == RISK_ANNOTATION)
            .map(|x| x.value.clone())
    };
    find(&c.method.annotations)
        .or_else(|| find(&m.interfaces[&c.iface].annotations))
        .or_else(|| marks.risk(&c.key))
        .and_then(|v| variant(&v).map(|d| (d, v)))
}

fn skip_reason(
    m: &Model,
    marks: &dyn MarkSource,
    t: &Tree,
    c: &Command,
) -> (Option<&'static str>, bool) {
    if c.method.streaming {
        return (Some("stream"), false);
    }
    if c.watch {
        return (Some("listener"), false);
    }
    if excluded(m, marks, t, c) {
        return (Some("excluded"), false);
    }
    let input = scan(m, marks, &c.key, c.method.params, true);
    if input.reason.is_some() {
        return (input.reason, false);
    }
    let output = scan(m, marks, &c.key, c.method.results, false);
    (output.reason, input.free)
}

fn meta(key: &str, member: Option<&str>, risk: Option<&str>, free: bool) -> String {
    let source = member.map_or_else(|| key.to_owned(), |mb| format!("{key}#{mb}"));
    obj([(
        META_KEY,
        obj([
            ("actor", s("ai")),
            ("risk", s(risk.unwrap_or("unknown"))),
            ("needsRelease", Json::Bool(true)),
            ("unmarked", Json::Bool(risk.is_none())),
            ("source", Json::Str(source)),
            ("freeContent", Json::Bool(free)),
        ]),
    )])
    .text()
}

/// The tools of a tree; `prefix` starts every name (`tasks`).
///
/// # Errors
/// Two tools get one name, or a name is longer than the specification allows.
pub fn build(m: &Model, marks: &dyn MarkSource, t: &Tree, prefix: &str) -> capnp::Result<Table> {
    let mut table = Table::default();
    let mut names = BTreeSet::new();
    for c in &t.commands {
        let (reason, free) = skip_reason(m, marks, t, c);
        if let Some(r) = reason {
            table.left.push((c.key.clone(), r));
            continue;
        }
        let params = &m.structs[&c.method.params];
        let members: Vec<Option<&str>> =
            if params.has_union && params.fields.iter().all(|f| f.union_member.is_some()) {
                params
                    .fields
                    .iter()
                    .map(|f| Some(f.name.as_str()))
                    .collect()
            } else {
                vec![None]
            };
        let output = Schema::new(m, marks, &c.key, false)
            .root(c.method.results, None)
            .text();
        let (danger, risk_value) = risk_of(m, marks, c).unzip();
        let doc = c.method.doc.trim();
        for member in members {
            let mut name = marks
                .tool_name(&c.key)
                .unwrap_or_else(|| format!("{prefix}.{}", c.words.join(".")));
            if let Some(mb) = member {
                name = format!("{name}.{}", kebab(mb));
            }
            if name.len() > MAX_NAME {
                return Err(capnp::Error::failed(format!(
                    "the tool name `{name}` is longer than {MAX_NAME} characters"
                )));
            }
            if !names.insert(name.clone()) {
                return Err(capnp::Error::failed(format!(
                    "two tools get the name `{name}` (`{}`)",
                    c.key
                )));
            }
            let member_doc = member
                .and_then(|mb| params.fields.iter().find(|f| f.name == mb))
                .map_or("", |f| f.doc.trim());
            let description = match (doc.is_empty(), member_doc.is_empty()) {
                (true, true) => NO_DOC.to_owned(),
                (false, true) => doc.to_owned(),
                (true, false) => member_doc.to_owned(),
                (false, false) => format!("{doc}\n\n{member_doc}"),
            };
            let title = if description == NO_DOC {
                name.clone()
            } else {
                description.lines().next().unwrap_or(&name).to_owned()
            };
            table.tools.push(Tool {
                title,
                description,
                key: c.key.clone(),
                member: member.map(str::to_owned),
                words: c.words.clone(),
                input: Schema::new(m, marks, &c.key, true)
                    .root(c.method.params, member)
                    .text(),
                output: output.clone(),
                meta: meta(&c.key, member, risk_value.as_deref(), free),
                risk: danger,
                free,
                name,
            });
        }
    }
    Ok(table)
}

/// The `Danger` of a tool as the generated code names it.
#[must_use]
pub fn risk_text(tool: &Tool) -> String {
    tool.risk
        .map_or_else(|| "None".to_owned(), |v| format!("Some(Danger::{v})"))
}
