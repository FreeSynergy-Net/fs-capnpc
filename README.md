# FreeSynergy Capnpc — build the contracts with Rust alone

**This is a building block. It is why anyone with a Rust toolchain can build FreeSynergy, with no C++ compiler.**

Capnpc is a Cap'n Proto schema compiler written in pure Rust. FreeSynergy describes its interfaces in Cap'n Proto. The official compiler is written in C++, which means every contributor would need a C++ toolchain. This one reads the same schema files and produces the same output, so the normal Rust code generator can take over from there.

What is different here: we do not ask you to trust it. Its output is compared with the official compiler (capnp 1.4.0): every node, every source-info record and the generated Rust text, for the official test schemas, the FreeSynergy schemas and a set of hand-written feature schemas.

## What it can do today

Published on crates.io: `capnp-compiler` 0.1.1 (2026-11-06). This tree is version 0.2.0, not released yet (no tag, no upload).

- A command line, `capnp-compiler compile`, with output folder, import paths and source prefix.
- A library with a builder, so schemas can come from memory and errors can be collected elsewhere.
- Output identical in content to the official compiler for the schemas above.
- Used by `fs-interfaces` to regenerate its contracts.

## Install

```text
cargo install capnp-compiler          # installs capnp-compiler and the plugins capnpc-cli and capnpc-mcp
```

## Command line code from an interface (capnpc-cli)

The generator `capnp_compiler::cligen` turns an interface into command line code: for every file one more Rust file
(`<name>_cli.rs`) with a table of commands (one per method) and one function per command. It parses nothing and prints
nothing itself; where the values come from, how they are shown, help texts and exit codes are the program's, behind
four small traits in `capnp_compiler::cligen::rt`. Two ways, one generator:

```text
capnp-compiler compile -o src/gen --cli src/gen tasks.capnp
# or, with the official C++ tool and the plugin binary capnpc-cli (found through PATH):
capnp compile -orust:src/gen -ocli:src/gen tasks.capnp
```

`--cli` is the main way: the request stays in memory, no PATH and no C++ tool are needed, it works together with `-o`.
Generated code needs only the library: `capnp-compiler = { version = "0.2", default-features = false }` (the default
feature `cli` adds the two command lines and `capnpc`). The annotations the generator reads (`$Cli...`) are in
`src/cligen/capnpc_cli.capnp` (use `-I` on that folder). It targets the output of `capnpc` 0.27. See
[DESIGN.md](DESIGN.md), section "cligen".

Contracts that import types from another crate (`/types.capnp` of `fs-types`): give the generator the same map that `capnpc`
gets, `GenerationCommand::new().crates_provide_map(HashMap::from([(types_file_id, "fs_types".into())]))`; the types are then
named `::fs_types::types_capnp::...`.

## MCP tool tables from an interface (capnpc-mcp)

The generator `capnp_compiler::mcpgen` writes `<name>_mcp.rs` from the same request: the table `TOOLS` (one MCP tool per
command, with its input and output JSON Schema as text, a title and description from the doc comments, the risk class
and an AI label in `_meta`), the list `LEFT` of what is no tool and why (a `secret`, a capability, a listener, an
`AnyPointer`, `risk(excluded)`), and `runner`, which finds the command code in `<name>_cli.rs`. It speaks no protocol:
the program that serves MCP reads the table. The generated code is off until the program switches it on
(`ENABLED_BY_DEFAULT = false`, `tools(&Off)` is empty). Written for the MCP revision 2026-07-28 only.

```text
capnp-compiler compile -o src/gen --cli src/gen --mcp src/gen tasks.capnp
# or, with the official C++ tool:  capnp compile -orust:src/gen -ocli:src/gen -omcp:src/gen tasks.capnp
```

`--mcp` without `--cli` works, but the tables then call `<name>_cli.rs`, which must come from somewhere: the tool says so
on standard error. Both plugin binaries need the feature `cli` (as `capnpc-cli`): one switch decides which programs
`cargo install` puts on the path. See [DESIGN.md](DESIGN.md), section "mcpgen", for the mapping and the specification.


