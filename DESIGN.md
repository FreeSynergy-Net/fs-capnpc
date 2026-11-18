# DESIGN - capnp-compiler

Pure-Rust Cap'n Proto schema compiler producing the same `CodeGeneratorRequest` as the official
C++ compiler (capnp 1.4.0), so `capnpc::codegen` yields identical Rust. Rules: ADR 0043 (traits first,
named patterns, files <= 200 lines / hard 300), ADR 0040 (why a Rust compiler).

## Pipeline (each stage behind an interface)

`SourceLoader` (bytes) -> `LexStage` -> `ParseStage` (together `Frontend`) -> resolve/translate
(`compiler`, `translator`, `layout`, `generics`, `value`) -> `Emitter` (`CodeGeneratorRequest`).
`ErrorReporter` receives diagnostics from every stage. `CompileCommand` composes everything.

## Pattern map

| Module | Pattern | Why |
|---|---|---|
| `command` | Facade + Builder | one fluent entry point hiding the pipeline; collaborators injectable |
| `source` (`SourceLoader`, `DiskLoader`, `MemoryLoader`) | Strategy | disk vs in-memory sources; tests and embedding |
| `source::ModuleLoader` | Facade | lookup rules (relative / absolute / std), caching, parsing |
| `source::std_files` | Registry | embedded `/capnp/*.capnp` table |
| `pipeline` (`LexStage`, `ParseStage`, `Frontend`) | Strategy + Facade | swap/compose front-end stages |
| `error` (`ErrorReporter`, `CollectingReporter`) | Strategy | diagnostics sink is injectable |
| `lexer` | Iterator-style scanner | token tree and statements |
| `parser` | Template method over a cursor | declaration grammar by kind |
| `ast` | Value types | grammar.capnp mirror |
| `compiler` | State (stub/expanded/bootstrap/final) + Template method (driver) | lazy node compilation like compiler.c++ |
| `generics` | Value types + Strategy-like brand evaluation | BrandScope / BrandedDecl |
| `translator` | Template method | per-kind node translation |
| `layout` | Strategy-free pure allocator | struct layout, unit-testable on its own |
| `value` | Interpreter | constants to `schema::Value` |
| `request` (`Emitter`, `RequestEmitter`) | Strategy + Adapter | serialise own IR with `capnp::schema_capnp` |
| `hashorder` | Registry-like lookup table emulation | official output order (kj hash tables) |

## Decisions
- Own IR (`schema`) instead of building capnp messages during compilation; serialised once at the end.
- `schema_capnp.rs` is vendored generated code (patched to carry source locations), exempt from size rules.
- `capnpc-embedded` is a dev-dependency only: the oracle for tests.

## State
See `PLAN.md` (State section) for the current status and per-schema oracle table.


## cligen (the generator of capnpc-cli)

One crate, two binaries (`capnpc-cli`), one model. The only dependency is `capnp`.

## Stages

`GenerationCommand` (Facade) -> `Model::from_request` (Adapter, the only place that reads `capnp::schema_capnp`)
-> `Tree::build` (Builder: commands and navigation words) -> emitters (Template). The generated code calls only
`capnp_compiler::cligen::rt`.

## Pattern map

| Module | Pattern | Why |
|---|---|---|
| `lib`, `emit/command` | Facade | one entry point for build scripts and the plugin binary |
| `model/read` | Adapter | the foreign request form is read in one place |
| `model/scope` | Strategy (port) | the Rust paths must be exactly those capnpc writes |
| `model/names` | Strategy | pure naming functions |
| `tree` | Builder | one walk from the root interface |
| `emit/file`, `emit/table`, `emit/runfn` | Template | text per command, per table |
| `emit/setters`, `emit/readers` | Template | one helper per struct, recursion is recursion |
| `emit/ctx` | Visitor with a work list | a helper may need another; written once, sorted by name |
| `emit/listener` | Adapter | typed `Server` on one side, `Sink` on the other |
| `rt` | Strategy | meaning (values, output, hooks) belongs to the program |

## Rules the tests hold

