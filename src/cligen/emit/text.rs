//! A small indenting text buffer.
//!
//! Pattern: Builder — lines in, one `String` out; the indentation is fixed (four spaces), so the output is stable.

/// The text being written.
#[derive(Default)]
pub struct Text {
    buf: String,
    depth: usize,
}

impl Text {
    /// One line at the current depth (an empty line carries no spaces).
    pub fn line(&mut self, s: impl AsRef<str>) {
        let s = s.as_ref();
        if !s.is_empty() {
            for _ in 0..self.depth {
                self.buf.push_str("    ");
            }
            self.buf.push_str(s);
        }
        self.buf.push('\n');
    }

    /// A line that opens a block: the following lines are indented.
    pub fn open(&mut self, s: impl AsRef<str>) {
        self.line(s);
        self.depth += 1;
    }

    /// A line that closes a block.
    pub fn close(&mut self, s: impl AsRef<str>) {
        self.depth = self.depth.saturating_sub(1);
        self.line(s);
    }

    /// An empty line.
    pub fn blank(&mut self) {
        self.buf.push('\n');
    }

    /// The text.
    #[must_use]
    pub fn finish(self) -> String {
        self.buf
    }
}
