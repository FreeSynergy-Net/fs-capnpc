//! T5: both ways to the command line code give the same bytes as the golden file: `capnp-compiler compile --cli <dir>`
//! (in one process) and the plugin binary `capnpc-cli` (the request on standard input, files into the current directory).
#![cfg(feature = "cli")]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn work(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("cligen-bins-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn golden() -> Vec<u8> {
    std::fs::read(root().join("tests/cligen/generated/tasks_cli.rs")).expect("golden file")
}

fn compile(extra: &[&Path], flag: &str) -> std::process::Output {
    let fixtures = root().join("tests/cligen/fixtures");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_capnp-compiler"));
    cmd.arg("compile").arg("--src-prefix").arg(&fixtures);
    cmd.arg(flag);
    for e in extra {
        cmd.arg(e);
    }
    cmd.arg(fixtures.join("tasks.capnp"));
    cmd.output().expect("run capnp-compiler")
}

#[test]
fn cli_option_writes_the_golden_file() {
    let out = work("option");
    let r = compile(&[&out], "--cli");
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    assert_eq!(std::fs::read(out.join("tasks_cli.rs")).unwrap(), golden());
}

#[test]
fn plugin_binary_writes_the_same_bytes() {
    let dir = work("plugin");
    let request = dir.join("tasks.request");
    let r = compile(&[&request], "--request");
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    let out = dir.join("out");
    std::fs::create_dir_all(&out).unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_capnpc-cli"))
        .current_dir(&out)
        .stdin(Stdio::from(std::fs::File::open(&request).unwrap()))
        .status()
        .expect("run capnpc-cli");
    assert!(status.success());
    assert_eq!(std::fs::read(out.join("tasks_cli.rs")).unwrap(), golden());
}

#[test]
fn plugin_binary_answers_version() {
    let r = Command::new(env!("CARGO_BIN_EXE_capnpc-cli"))
        .arg("--version")
        .output()
        .expect("run capnpc-cli");
    assert!(r.status.success());
    assert_eq!(
        String::from_utf8_lossy(&r.stdout),
        format!("capnpc-cli {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn cli_option_needs_a_value() {
    let r = Command::new(env!("CARGO_BIN_EXE_capnp-compiler"))
        .args(["compile", "a.capnp", "--cli"])
        .output()
        .expect("run capnp-compiler");
    assert_eq!(r.status.code(), Some(2));
}

#[test]
fn output_and_cli_options_together_write_both() {
    let dir = work("both");
    let (types, cli) = (dir.join("types"), dir.join("cli"));
    let r = compile(&[&types, Path::new("--cli"), &cli], "-o");
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    assert_eq!(std::fs::read(cli.join("tasks_cli.rs")).unwrap(), golden());
    assert!(types.join("tasks_capnp.rs").is_file());
}

#[test]
fn plugin_binary_help_is_zero_and_an_unknown_argument_is_two() {
    let help = Command::new(env!("CARGO_BIN_EXE_capnpc-cli"))
        .arg("--help")
        .output()
        .expect("run capnpc-cli");
    assert_eq!(help.status.code(), Some(0));
    let bad = Command::new(env!("CARGO_BIN_EXE_capnpc-cli"))
        .arg("foo")
        .output()
        .expect("run capnpc-cli");
    assert_eq!(bad.status.code(), Some(2));
    assert_eq!(bad.stdout.len(), 0);
}
