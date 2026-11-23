//! The generated enum reader has a catch-all arm: a variant the schema did not know when the code was generated (the
//! enum grew later in the crate that owns it, e.g. `fs-types` `BasicType`) is reported as `Value::UnknownEnum(number)`,
//! the code still compiles, and nothing panics or is dropped silently.

#[path = "cligen_common/mod.rs"]
mod common;

use std::path::PathBuf;
use std::process::Command;

/// The text of `fn enum_features_capnp_colour` out of the generated file.
fn enum_fn_text() -> String {
    let t = common::generated(&["features.capnp", "capnpc_cli.capnp"], "features_cli.rs");
    let at = t
        .find("fn enum_features_capnp_colour(")
        .expect("enum reader");
    let rest = &t[at..];
    let end = rest.find("\n}\n").expect("end of the reader") + 3;
    rest[..end].to_owned()
}

#[test]
fn enum_reader_has_a_visible_catch_all() {
    let f = enum_fn_text();
    assert!(f.contains("-> Value"), "{f}");
    assert!(f.contains("Value::UnknownEnum("), "{f}");
}

#[test]
fn grown_enum_compiles_and_reports_the_unknown_variant() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("enum-fallback-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // The owner crate has one variant more than the schema of the generation (`Newer`).
    let program = format!(
        r#"#![allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum Value {{ Enum(&'static str), UnknownEnum(u16) }}
mod features_capnp {{
    #[repr(u16)]
    #[derive(Clone, Copy)]
    pub enum Colour {{ Red, DeepBlue, Green, Newer }}
}}
{}
fn main() {{
    use features_capnp::Colour;
    assert_eq!(enum_features_capnp_colour(Colour::DeepBlue), Value::Enum("deep-blue"));
    assert_eq!(enum_features_capnp_colour(Colour::Newer), Value::UnknownEnum(3));
}}
"#,
        enum_fn_text()
    );
    let src = dir.join("main.rs");
    std::fs::write(&src, program).unwrap();
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let bin = dir.join("prog");
    let built = Command::new(rustc)
        .args(["--edition", "2021", "-D", "warnings", "-o"])
        .arg(&bin)
        .arg(&src)
        .output()
        .expect("run rustc");
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let ran = Command::new(&bin).output().expect("run the program");
    assert!(
        ran.status.success(),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );
}
