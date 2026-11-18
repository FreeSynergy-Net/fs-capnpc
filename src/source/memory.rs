//! In-memory [`SourceLoader`].
//!
//! Pattern: Strategy — an alternative backend holding virtual files, for tests and embedding.

use super::paths::{Dir, virtual_key, virtual_text};
use super::provider::SourceLoader;
use super::std_files::std_file;
use std::collections::HashMap;

/// Serves schema files from a map of virtual absolute paths (`/dir/file.capnp`).
///
/// Virtual paths are the same on every host: `/` and `\` separate, `.` and `..` are resolved, a drive
/// (`C:\dir`) is part of the path. Adding and reading go through one normalisation.
#[derive(Clone, Debug, Default)]
pub struct MemoryLoader {
    files: HashMap<String, Vec<u8>>,
}

impl MemoryLoader {
    /// An empty file set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add (or replace) a virtual file, e.g. `add("/src/a.capnp", "...")`; the path is normalised.
    pub fn add(&mut self, path: &str, content: impl Into<Vec<u8>>) -> &mut Self {
        self.files.insert(virtual_text(path), content.into());
        self
    }
}

impl SourceLoader for MemoryLoader {
    fn read(&self, dir: &Dir, path: &[String]) -> Option<Vec<u8>> {
        match dir {
            Dir::Disk(root) => self.files.get(&virtual_key(root, path)).cloned(),
            Dir::Std => std_file(path),
        }
    }

    fn identity(&self, dir: &Dir, path: &[String]) -> String {
        match dir {
            Dir::Disk(root) => virtual_key(root, path),
            Dir::Std => format!("std:{}", path.join("/")),
        }
    }
}
