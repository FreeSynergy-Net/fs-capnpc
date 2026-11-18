//! Tests for the edge cases of path arithmetic (`AbsPath`, `split_source`) and the host wiring.
//!
//! Pattern: Strategy — `AbsPath` is pure text, so the Windows rules are checked on every host.

use super::paths::split_source;
use super::*;
use std::path::PathBuf;

fn unix() -> AbsPath {
    AbsPath::unix_root()
}

fn win_cwd() -> AbsPath {
    AbsPath::parse(r"D:\a\repo", true, &unix())
}

#[cfg(unix)]
#[test]
fn a_unix_host_reads_the_source_file_with_the_unix_rules() {
    let l = ModuleLoader::new();
    // `\` is an ordinary name character, `C:` is no drive: nothing is split or re-rooted
    let (dir, path) = l.interpret_source_file(std::path::Path::new(r"/w/a\b.capnp"));
    assert_eq!(
        (dir, path),
        (
            Dir::Disk("/".into()),
            vec!["w".to_string(), r"a\b.capnp".into()]
        )
    );
    let (dir, path) = l.interpret_source_file(std::path::Path::new(r"C:\x.capnp"));
    let cwd = std::env::current_dir().unwrap();
    assert_eq!(dir, Dir::Disk(cwd));
    assert_eq!(path.last().map(String::as_str), Some(r"C:\x.capnp"));
}

#[test]
fn a_file_that_is_an_ancestor_of_the_current_directory_does_not_panic() {
    let cwd = AbsPath::parse("/a/repo", false, &unix());
    for windows in [false, true] {
        let (dir, path) = split_source("/a", &[], &cwd, windows);
        assert_eq!((dir, path), (PathBuf::from("/"), vec!["a".to_string()]));
        // also as a source prefix of its own and the file being the directory itself
        let (dir, path) = split_source("/a", &["/a".to_string()], &cwd, windows);
        assert_eq!((dir, path), (PathBuf::from("/"), vec!["a".to_string()]));
    }
    let (dir, path) = split_source("/a/repo", &[], &cwd, false);
    assert_eq!((dir, path), (PathBuf::from("/a/repo"), vec![]));
}

#[test]
fn a_unc_path_with_forward_slashes_is_a_unc_root() {
    let p = AbsPath::parse("//srv/share/d/f.capnp", true, &win_cwd());
    assert_eq!(
        (p.root.as_str(), p.comps.as_slice()),
        ("//srv/share/", &["d".into(), "f.capnp".into()][..])
    );
    let mixed = AbsPath::parse(r"//srv\share/d\f.capnp", true, &win_cwd());
    assert_eq!(mixed, p);
    // on a Unix host `//srv/share/d` is `/srv/share/d`
    let u = AbsPath::parse("//srv/share/d", false, &unix());
    assert_eq!((u.root.as_str(), u.comps.len()), ("/", 3));
}

#[test]
fn a_verbatim_unc_path_is_the_unc_path_not_a_relative_one() {
    // `canonicalize()` on Windows returns this form
    let want = AbsPath::parse(r"\\srv\share\x", true, &win_cwd());
    for text in [r"\\?\UNC\srv\share\x", r"\\?\unc\srv\share\x"] {
        let p = AbsPath::parse(text, true, &win_cwd());
        assert_eq!(p, want, "{text}");
        assert_eq!(p.key(), "//srv/share/x");
    }
    // a verbatim drive path stays a drive path; "UNC" alone as a folder name is just a name
    assert_eq!(
        AbsPath::parse(r"\\?\C:\UNC\x", true, &win_cwd()).key(),
        "C:/UNC/x"
    );
    assert_eq!(
        AbsPath::parse(r"UNC\srv\x", true, &win_cwd()).key(),
        "D:/a/repo/UNC/srv/x"
    );
    // only the marker `UNC` plus a separator makes a verbatim path a share
    assert_eq!(
        AbsPath::parse(r"\\?\ABC\srv\x", true, &win_cwd()).key(),
        "D:/a/repo/ABC/srv/x"
    );
    assert_eq!(
        AbsPath::parse(r"\\?\UNCsrv\x", true, &win_cwd()).key(),
        "D:/a/repo/UNCsrv/x"
    );
}

#[test]
fn a_drive_relative_path_is_not_made_absolute() {
    // same drive as the base: relative to the current directory of that drive
    assert_eq!(
        AbsPath::parse("d:foo", true, &win_cwd()).key(),
        "D:/a/repo/foo"
    );
    assert_eq!(
        AbsPath::parse(r"D:..\foo", true, &win_cwd()).key(),
        "D:/a/foo"
    );
    // another drive: its current directory is unknown, the path stays drive-relative (`C:foo`, never `C:\foo`)
    let p = AbsPath::parse("c:foo/bar", true, &win_cwd());
    assert_eq!((p.root.as_str(), p.comps.len()), ("C:", 2));
    assert_eq!(p.key(), "C:foo/bar");
    assert_ne!(p.key(), "C:/foo/bar");
    assert_eq!(AbsPath::parse(r"C:\foo", true, &win_cwd()).key(), "C:/foo");
    // and a drive-relative file is not taken for a file below the cwd or the root of C:
    let (dir, path) = split_source("C:foo.capnp", &[], &win_cwd(), true);
    assert_eq!(
        (dir, path),
        (PathBuf::from("C:"), vec!["foo.capnp".to_string()])
    );
}

#[test]
fn a_server_without_a_share_is_not_a_double_slash_root() {
    for text in ["//srv", r"\\srv", "//srv/", r"\\?\UNC\srv"] {
        let p = AbsPath::parse(text, true, &win_cwd());
        assert_eq!((p.root.as_str(), p.comps.len()), ("//srv/", 0), "{text}");
        assert_eq!(p.key(), "//srv/", "{text}");
    }
    assert_eq!(AbsPath::parse("//srv/x", true, &win_cwd()).root, "//srv/x/");
    // no server at all: the plain root
    assert_eq!(AbsPath::parse("//", true, &win_cwd()).root, "/");
    assert_eq!(AbsPath::parse("///a", true, &win_cwd()).key(), "/a");
}
