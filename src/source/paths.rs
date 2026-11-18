//! Path arithmetic for schema lookup: directories, lexical normalisation, absolute components.
//!
//! Pattern: Newtype-style helpers — pure functions over path components, no I/O of their own.

use std::path::{Path, PathBuf};

/// A search location: a directory on disk or the embedded standard files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dir {
    /// A directory in the file system (or virtual root).
    Disk(PathBuf),
    /// The embedded `/capnp/*.capnp` files.
    Std,
}

/// Lexically normalise a path into components (resolves `.` and `..`). Returns `None` when the path
/// escapes the root.
pub fn eval_components(base: &[String], rel: &str) -> Option<Vec<String>> {
    let mut out: Vec<String> = base.to_vec();
    for part in rel.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                out.pop()?;
            }
            p => out.push(p.to_string()),
        }
    }
    Some(out)
}

/// An absolute path as a root plus normalised components, parsed from text.
///
/// The parsing is a pure function of the text and a `windows` flag, never of `std::path`, so the same
/// text gives the same result on every host (the tests run the Windows rules on Linux, too). The root
/// is `/` (Unix, or a Windows root without drive), `C:/` (drive) or `//server/share/` (UNC) and
/// always ends with `/`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbsPath {
    /// Root with its drive or share, ending in `/`.
    pub root: String,
    /// Components below the root, without `.`, `..` and empty parts.
    pub comps: Vec<String>,
}

impl AbsPath {
    /// The plain root `/`.
    #[must_use]
    pub fn unix_root() -> Self {
        AbsPath {
            root: "/".into(),
            comps: vec![],
        }
    }

    /// The current directory of the process (`/` when it cannot be read).
    #[must_use]
    pub fn cwd() -> Self {
        let text = std::env::current_dir().map(|p| p.to_string_lossy().into_owned());
        text.map_or_else(
            |_| Self::unix_root(),
            |t| Self::parse(&t, cfg!(windows), &Self::unix_root()),
        )
    }

    /// Parse `text`; a relative path is taken below `base`. With `windows`, `\` separates like `/`,
    /// and drives (`C:`, `\\?\C:`) and UNC shares are roots; without it `\` is an ordinary name character.
    #[must_use]
    pub fn parse(text: &str, windows: bool, base: &AbsPath) -> AbsPath {
        let sep = |c: char| c == '/' || (windows && c == '\\');
        let text = if windows {
            text.strip_prefix("\\\\?\\").unwrap_or(text)
        } else {
            text
        };
        let b = text.as_bytes();
        let (root, rest, mut comps): (String, &str, Vec<String>) =
            if windows && b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
                (
                    format!("{}:/", char::from(b[0]).to_ascii_uppercase()),
                    &text[2..],
                    vec![],
                )
            } else if windows && text.starts_with(|c| sep(c)) && text[1..].starts_with(|c| sep(c)) {
                let mut parts = text[2..].splitn(3, sep);
                let (server, share) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
                (
                    format!("//{server}/{share}/"),
                    parts.next().unwrap_or(""),
                    vec![],
                )
            } else if text.starts_with(sep) {
                ("/".to_string(), text, vec![])
            } else {
                (base.root.clone(), text, base.comps.clone())
            };
        for part in rest.split(sep) {
            match part {
                "" | "." => {}
                ".." => {
                    comps.pop();
                }
                p => comps.push(p.to_string()),
            }
        }
        AbsPath { root, comps }
    }

    /// The path as one `/`-separated string; the identity of a virtual file.
    #[must_use]
    pub fn key(&self) -> String {
        format!("{}{}", self.root, self.comps.join("/"))
    }

    /// The path for the host file system.
    #[must_use]
    pub fn to_path(&self) -> PathBuf {
        join_path(Path::new(&self.root), &self.comps)
    }
}

/// Split a file named on the command line into the directory and the path below it: the longest
/// source prefix wins, then the current directory, then the root of the file.
pub(super) fn split_source(
    file: &str,
    prefixes: &[String],
    cwd: &AbsPath,
    windows: bool,
) -> (PathBuf, Vec<String>) {
    let path = AbsPath::parse(file, windows, cwd);
    let below_dir = |p: &AbsPath| {
        same_start(&[p.root.clone()], &[path.root.clone()], windows)
            && same_start(&path.comps, &p.comps, windows)
    };
    let best = prefixes
        .iter()
        .map(|p| AbsPath::parse(p, windows, cwd))
        .filter(|p| !p.comps.is_empty() && path.comps.len() > p.comps.len() && below_dir(p))
        .max_by_key(|p| p.comps.len());
    let below = best.or_else(|| below_dir(cwd).then(|| cwd.clone()));
    match below {
        Some(dir) => (dir.to_path(), path.comps[dir.comps.len()..].to_vec()),
        None => (PathBuf::from(&path.root), path.comps),
    }
}

/// `path` starts with the components `start`; on Windows names compare without regard to ASCII case.
fn same_start(path: &[String], start: &[String], windows: bool) -> bool {
    path.len() >= start.len()
        && path.iter().zip(start).all(|(a, b)| {
            if windows {
                a.eq_ignore_ascii_case(b)
            } else {
                a == b
            }
        })
}

/// Key of a virtual file below `root`: `\` and `/` both separate, `.` and `..` are resolved.
pub(super) fn virtual_key(root: &Path, comps: &[String]) -> String {
    virtual_text(&format!("{}/{}", root.to_string_lossy(), comps.join("/")))
}

/// Normalised key of a virtual path given as text.
pub(super) fn virtual_text(text: &str) -> String {
    AbsPath::parse(text, true, &AbsPath::unix_root()).key()
}

/// Join path components below `root`.
pub(super) fn join_path(root: &Path, comps: &[String]) -> PathBuf {
    let mut p = root.to_path_buf();
    for c in comps {
        p.push(c);
    }
    p
}