## What is not there yet

- For large schemas the layout of the message in memory differs from the official one. The content is identical and readers do not care.
- Only compilation to the request for the Rust generator, not other languages.

## Part of FreeSynergy

Capnpc keeps the contract repo buildable on any machine with Rust. FreeSynergy is a family of programs that are free for everyone, forever, and that keep your data yours. The documentation, the manifesto and the overall plan live in [fs-documentation](https://github.com/FreeSynergy/fs-documentation); the reading version is at <https://freesynergy.github.io/fs-documentation/>. Start with the [manifesto](https://freesynergy.github.io/fs-documentation/manifesto/) if you want to know why we do this.


## For developers

- One published package, `capnp-compiler`, with the library, the binaries `capnp-compiler` and `capnpc-cli` (`src/bin/capnpc-cli.rs`) and the generator for command line code in `src/cligen/` (see above); `crates/cligen-fixtures` is its test crate (not published).
- The compiler pipeline is a chain of traits (`SourceLoader`, `Frontend`, `ErrorReporter`, `Emitter`); `CompileCommand` composes them.
- Library use: `CompileCommand::new().file("schema/hello.capnp").run_bytes()?` gives the request for `capnpc::codegen`.
- Build and test: `cargo build` and `cargo test`. The tests use the official compiler (as WASM, `capnpc-embedded`) as an oracle.
- Design and the pattern map: [DESIGN.md](DESIGN.md). The status of each oracle schema is in `PLAN.md`. Credits in `NOTICE`.
- Licence: MIT OR Apache-2.0.
- AI collaboration: AI (Claude, Anthropic) took part in design and code. Review by humans and further checks (tests against the official compiler, CI gates).

## Reference

A Cap'n Proto schema compiler written in pure Rust. It reads `.capnp` schema files and produces the
`CodeGeneratorRequest` that `capnpc::codegen` (capnproto-rust) turns into Rust code - **without the
C++ `capnp` tool and without any C/C++ toolchain**.

The output is verified against the official compiler (capnp 1.4.0, run through `capnpc-embedded`
in the tests): every node, every source-info record and the generated Rust text are compared for the
official `test.capnp` family, the FreeSynergy schemas and a set of hand written feature schemas.

Part of the [FreeSynergy](https://freesynergy.github.io/fs-documentation/) project.

### Command line

```text
capnp-compiler compile -o <dir> [-I <import-dir>]... [--src-prefix <dir>]... files...
```

| Option | Meaning |
|---|---|
| `-o, --output <dir>` | write `<name>_capnp.rs` for every requested file into `<dir>` |
| `-I, --import-path <dir>` | search `<dir>` for absolute imports (`import "/foo.capnp"`) |
| `--src-prefix <dir>` | strip `<dir>` from file names (as `capnp compile --src-prefix`) |
| `--no-standard-import` | do not fall back to the embedded `/capnp/*.capnp` files |
| `--request <file>` | additionally write the raw serialized `CodeGeneratorRequest` |

### Library

```rust
use capnp_compiler::CompileCommand;

let mut cmd = CompileCommand::new();
cmd.src_prefix("schema").file("schema/hello.capnp");
let request: Vec<u8> = cmd.run_bytes()?;           // CodeGeneratorRequest, ready for capnpc
capnpc::codegen::CodeGenerationCommand::new()
    .output_directory("src/generated")
    .run(&request[..])?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Every stage is a trait (`SourceLoader`, `Frontend`, `ErrorReporter`, `Emitter`), so sources can come
from memory (`MemoryLoader`) and diagnostics can be collected elsewhere. See `DESIGN.md` for the
architecture and the pattern map, `PLAN.md` for the status of each oracle schema, and `examples/`.

### Known difference

The serialized request is content-identical to the official one, but for large schemas the *segment
layout* of the message differs (the C++ builder only fills its most recent segment). Readers such as
`capnpc` do not care.
