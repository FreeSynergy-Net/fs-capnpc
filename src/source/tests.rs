//! Tests for path evaluation and the loaders.
//!
//! Pattern: Strategy — the same assertions run against the in-memory and disk strategies.

use super::paths::split_source;
use super::*;
use crate::pipeline::DefaultFrontend;
use std::path::PathBuf;

#[test]
fn eval() {
    let base = vec!["a".to_string(), "b".to_string()];
    assert_eq!(
        eval_components(&base, "../c.capnp"),
        Some(vec!["a".into(), "c.capnp".into()])
    );
    assert_eq!(eval_components(&base, "../../../x"), None);
    assert_eq!(
        eval_components(&[], "capnp/c++.capnp"),
        Some(vec!["capnp".into(), "c++.capnp".into()])
    );
}

#[test]
fn std_import() {
    let mut l = ModuleLoader::new();
    let mut errs = vec![];
    let m = l
        .load_module(&Dir::Std, &["capnp".into(), "c++.capnp".into()], &mut errs)
        .unwrap();
    assert!(errs.is_empty(), "{errs:?}");
    assert!(
        l.import_relative(m, "/capnp/stream.capnp", &mut errs)
            .is_some()
    );
}

#[test]
fn memory_relative_import() {
    let mut mem = MemoryLoader::new();
    mem.add(
        "/w/a.capnp",
        "@0xdbb9ad1f14bf0b36;\nusing B = import \"b.capnp\";\n",
    );
    mem.add("/w/b.capnp", "@0xdbb9ad1f14bf0b37;\nstruct S {}\n");
    let mut l = ModuleLoader::with(Box::new(mem), Box::new(DefaultFrontend::standard()));
    let dir = Dir::Disk("/w".into());
    let mut errs = vec![];
    let a = l.load_module(&dir, &["a.capnp".into()], &mut errs).unwrap();
    assert!(l.import_relative(a, "b.capnp", &mut errs).is_some());
    assert!(errs.is_empty(), "{errs:?}");
    assert!(
        l.load_module(&dir, &["missing.capnp".into()], &mut errs)
            .is_none()
    );
}

fn win_cwd() -> AbsPath {
    AbsPath::parse(r"D:\a\repo", true, &AbsPath::unix_root())
}

#[test]
fn windows_text_keeps_drive_and_resolves_both_separators() {
    let p = AbsPath::parse(r"c:\Users\x\..\y/./hello.capnp", true, &win_cwd());
    assert_eq!(p.root, "C:/");
    assert_eq!(p.comps, ["Users", "y", "hello.capnp"]);
    assert_eq!(p.key(), "C:/Users/y/hello.capnp");
    let unc = AbsPath::parse(r"\\srv\share\d\f.capnp", true, &win_cwd());
    assert_eq!((unc.root.as_str(), unc.comps.len()), ("//srv/share/", 2));
    let ext = AbsPath::parse(r"\\?\C:\t\f.capnp", true, &win_cwd());
    assert_eq!(ext.key(), "C:/t/f.capnp");
}

#[test]
fn a_root_without_drive_is_not_put_below_the_current_directory() {
    // the old code joined "/w/a.capnp" below the cwd on Windows (it is not absolute there)
    let p = AbsPath::parse("/w/a.capnp", true, &win_cwd());
    assert_eq!(
        (p.root.as_str(), p.comps.as_slice()),
        ("/", &["w".into(), "a.capnp".into()][..])
    );
    let rel = AbsPath::parse(r"sub\a.capnp", true, &win_cwd());
    assert_eq!(rel.key(), "D:/a/repo/sub/a.capnp");
}

#[test]
fn unix_text_keeps_the_backslash_as_a_name_character() {
    let p = AbsPath::parse(r"/a\b/../c", false, &AbsPath::unix_root());
    assert_eq!(p.comps, ["c"]);
    let q = AbsPath::parse(r"/a\b/c", false, &AbsPath::unix_root());
    assert_eq!(q.comps, [r"a\b", "c"]);
}

#[test]
fn a_file_on_another_drive_keeps_its_drive() {
    // end_to_end_generates_rust on Windows: temp dir on C:, workspace on D:
    let (dir, path) = split_source(
        r"C:\Users\r\Temp\cli-1\hello.capnp",
        &[r"C:\Users\r\Temp\cli-1".to_string()],
        &win_cwd(),
        true,
    );
    assert_eq!(
        dir,
        PathBuf::from("C:/")
            .join("Users")
            .join("r")
            .join("Temp")
            .join("cli-1")
    );
    assert_eq!(path, ["hello.capnp"]);
    let (dir, path) = split_source(r"C:\x\y.capnp", &[], &win_cwd(), true);
    assert_eq!(
        (dir, path),
        (
            PathBuf::from("C:/"),
            vec!["x".to_string(), "y.capnp".into()]
        )
    );
    let (dir, path) = split_source(r"d:\A\repo\s\y.capnp", &[], &win_cwd(), true);
    assert_eq!(
        (dir, path),
        (
            AbsPath::parse(r"D:\a\repo", true, &win_cwd()).to_path(),
            vec!["s".to_string(), "y.capnp".into()]
        )
    );
}

#[test]
fn the_longest_source_prefix_wins_and_a_file_is_never_its_own_prefix() {
    let prefixes = [
        "/w".to_string(),
        "/w/sub".to_string(),
        "/w/sub/a.capnp".to_string(),
    ];
    let (dir, path) = split_source("/w/sub/a.capnp", &prefixes, &AbsPath::unix_root(), false);
    assert_eq!(
        (dir, path),
        (PathBuf::from("/w/sub"), vec!["a.capnp".to_string()])
    );
}

#[test]
fn memory_files_are_found_whatever_the_separators_or_dots() {
    let mut mem = MemoryLoader::new();
    mem.add(r"\w\dir\..\a.capnp", "@0xdbb9ad1f14bf0b36;\n");
    mem.add("/w/./sub//b.capnp", "x");
    mem.add(r"C:\w\c.capnp", "y");
    for root in ["/w", r"\w", "/w/", r"/w/sub/.."] {
        let d = Dir::Disk(root.into());
        assert!(mem.read(&d, &["a.capnp".into()]).is_some(), "{root}");
        assert_eq!(mem.identity(&d, &["a.capnp".into()]), "/w/a.capnp");
    }
    let d = Dir::Disk(r"\w".into());
    assert!(mem.read(&d, &["sub".into(), "b.capnp".into()]).is_some());
    assert!(mem.read(&d, &[r"sub\b.capnp".into()]).is_some());
    assert!(
        mem.read(&Dir::Disk("C:/w".into()), &["c.capnp".into()])
            .is_some()
    );
    assert!(mem.read(&d, &["c.capnp".into()]).is_none());
}

#[test]
fn memory_import_goes_up_with_dotdot_from_a_backslash_directory() {
    let mut mem = MemoryLoader::new();
    mem.add("/w/lib/b.capnp", "@0xdbb9ad1f14bf0b37;\nstruct S {}\n");
    mem.add(
        "/w/app/a.capnp",
        "@0xdbb9ad1f14bf0b36;\nusing B = import \"../lib/b.capnp\";\n",
    );
    let mut l = ModuleLoader::with(Box::new(mem), Box::new(DefaultFrontend::standard()));
    let dir = Dir::Disk(r"\w".into());
    let mut errs = vec![];
    let a = l
        .load_module(&dir, &["app".into(), "a.capnp".into()], &mut errs)
        .unwrap();
    assert!(l.import_relative(a, "../lib/b.capnp", &mut errs).is_some());
    assert!(errs.is_empty(), "{errs:?}");
}
