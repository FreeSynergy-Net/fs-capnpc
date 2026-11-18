# capnpc-cli

A Cap'n Proto code generator plugin that turns an interface into command line code.

`capnpc-rust` writes typed clients for your interface. `capnpc-cli` reads the same `CodeGeneratorRequest` and writes, for
every file, one more Rust file: a table of commands (one per method), and one function per command that parses nothing
and prints nothing itself. It only does what the types decide: it walks to the right capability, writes the
parameters with the typed setters, makes the call and walks the result. Where the values come from, how they are
shown, help texts, exit codes: that is your program, behind four small traits in `capnpc_cli::rt`.

**Status: version 0.0.x, not published.** It targets the output of `capnpc` 0.27 (the names of the generated types
must match; 0.28 changes some of them). A tool generator for MCP (`capnpc-mcp`) is a frame only.

## What is generated

For `tasks.capnp` with the root interface `Tasks`, `tasks_cli.rs` has

- `COMMANDS: &[Command]`: words (`["items", "create"]`), key (`tasks/Items.create`), interface id and ordinal,
  parameters with path, type, union membership, secret mark and doc comment, and whether the command watches;
- `GROUPS: &[Group]`: the words that only lead to commands (`items`);
- `run_items_create(root, input, output)` and one function for every other command;
- `dispatch(words)`, a `match` over the table (there is no extension point);
- for an interface that is passed as a parameter (a listener): a server `<Name>Lines<S: Sink>` that gives every call
  to the sink as one line, and answers only after the sink took it, and never with an error;
- helper functions per struct and enum (`set_*`, `out_*`, `enum_*`), so a struct that contains itself works;
- the head names the generator version and the ABI of `rt`; a wrong `rt` is a compile error, not a run-time one.

Covered: scalars, text, data, enums, structs, groups, unions (two members given is an error), lists of everything
(also lists of lists), defaults that are not 0 (a given 0 is written as 0), listeners. Not readable (`AnyPointer`,
generic types, capabilities in structs) are listed in the table as `ParamTy::Unreadable` with the reason, and shown in
the output as `Value::Unreadable`; nothing is dropped silently.

Words are the kebab case of the method names (`createRelation` becomes `create-relation`). A method on the top level that is named
like a word the program owns (`serve`, `mcp`, `help`, `version`) keeps its contract name, but its word becomes
`method-<word>` (`method-version`) and the table says `renamed: true` (also for the commands below such a method);
below the top level nothing is renamed. Two methods that end up with the same words are an error, never a silent overwrite.
A file with several interfaces that nothing leads to has one module per interface.

## Marks

`capnpc_cli.capnp` (in this crate) has the annotations `secret` (never an argument), `risk`, `resource`, `reason`
and `kind` (a hint for the basic type). Import it with `using Cli = import "/capnpc_cli.capnp";` and add the folder of
the file as an import path. Marks kept elsewhere come in through the `MarkSource` trait.

## Use

In a `build.rs`, with the pure Rust compiler (no C++):

```rust,ignore
let request = capnp_compiler::CompileCommand::new().file("x.capnp").run_bytes()?;
capnpc::codegen::CodeGenerationCommand::new().output_directory(&out).run(&request[..])?;
capnpc_cli::GenerationCommand::new().output_directory(&out).run(&request[..])?;
```

or as a plugin: `capnp compile -orust:out -ocli:out x.capnp`. The generated file needs `capnpc-cli` and, if it has
a listener, `capnp-rpc`. The output is plain, unformatted text: put `#[rustfmt::skip]` on the module that includes it.

A command is run like this:

```rust,ignore
let run = tasks_cli::dispatch(&["items", "create"]).expect("command");
let held = run(&root, &mut my_input, &mut my_output).await?;   // `held` keeps a subscription alive
```

## Determinism

The output depends on the request only: sorted maps, fixed indentation, no time, no absolute path, `/` in names.
The same request gives the same bytes, also with the nodes in another order.

## AI collaboration

Design and code were written with the help of an AI (Claude, Anthropic). Review is done by humans and by further
checks (the tests of this crate, compile and run tests against a fake server, CI gates).

Licence: MIT OR Apache-2.0.
