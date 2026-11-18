//! The entry point: read a request, write the files.
//!
//! Pattern: Facade — `GenerationCommand` hides the stages (model, tree, emitters) behind one builder.

use std::path::PathBuf;

use super::ctx::{MarkSource, NoMarks};
use super::file::generate;
use crate::cligen::model::Model;

/// The name and version of this generator, as written into the head of every file.
pub const GENERATED_BY: &str = concat!("capnpc-cli ", env!("CARGO_PKG_VERSION"));

/// The path of the runtime part (`rt`) that the generated code calls; the one place that names it.
pub const RT_PATH: &str = "capnp_compiler::cligen::rt";

/// One generated file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedFile {
    /// File name, `tasks_cli.rs`.
    pub name: String,
    /// The Rust source.
    pub content: String,
}

/// Generate the command line code for a request.
pub struct GenerationCommand {
    output_directory: Option<PathBuf>,
    marks: Box<dyn MarkSource>,
}

impl Default for GenerationCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl GenerationCommand {
    /// A command with no output directory and no marks beyond the annotations.
    #[must_use]
    pub fn new() -> Self {
        Self {
            output_directory: None,
            marks: Box::new(NoMarks),
        }
    }

    /// Where `run` writes the files (default: the current directory, as a capnp plugin does).
    pub fn output_directory(&mut self, dir: impl Into<PathBuf>) -> &mut Self {
        self.output_directory = Some(dir.into());
        self
    }

    /// Marks kept outside the schema (they add to the annotations).
    pub fn marks(&mut self, marks: Box<dyn MarkSource>) -> &mut Self {
        self.marks = marks;
        self
    }

    /// The files for a request, without writing anything.
    ///
    /// # Errors
    /// The request is malformed, or two commands get the same words (a method named like a reserved word is no error: its
    /// word gets the prefix `method-`).
    pub fn generate(&self, request: &[u8]) -> capnp::Result<Vec<GeneratedFile>> {
        generate(&Model::from_request(request)?, &*self.marks)
    }

    /// Read a request and write the files into the output directory.
    ///
    /// # Errors
    /// As `generate`, or a file cannot be written.
    pub fn run(&self, mut request: impl std::io::Read) -> capnp::Result<()> {
        let mut bytes = Vec::new();
        request.read_to_end(&mut bytes)?;
        let dir = self.output_directory.clone().unwrap_or_default();
        for f in self.generate(&bytes)? {
            std::fs::write(dir.join(&f.name), f.content)?;
        }
        Ok(())
    }
}
