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
See PLAN.md in the repository on GitHub (State section; not in the package) for the current status and per-schema oracle table.


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
generator for MCP: `mcpgen` (below). No private capnp API is used.

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
| result struct | `outputSchema` (lax object, see below); the run time answers with `structuredContent` that fits it |
| doc comment | `description`; its first line is the `title` |
| `risk` annotation | `_meta["net.freesynergy/mcp"].risk` and `ToolSpec.risk` (not marked: `unknown` / `None`) |
| `secret`, `risk(excluded)` (method, interface, or a navigation method on the way), `AnyPointer`/generics, capability parameter or result, listener, `-> stream` | no tool; the key and the reason are in `LEFT` |

The encoding is the capnp JSON encoding (the codec of Cap'n Proto), so what a tool takes and gives is what that codec reads
and writes: property and enum names are the names of the contract (`otherUid`, `dependsOn`; the command code of `cligen`
spells them in kebab case, so the run time maps with `cligen::model::kebab`). Integers up to 32 bits carry their bounds
(`minimum`/`maximum`); a 64-bit integer is a string (`pattern`, the width and bounds in `x-fs-int`), as the codec writes
it and because a number loses digits above 2^53. A float is `anyOf` a number and the strings `Infinity`, `-Infinity`,
`NaN`. `Data` is a base64 string (`contentEncoding`): the codec's own default is an array of bytes, we use its `$Json.base64`
rule for every `Data`, and the encoder of the run time (M4) does the same. A `kind` hint on a `Text` is a `format` (`uuid`,
`date-time`, `date`, `email`, `uri`), any other hint is `x-fs-kind`. Marks that a program keeps outside the schema
(`MarkSource`) count like annotations; they apply to the parameters of the command itself (path as in `cligen`), not inside
`$defs`, and not to result fields (see below).

An argument schema is strict: closed, every field that is not in a union required, `oneOf` for a union. A result schema is
lax: the codec leaves out a null pointer and the default member of a union, so only what it always writes (a number, a
boolean, an enum) is `required`, and there is no `oneOf`; otherwise a correct `structuredContent` would break the rule
that it must fit the `outputSchema`.

`secret` on a result field: the annotation counts (no tool). A program mark (`MarkSource::secret`) is defined for the
parameter path of a command and is not read for results; the answer to that is in M3, where the screen of `fs-mcp` (secret
words, free text, `AnyPointer`) must be applied as a mandatory intersection with the generated table (`tools/list` = table
and catalog); a result field that the program marks as secret outside the schema is then taken out by that screen. We chose
this over a second meaning of the same method, because `cligen` and `mcpgen` share the trait and the path form of a result
is not defined there.

The label of an AI: `rt::actor` makes an `AiActor` (`ai:<name>`, only `A-Za-z0-9._- ` of the name, every other character
`_`, at most 64), and `rt::result_meta` takes only that type, so a result cannot carry a label that does not start with `ai:`.
The table `TOOLS` is `#[doc(hidden)]`: a program reads `tools(&switch)`, which is empty while the switch is off.

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
- Errors: an unknown tool or a malformed request is the JSON-RPC error `-32602`; arguments that break the `inputSchema`
  and an error of the other side are a result with `isError: true` (input validation is a tool execution error, not a
  protocol error). A request for another protocol version is `-32022` (`UnsupportedProtocolVersion`) with `data.supported`
  listing `2026-07-28`; this is for the run time (M4), the generator only fixes the date in `rt::PROTOCOL_VERSION`.
- This revision is stateless: there is no `initialize` handshake; every request carries `io.modelcontextprotocol/protocolVersion`,
  `.../clientInfo` (optional) and `.../clientCapabilities` in its `_meta`, a missing required field is `-32602`. The name of
  the client is therefore read per request (`rt::CLIENT_INFO_KEY`); the specification calls it self-reported and says it
  must not decide anything, so we use it to label the actor only. (The plan paper still spoke of `initialize`; that is
  older than this revision and not in it.)
- `$ref` must not be dereferenced over the network; we only write local `#/$defs/...` references.
- `tools/list` returns the same set for every connection and in a deterministic order: the table is in walk order.

## The annotation `sensitive` (unreleased)

Two stages of what must not travel (decision of the strategy, 19.11.2026): a SECRET (credentials, keys, tokens: `$Cli.secret`) never leaves,
not even with a release, so a tool with one is no tool; SENSITIVE data (`$Cli.sensitive`: health data, marked contacts, data protection
categories) leaves only for a receiver with an explicit release, so the tool stays a tool and the table says where the data is
(`x-fs-sensitive` in the schema, `sensitive` in `_meta`). The generator only says; WHO may receive is the decision of the program that serves
the tools, from the key of the connection and never from a label that the client writes about itself. `SCHEMA_DIR` is how a contract finds
`capnpc_cli.capnp` without a copy.
