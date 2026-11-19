//! What keeps a command out of the tool table: a secret, a value the generator cannot read, a capability.
//!
//! Pattern: Visitor — one walk over the parameters (or the results) of a method; the first reason found wins, in the
//! order of the fields, so the answer never depends on anything but the contract. Unclear means no tool.

use crate::cligen::MarkSource;
use crate::cligen::model::{Annotation, FieldKind, Model, Ty, kebab};

/// The outcome of one walk.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Scan {
    /// Why there is no tool, if there is no tool.
    pub reason: Option<&'static str>,
    /// A text or bytes field without a `kind` hint: it can hold what a person wrote.
    pub free: bool,
    /// A field marked `sensitive` (by the annotation, or for a parameter by the marks of the program).
    pub sensitive: bool,
}

struct Walk<'a> {
    m: &'a Model,
    marks: &'a dyn MarkSource,
    key: &'a str,
    params: bool,
    out: Scan,
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{prefix}.{name}")
    }
}

/// Walk the struct `sid`; `params` is true for the parameters of the command `key` (the marks of the program apply
/// there, with the same paths `cligen` uses), false for its results.
#[must_use]
pub fn scan(m: &Model, marks: &dyn MarkSource, key: &str, sid: u64, params: bool) -> Scan {
    let mut w = Walk {
        m,
        marks,
        key,
        params,
        out: Scan::default(),
    };
    w.walk(sid, "", &mut Vec::new());
    w.out
}

impl Walk<'_> {
    fn hit(&mut self, reason: &'static str) {
        if self.out.reason.is_none() {
            self.out.reason = Some(reason);
        }
    }

    fn walk(&mut self, sid: u64, prefix: &str, stack: &mut Vec<u64>) {
        let m = self.m;
        stack.push(sid);
        for f in &m.structs[&sid].fields {
            let path = join(prefix, &kebab(&f.name));
            let secret = f.annotations.iter().any(Annotation::is_secret)
                || (self.params && self.marks.secret(self.key, &path));
            if secret {
                self.hit("secret");
            }
            if f.annotations.iter().any(Annotation::is_sensitive)
                || (self.params && self.marks.sensitive(self.key, &path))
            {
                self.out.sensitive = true;
            }
            let kinded = f.annotations.iter().any(Annotation::is_kind)
                || (self.params && self.marks.kind(self.key, &path).is_some());
            match &f.kind {
                FieldKind::Group(g) => self.walk(*g, &path, stack),
                FieldKind::Slot(t) => self.ty(t, &path, kinded, stack),
            }
        }
        stack.pop();
    }

    fn ty(&mut self, t: &Ty, path: &str, kinded: bool, stack: &mut Vec<u64>) {
        match t {
            Ty::Interface(_) => self.hit("capability"),
            Ty::Unreadable(why) => self.hit(why),
            Ty::Text | Ty::Data => self.out.free |= self.params && !kinded,
            Ty::List(e) => self.ty(e, &format!("{path}.*"), kinded, stack),
            Ty::Struct(id) if !stack.contains(id) => self.walk(*id, path, stack),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::scan;
    use crate::CompileCommand;
    use crate::cligen::NoMarks;
    use crate::cligen::model::Model;
    use std::path::PathBuf;

    fn notes() -> Model {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut cmd = CompileCommand::new();
        cmd.src_prefix(root.join("tests/cligen/fixtures"));
        cmd.import_path(root.join("src/cligen"));
        cmd.file(root.join("tests/cligen/fixtures/mcp.capnp"));
        Model::from_request(&cmd.run_bytes().expect("fixture compiles")).expect("request reads")
    }

    /// The parameters and the results of the method `name` of `Notes`.
    fn method(m: &Model, name: &str) -> (String, u64, u64) {
        let (_, notes) = m.interface("Notes").expect("Notes");
        let me = notes.methods.iter().find(|x| x.name == name).expect(name);
        (format!("mcp/Notes.{name}"), me.params, me.results)
    }

    #[test]
    fn free_text_is_a_property_of_the_arguments_only() {
        let m = notes();
        let (key, params, results) = method(&m, "add");
        // `add` takes free text; its result holds a Text too (`Note.body`), which says nothing about the arguments
        assert!(scan(&m, &NoMarks, &key, params, true).free);
        assert!(!scan(&m, &NoMarks, &key, results, false).free);
        let (key, params, results) = method(&m, "ratio");
        assert!(!scan(&m, &NoMarks, &key, params, true).free);
        assert!(!scan(&m, &NoMarks, &key, results, false).free);
    }

    #[test]
    fn a_secret_annotation_counts_in_a_result_too() {
        let m = notes();
        let (key, params, results) = method(&m, "token");
        assert_eq!(
            scan(&m, &NoMarks, &key, results, false).reason,
            Some("secret")
        );
        assert_eq!(scan(&m, &NoMarks, &key, params, true).reason, None);
    }
}
