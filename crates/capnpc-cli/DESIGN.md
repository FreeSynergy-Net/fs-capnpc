# Design of capnpc-cli

One crate, two binaries (`capnpc-cli`, `capnpc-mcp` as a frame), one model. The only dependency is `capnp`.

## Stages

`GenerationCommand` (Facade) -> `Model::from_request` (Adapter, the only place that reads `capnp::schema_capnp`)
-> `Tree::build` (Builder: commands and navigation words) -> emitters (Template). The generated code calls only
`capnpc_cli::rt`.

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

## Not in this crate

Reading values (uuid, time), help, completion, exit codes, the coverage check: the program (fs-cli). The tool
generator for MCP (AP4). No private capnp API is used.
