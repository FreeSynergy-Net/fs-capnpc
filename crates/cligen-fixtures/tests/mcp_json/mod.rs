//! A small JSON reader for the tests (the fixtures crate has no serde): text in, a tree out.
#![allow(dead_code)]

use capnp_compiler::mcpgen::rt::{Switch, ToolSpec};

/// A JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum V {
    Null,
    Bool(bool),
    Num(String),
    Str(String),
    Arr(Vec<V>),
    Obj(Vec<(String, V)>),
}

impl V {
    pub fn get(&self, key: &str) -> Option<&V> {
        match self {
            V::Obj(p) => p.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn str(&self) -> Option<&str> {
        match self {
            V::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn arr(&self) -> &[V] {
        match self {
            V::Arr(a) => a,
            _ => &[],
        }
    }

    pub fn keys(&self) -> Vec<&str> {
        match self {
            V::Obj(p) => p.iter().map(|(k, _)| k.as_str()).collect(),
            _ => Vec::new(),
        }
    }

    /// `a/b/c` through objects (a key may hold dots).
    pub fn at(&self, path: &str) -> Option<&V> {
        path.split('/').try_fold(self, |v, k| v.get(k))
    }
}

struct P<'a> {
    b: &'a [u8],
    i: usize,
}

pub fn parse(text: &str) -> Result<V, String> {
    let mut p = P {
        b: text.as_bytes(),
        i: 0,
    };
    let v = p.value()?;
    p.ws();
    if p.i != p.b.len() {
        return Err(format!("trailing text at {}", p.i));
    }
    Ok(v)
}

impl P<'_> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\n' | b'\r' | b'\t') {
            self.i += 1;
        }
    }

    fn eat(&mut self, c: u8) -> Result<(), String> {
        self.ws();
        if self.b.get(self.i) == Some(&c) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("expected `{}` at {}", c as char, self.i))
        }
    }

    fn lit(&mut self, word: &str, v: V) -> Result<V, String> {
        if self.b[self.i..].starts_with(word.as_bytes()) {
            self.i += word.len();
            Ok(v)
        } else {
            Err(format!("bad literal at {}", self.i))
        }
    }

    fn value(&mut self) -> Result<V, String> {
        self.ws();
        match self.b.get(self.i).copied() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => self.string().map(V::Str),
            Some(b't') => self.lit("true", V::Bool(true)),
            Some(b'f') => self.lit("false", V::Bool(false)),
            Some(b'n') => self.lit("null", V::Null),
            Some(c) if c == b'-' || c.is_ascii_digit() => {
                let start = self.i;
                while self.i < self.b.len()
                    && matches!(
                        self.b[self.i],
                        b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'
                    )
                {
                    self.i += 1;
                }
                Ok(V::Num(
                    String::from_utf8_lossy(&self.b[start..self.i]).into_owned(),
                ))
            }
            _ => Err(format!("unexpected input at {}", self.i)),
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.eat(b'"')?;
        let mut out = Vec::new();
        loop {
            let c = *self.b.get(self.i).ok_or("unterminated string")?;
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let e = *self.b.get(self.i).ok_or("bad escape")?;
                    self.i += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let hex = std::str::from_utf8(&self.b[self.i..self.i + 4])
                                .map_err(|e| e.to_string())?;
                            let n = u32::from_str_radix(hex, 16).map_err(|e| e.to_string())?;
                            self.i += 4;
                            let ch = char::from_u32(n).ok_or("bad unicode escape")?;
                            out.extend_from_slice(ch.to_string().as_bytes());
                        }
                        other => out.push(other),
                    }
                }
                c if c < 0x20 => return Err("control character in string".into()),
                c => out.push(c),
            }
        }
        String::from_utf8(out).map_err(|e| e.to_string())
    }

    fn array(&mut self) -> Result<V, String> {
        self.eat(b'[')?;
        let mut items = Vec::new();
        self.ws();
        if self.b.get(self.i) == Some(&b']') {
            self.i += 1;
            return Ok(V::Arr(items));
        }
        loop {
            items.push(self.value()?);
            self.ws();
            match self.b.get(self.i) {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    return Ok(V::Arr(items));
                }
                _ => return Err(format!("bad array at {}", self.i)),
            }
        }
    }

    fn object(&mut self) -> Result<V, String> {
        self.eat(b'{')?;
        let mut pairs: Vec<(String, V)> = Vec::new();
        self.ws();
        if self.b.get(self.i) == Some(&b'}') {
            self.i += 1;
            return Ok(V::Obj(pairs));
        }
        loop {
            self.ws();
            let k = self.string()?;
            if pairs.iter().any(|(x, _)| *x == k) {
                return Err(format!("duplicate key `{k}`"));
            }
            self.eat(b':')?;
            pairs.push((k, self.value()?));
            self.ws();
            match self.b.get(self.i) {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    return Ok(V::Obj(pairs));
                }
                _ => return Err(format!("bad object at {}", self.i)),
            }
        }
    }
}

pub struct On;
impl Switch for On {
    fn is_on(&self) -> bool {
        true
    }
}

pub fn tool<'a>(t: &'a [ToolSpec], name: &str) -> &'a ToolSpec {
    t.iter()
        .find(|x| x.name == name)
        .unwrap_or_else(|| panic!("no tool {name}"))
}

pub fn input(t: &ToolSpec) -> V {
    parse(t.input_schema).unwrap_or_else(|e| panic!("{}: input schema: {e}", t.name))
}

pub fn output(t: &ToolSpec) -> V {
    parse(t.output_schema).unwrap_or_else(|e| panic!("{}: output schema: {e}", t.name))
}

pub fn left(l: &[(&str, &str)], key: &str) -> Option<String> {
    l.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, r)| (*r).to_owned())
}