- typed setters only; a given value is written also if it is 0 (the default of a slot is capnpc's job);
- two members of a union given: an error naming the union;
- a listener answers after the sink took the line, and never with an error (`common.capnp`: a stream that failed once
  stays failed);
- nothing is dropped: unreadable types are rows and output values;
- the generated file compiles together with the capnpc output and runs against a fake server (`capnpc-cli-fixtures`);
- the output is byte-equal for equal requests, also with the nodes reordered.

## Not in cligen

Reading values (uuid, time), help, completion, exit codes, the coverage check: the program (fs-cli). The tool
generator for MCP (AP4, open). No private capnp API is used.

## mcpgen (the generator of capnpc-mcp)

`capnp_compiler::mcpgen` reads the same `CodeGeneratorRequest` as `cligen`, uses the command tree of `cligen` (one tool
per command; no second reading of the contract) and writes `<name>_mcp.rs`: the table `TOOLS` (name, title,
description, key, words, `input_schema`, `output_schema`, `meta`, risk, free-content flag; the schemas are JSON text, no
serde), the list `LEFT` (every command that is no tool, with the reason), `tools(&dyn Switch)` and `runner(tool)`, which
finds the function of the command in `<name>_cli.rs` (`dispatch(words)`). There is no protocol and no run time here: the
program that serves MCP (`fs-mcp`, stage 2) reads the table. `mcpgen::rt` holds only data and the switch.

| Contract | Tool |
|---|---|
| a command (method, reached through getters without parameters) | one tool; name `<file>.<words joined by .>` |
| a method whose parameters are a union only | one tool for each member, name ends `.<member>` |
| parameter struct | `inputSchema`: `type: object`, `properties`, `required` = every field that is not in a union, `oneOf` for a union inside, `additionalProperties: false`, `$defs` for named structs |
| result struct | `outputSchema` (open object); the run time answers with `structuredContent` that fits it |
| doc comment | `description`; its first line is the `title` |
| `risk` annotation | `_meta["net.freesynergy/mcp"].risk` and `ToolSpec.risk` (not marked: `unknown` / `None`) |
| `secret`, `risk(excluded)` (method, interface, or a navigation method on the way), `AnyPointer`/generics, capability parameter or result, listener, `-> stream` | no tool; the key and the reason are in `LEFT` |

Property and enum names are the names of the contract (`otherUid`, `dependsOn`); the command code of `cligen` spells
them in kebab case (`other-uid`), so the run time maps with `cligen::model::kebab`. Integers carry their bounds
(`minimum`/`maximum` per width), `Data` is a base64 string, a `kind` hint on a `Text` is a `format` (`uuid`, `date-time`,
`date`, `email`, `uri`), any other hint is `x-fs-kind`. Marks that a program keeps outside the schema (`MarkSource`) count
like annotations; they apply to the parameters of the command itself (path as in `cligen`), not inside `$defs`.

Safety: the generated code is off. `ENABLED_BY_DEFAULT` is `false`, `tools(&Off)` is empty, `Never` (the vault) never
turns on. Every tool says `"actor":"ai"` in its `_meta`; `rt::actor` makes `ai:<client name>` and `rt::result_meta` the
`_meta` of a result. Nothing is a tool unless it is clear that it may be.

### The MCP revision: 2026-07-28 (read on 2026-11-18)

Sources: `modelcontextprotocol.io/specification/2026-07-28/server/tools` and `.../basic/index` (the pages themselves,
nothing copied). What the generator and `rt` rely on:

- Tool: `name` (1-128 characters, `A-Z a-z 0-9 _ - .`, unique), optional `title`, `description`, `inputSchema` (a valid object
  schema, never `null`; without parameters `{"type":"object","additionalProperties":false}` is recommended),
  optional `outputSchema`, `annotations`, `_meta`. Schemas default to JSON Schema 2020-12.
- `outputSchema`: the server MUST give structured results that conform; for compatibility it SHOULD also return the JSON as a
  text block (`content`). `tools/call` results carry `resultType: "complete"` (`rt::RESULT_TYPE`).
- Errors: an unknown tool or a malformed request is the JSON-RPC error `-32602`; an error of the other side is a result with
  `isError: true`.
- This revision is stateless: there is no `initialize` handshake; every request carries `io.modelcontextprotocol/protocolVersion`,
  `.../clientInfo` (optional) and `.../clientCapabilities` in its `_meta`, a missing required field is `-32602`. The name of
  the client is therefore read per request (`rt::CLIENT_INFO_KEY`); the specification calls it self-reported and says it
  must not decide anything, so we use it to label the actor only. (The plan paper still spoke of `initialize`; that is
  older than this revision and not in it.)
- `$ref` must not be dereferenced over the network; we only write local `#/$defs/...` references.
- `tools/list` returns the same set for every connection and in a deterministic order: the table is in walk order.
