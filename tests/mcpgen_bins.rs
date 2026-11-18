//! Both ways to the tool tables give the same bytes as the golden file: `capnp-compiler compile --mcp <dir>` (in one
//! process) and the plugin binary `capnpc-mcp` (the request on standard input, files into the current directory).
#![cfg(feature = "cli")]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn work(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("mcpgen-bins-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn golden() -> Vec<u8> {
    std::fs::read(root().join("tests/mcpgen/generated/tasks_mcp.rs")).expect("golden file")
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
fn mcp_option_writes_the_golden_file() {
    let out = work("option");
    let r = compile(&[&out], "--mcp");
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    assert_eq!(std::fs::read(out.join("tasks_mcp.rs")).unwrap(), golden());
    // alone, the tables have no command code to call: said, not silent
    assert!(String::from_utf8_lossy(&r.stderr).contains("note:"));
    assert!(!out.join("tasks_cli.rs").exists());
}

#[test]
fn plugin_binary_writes_the_same_bytes() {
    let dir = work("plugin");
    let request = dir.join("tasks.request");
    let r = compile(&[&request], "--request");
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    let out = dir.join("out");
    std::fs::create_dir_all(&out).unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_capnpc-mcp"))
        .current_dir(&out)
        .stdin(Stdio::from(std::fs::File::open(&request).unwrap()))
        .status()
        .expect("run capnpc-mcp");
    assert!(status.success());
    assert_eq!(std::fs::read(out.join("tasks_mcp.rs")).unwrap(), golden());
}

#[test]
fn plugin_binary_answers_version() {
    let r = Command::new(env!("CARGO_BIN_EXE_capnpc-mcp"))
        .arg("--version")
        .output()
        .expect("run capnpc-mcp");
    assert!(r.status.success());
    assert_eq!(
        String::from_utf8_lossy(&r.stdout),
        format!("capnpc-mcp {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn mcp_option_needs_a_value() {
    let r = Command::new(env!("CARGO_BIN_EXE_capnp-compiler"))
        .args(["compile", "a.capnp", "--mcp"])
        .output()
        .expect("run capnp-compiler");
    assert_eq!(r.status.code(), Some(2));
}

#[test]
fn cli_and_mcp_options_together_write_both_and_say_nothing() {
    let dir = work("both");
    let (cli, mcp) = (dir.join("cli"), dir.join("mcp"));
    let r = compile(&[&mcp, Path::new("--cli"), &cli], "--mcp");
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    assert_eq!(std::fs::read(mcp.join("tasks_mcp.rs")).unwrap(), golden());
    assert!(cli.join("tasks_cli.rs").is_file());
    assert!(!String::from_utf8_lossy(&r.stderr).contains("note:"));
}

#[test]
fn plugin_binary_help_is_zero_and_an_unknown_argument_is_two() {
    let help = Command::new(env!("CARGO_BIN_EXE_capnpc-mcp"))
        .arg("--help")
        .output()
        .expect("run capnpc-mcp");
    assert_eq!(help.status.code(), Some(0));
    let bad = Command::new(env!("CARGO_BIN_EXE_capnpc-mcp"))
        .arg("foo")
        .output()
        .expect("run capnpc-mcp");
    assert_eq!(bad.status.code(), Some(2));
    assert_eq!(bad.stdout.len(), 0);
}

#[test]
fn garbage_on_standard_input_is_an_error_not_a_stub() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_capnpc-mcp"))
        .current_dir(work("garbage"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run capnpc-mcp");
    {
        use std::io::Write;
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"not a request")
            .unwrap();
    }
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).starts_with("capnpc-mcp: "));
}
