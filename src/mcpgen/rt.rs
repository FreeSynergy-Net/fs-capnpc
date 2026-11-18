//! The run-time side of the generated tool tables: plain data, the switch and the marks of an AI actor.
//!
//! Pattern: Strategy — the program decides through [`Switch`] whether the tools exist at all; the generated code
//! only holds data. Nothing here speaks the protocol.

/// Version of the contract between generated tool tables and this module. Generated code asserts it at compile time.
pub const ABI: u32 = 1;

/// The MCP revision the generated schemas and the result form are written for.
pub const PROTOCOL_VERSION: &str = "2026-07-28";

/// The key of our own entries inside `_meta` (reverse DNS, as the specification asks).
pub const META_KEY: &str = "net.freesynergy/mcp";

/// The `_meta` key a request carries the name of the client under (self-reported, so it labels and never decides).
pub const CLIENT_INFO_KEY: &str = "io.modelcontextprotocol/clientInfo";

/// The `resultType` of a finished call (`tools/call`, `tools/list`).
pub const RESULT_TYPE: &str = "complete";

/// Do two texts say the same? (`==` on `str` is not allowed in a `const`.)
#[must_use]
pub const fn same_text(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// What a call does to the data of people (the enumerants of `Danger` in `capnpc_cli.capnp`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Danger {
    /// Reads.
    Read,
    /// Adds.
    Add,
    /// Changes.
    Change,
    /// Destroys.
    Destructive,
    /// Acts in the name of a person.
    ActsInName,
    /// Never a tool.
    Excluded,
}

impl Danger {
    /// The name as the annotation spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Add => "add",
            Self::Change => "change",
            Self::Destructive => "destructive",
            Self::ActsInName => "actsInName",
            Self::Excluded => "excluded",
        }
    }
}

/// One tool, as data. The two schemas are JSON text (JSON Schema 2020-12), `meta` is the JSON text of the tool `_meta`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolSpec {
    /// Unique name: `tasks.items.create`.
    pub name: &'static str,
    /// Short human name (the first line of the doc comment, else the name).
    pub title: &'static str,
    /// The doc comment of the method (says so if there is none).
    pub description: &'static str,
    /// Coverage key `file/Interface.method`, the same as in `<name>_cli.rs`.
    pub key: &'static str,
    /// The union member this tool is for, if the method takes a union only (one tool per member).
    pub member: Option<&'static str>,
    /// The words of the command in `<name>_cli.rs`: `dispatch(words)` finds its function.
    pub words: &'static [&'static str],
    /// JSON Schema of the arguments (`type: object`).
    pub input_schema: &'static str,
    /// JSON Schema of the structured result.
    pub output_schema: &'static str,
    /// The `_meta` of the tool: `{"net.freesynergy/mcp": {...}}`, always with `"actor":"ai"`.
    pub meta: &'static str,
    /// The risk class from the schema marks; `None` = not marked.
    pub risk: Option<Danger>,
    /// The tool takes free text or bytes, which can hold what a person wrote (secrets included).
    pub free_content: bool,
}

/// Is the tool surface on? The generated tables are off until the program says otherwise.
pub trait Switch {
    /// True if the tools may be listed and called.
    fn is_on(&self) -> bool;
}

/// The default: off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Off;

impl Switch for Off {
    fn is_on(&self) -> bool {
        false
    }
}

/// The vault: off, always. Nothing turns it on, no setting and no flag.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Never;

impl Switch for Never {
    fn is_on(&self) -> bool {
        false
    }
}

/// The tools a switch lets through: all of `tools` if it is on, none otherwise.
#[must_use]
pub fn visible<'a>(switch: &dyn Switch, tools: &'a [ToolSpec]) -> &'a [ToolSpec] {
    if switch.is_on() { tools } else { &[] }
}

/// Text as a JSON string, with the quotes.
#[must_use]
pub fn json_string(s: &str) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The actor of a connection: `ai:<client name>`, an AI always. It can only be made by [`actor`], so no result can
/// carry a label that does not start with `ai:` (the type, not a convention, holds it).
///
/// The field is private: outside this module the type cannot be built by hand.
///
/// ```compile_fail
/// let _ = capnp_compiler::mcpgen::rt::AiActor(String::from("human:alice"));
/// ```
///
/// ```
/// assert_eq!(capnp_compiler::mcpgen::rt::actor(Some("human:alice")).as_str(), "ai:human_alice");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiActor(String);

impl AiActor {
    /// The label, `ai:x`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AiActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The longest client name kept in a label.
pub const ACTOR_NAME_MAX: usize = 64;

/// The actor of a connection: `ai:<client name>`. The name is what the client says about itself in the `_meta` of its
/// request (`io.modelcontextprotocol/clientInfo`): it labels, it never grants anything. Only `A-Z a-z 0-9 . _ - space`
/// are kept, every other character (a `:`, a control or direction character) becomes `_`, so `human:alice` is
/// `ai:human_alice`; at most [`ACTOR_NAME_MAX`] characters.
#[must_use]
pub fn actor(client_name: Option<&str>) -> AiActor {
    let name: String = client_name
        .unwrap_or_default()
        .trim()
        .chars()
        .take(ACTOR_NAME_MAX)
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ' ') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let name = name.trim();
    AiActor(format!(
        "ai:{}",
        if name.is_empty() { "unknown" } else { name }
    ))
}

/// The `_meta` of a result: who acted, marked as an AI.
#[must_use]
pub fn result_meta(actor: &AiActor) -> String {
    format!(
        "{{\"{META_KEY}\":{{\"actor\":{},\"ai\":true}}}}",
        json_string(actor.as_str())
    )
}
