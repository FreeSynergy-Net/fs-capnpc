//! A tiny JSON value that writes itself as compact text, keys in the order they were put in.
//!
//! Pattern: Value Object — the schemas are built from it and written once; no serde, and no hash order in the output.

use super::rt::json_string;

/// A JSON value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Json {
    /// `true` / `false`.
    Bool(bool),
    /// A number as its digits.
    Num(String),
    /// A string.
    Str(String),
    /// An array.
    Arr(Vec<Json>),
    /// An object, keys in insertion order.
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// The compact text.
    #[must_use]
    pub fn text(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Self::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Self::Num(n) => out.push_str(n),
            Self::Str(s) => out.push_str(&json_string(s)),
            Self::Arr(items) => {
                out.push('[');
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write(out);
                }
                out.push(']');
            }
            Self::Obj(pairs) => {
                out.push('{');
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push_str(&json_string(k));
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }

    /// Append a member to an object (nothing happens to any other value).
    pub fn put(&mut self, key: &str, value: Self) {
        if let Self::Obj(pairs) = self {
            pairs.push((key.to_owned(), value));
        }
    }
}

/// An object from pairs.
#[must_use]
pub fn obj<const N: usize>(pairs: [(&str, Json); N]) -> Json {
    Json::Obj(pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}

/// A string value.
#[must_use]
pub fn s(text: &str) -> Json {
    Json::Str(text.to_owned())
}
