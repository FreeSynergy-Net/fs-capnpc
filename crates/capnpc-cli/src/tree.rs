//! The command tree below a root interface: `<interface-word> <method-word>` with one command per method.
//!
//! Pattern: Builder — one walk over the interfaces from the root; a method without parameters that returns exactly
//! one capability is a navigation step (a group word), every other method is a command.

use crate::model::{FieldKind, Method, Model, Ty, kebab, snake};

/// Words a program owns. A contract method with one of these names keeps its contract name; its word is prefixed.
pub const RESERVED: &[&str] = &["serve", "mcp", "help", "version"];

/// What a contract method named like a reserved word gets in front: `version` becomes `method-version`.
pub const RENAME_PREFIX: &str = "method-";

/// One call on the way to a command: a navigation method and the field of its result that holds the capability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    /// Name of the method.
    pub method: String,
    /// Name of the result field that holds the capability.
    pub field: String,
}

/// A command: one contract method.
#[derive(Clone, Debug)]
pub struct Command {
    /// Words after the program name: `["items", "create"]`.
    pub words: Vec<String>,
    /// Coverage key `file/Interface.method` (capnp spelling).
    pub key: String,
    /// Navigation calls before this one.
    pub path: Vec<Step>,
    /// Interface id that owns the method.
    pub iface: u64,
    /// The method.
    pub method: Method,
    /// The method takes a listener: the command watches.
    pub watch: bool,
    /// Name of the generated function, `run_items_create`.
    pub fn_name: String,
    /// The first word was a word the program owns (`version`) and got the prefix `method-`.
    pub renamed: bool,
}

/// A navigation word: covered by being the way to the commands below it.
#[derive(Clone, Debug)]
pub struct Group {
    /// Words after the program name.
    pub words: Vec<String>,
    /// Coverage key `file/Interface.method`.
    pub key: String,
}

/// The tree of one root interface.
#[derive(Debug, Default)]
pub struct Tree {
    /// Node id of the root interface.
    pub root: u64,
    /// Commands (leaves) in walk order.
    pub commands: Vec<Command>,
    /// Navigation words in walk order.
    pub groups: Vec<Group>,
}

impl Tree {
    /// Every covered key: commands and navigation words.
    #[must_use]
    pub fn covered_keys(&self) -> Vec<&str> {
        self.commands
            .iter()
            .map(|c| c.key.as_str())
            .chain(self.groups.iter().map(|g| g.key.as_str()))
            .collect()
    }
}

/// The interface a navigation method leads to, if the method is one.
fn navigation(m: &Model, me: &Method) -> Option<(u64, String)> {
    let (p, r) = (m.structs.get(&me.params)?, m.structs.get(&me.results)?);
    match (p.fields.as_slice(), r.fields.as_slice()) {
        ([], [f]) => match &f.kind {
            FieldKind::Slot(Ty::Interface(id)) if m.interfaces.contains_key(id) => {
                Some((*id, f.name.clone()))
            }
            _ => None,
        },
        _ => None,
    }
}

/// Does a method of the model take an interface (a listener) as a parameter?
#[must_use]
pub fn takes_listener(m: &Model, me: &Method) -> bool {
    m.structs.get(&me.params).is_some_and(|s| {
        s.fields
            .iter()
            .any(|f| matches!(f.kind, FieldKind::Slot(Ty::Interface(_))))
    })
}

/// The interfaces of a file that no navigation method and no parameter leads to: the roots of the command trees.
#[must_use]
pub fn roots(m: &Model, file_stem: &str) -> Vec<u64> {
    let mut reached = std::collections::BTreeSet::new();
    for i in m.interfaces.values() {
        for me in &i.methods {
            for sid in [me.params, me.results] {
                for f in m.structs.get(&sid).map_or(&[][..], |s| &s.fields[..]) {
                    if let FieldKind::Slot(Ty::Interface(id)) = &f.kind {
                        reached.insert(*id);
                    }
                }
            }
        }
    }
    m.interfaces
        .iter()
        .filter(|(id, i)| i.file == file_stem && !reached.contains(id))
        .map(|(id, _)| *id)
        .collect()
}

impl Tree {
    /// Build the tree below the root interface `root`.
    ///
    /// # Errors
    /// Two methods get the same words.
    pub fn build(m: &Model, root: u64) -> capnp::Result<Self> {
        let mut t = Self {
            root,
            ..Self::default()
        };
        walk(m, &[root], &[], &[], false, &mut t)?;
        let mut seen = std::collections::BTreeSet::new();
        let mut names = std::collections::BTreeMap::new();
        for c in &mut t.commands {
            if !seen.insert(c.words.clone()) {
                return Err(capnp::Error::failed(format!(
                    "two commands get the words `{}`",
                    c.words.join(" ")
                )));
            }
            let n = names.entry(c.fn_name.clone()).or_insert(0_u32);
            *n += 1;
            if *n > 1 {
                c.fn_name = format!("{}_{n}", c.fn_name);
            }
        }
        Ok(t)
    }
}

fn walk(
    m: &Model,
    stack: &[u64],
    words: &[String],
    path: &[Step],
    renamed: bool,
    t: &mut Tree,
) -> capnp::Result<()> {
    let iface = stack[stack.len() - 1];
    let i = &m.interfaces[&iface];
    let key_of = |me: &Method| format!("{}/{}.{}", i.file, i.name, me.name);
    for me in &i.methods {
        let mut w = kebab(&me.name);
        // the contract name stays; the word on the command line gets a prefix, and the table says so
        let clash = words.is_empty() && RESERVED.contains(&w.as_str());
        if clash {
            w = format!("{RENAME_PREFIX}{w}");
        }
        let renamed = renamed || clash;
        let mut all = words.to_vec();
        all.push(w);
        // a navigation that leads back to an interface on the way is a command (its result is a capability)
        let nav = navigation(m, me).filter(|(next, _)| !stack.contains(next));
        if let Some((next, field)) = nav {
            t.groups.push(Group {
                words: all.clone(),
                key: key_of(me),
            });
            let mut p = path.to_vec();
            p.push(Step {
                method: me.name.clone(),
                field,
            });
            let mut st = stack.to_vec();
            st.push(next);
            walk(m, &st, &all, &p, renamed, t)?;
            continue;
        }
        t.commands.push(Command {
            words: all,
            key: key_of(me),
            path: path.to_vec(),
            iface,
            method: me.clone(),
            watch: takes_listener(m, me),
            fn_name: format!(
                "run_{}_{}",
                snake(&i.name).replace('.', "_"),
                snake(&me.name)
            ),
            renamed,
        });
    }
    Ok(())
}
