# Changelog

## Unreleased

- New workspace crate `capnpc-cli` (with the test crate `capnpc-cli-fixtures`): a Cap'n Proto generator plugin that
  writes command line code (command tables and typed call functions) for an interface; `capnp-compiler` is unchanged.
  The AI collaboration is stated in `NOTICE` and the READMEs.
  The generated code states its `rt` ABI (1) and fails to compile against another one. A top-level method named like
  a word the program owns (`serve`, `mcp`, `help`, `version`) keeps its contract name; its command word is
  `method-<word>`, the table says `renamed: true` (also below it); two methods with the same words are an error.

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
