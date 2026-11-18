# Changelog

## Unreleased

- New workspace crate `capnpc-cli` (with the test crate `capnpc-cli-fixtures`): a Cap'n Proto generator plugin that
  writes command line code (command tables and typed call functions) for an interface; `capnp-compiler` is unchanged.
  The AI collaboration is stated in `NOTICE` and the READMEs.
  The generated code states its `rt` ABI (3) and fails to compile against another one. `run_*` return an `Outcome`;
  the generated listener counts the calls it cannot read itself, before the sink sees the line (a sink that swallows
  errors, or never calls `render`, cannot hide them: a call that fails only while it is rendered is counted too), and the
  generated `run_to_exit` makes the exit code (0 ok, 1 the other side answered with an error, 2 a wrong word or value,
  3 not reachable, the connection was lost in the middle of a stream, or a call of a watch could not be read) after the
  program has waited for the watch. `wait` returns a `Result`: `Ok` when the user stopped the watch, an error (a lost
  connection is `Disconnected`, so 3) when the stream broke. A top-level method named like
  a word the program owns (`serve`, `mcp`, `help`, `version`) keeps its contract name; its command word is
  `method-<word>`, the table says `renamed: true` (also below it); two methods with the same words are an error.
- Windows gate: new workflow `windows` (windows-2022: fmt, pedantic clippy, tests of the whole workspace) and a
  `.gitattributes` that checks the golden files out with LF (they are compared byte for byte). The `ci` workflow runs on
  `main` and on pull requests only (the shared pattern of the repositories), so a branch head without a pull request has
  only the `check` run. New test `wait_error`: a `wait` that ends with another error than a lost connection (failed,
  unimplemented, overloaded) is exit code 1, a lost connection is 3, a clean wait is 0.
  (Deutsch: Windows-Gate als eigener Workflow `windows`, `.gitattributes` mit LF fuer die Golden-Dateien; `ci` laeuft nur
  auf `main` und Pull Requests; neuer Test `wait_error`: ein anderer Fehler von `wait` als ein Verbindungsverlust ist
  Exit 1, Verbindungsverlust 3, sauberes Ende 0.)

## 0.1.1 — 2026-11-06

- Package metadata: `rust-version = 1.88`, `authors`, security contact; release packages via fs-release (Linux, Windows, macOS), SPDX SBOM, OpenPGP signatures.

## 0.1.0 (unreleased)

- Pure Rust lexer, parser and compiler for the full Cap'n Proto schema language: structs, unions,
  groups, enums, interfaces (generic methods, streaming), constants, annotations, generics with
  brands, aliases (`using`), imports, `embed`.
- Struct layout allocator and type-id generation identical to capnp 1.4.0.
- `CodeGeneratorRequest` output content-identical to the official compiler, including source info
  (ordering reproduces the libc++ hash table of the reference build).
- Library API: `CompileCommand` builder; traits `SourceLoader`, `Frontend`, `ErrorReporter`, `Emitter`.
- CLI: `capnp-compiler compile -o <dir> files...`.
