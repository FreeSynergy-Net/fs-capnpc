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
