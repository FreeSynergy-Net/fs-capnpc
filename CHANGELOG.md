# Changelog

## 0.2.0 - unreleased

- Added: the command line code generator `capnp_compiler::cligen` (moved here from the workspace crate `capnpc-cli`,
  history kept, code unchanged apart from two things in the generated text: the runtime path is now
  `capnp_compiler::cligen::rt`, and the head line names `capnpc-cli 0.2.0`), the binary `capnpc-cli` (plugin for
  `capnp compile -ocli:<dir>`) and the option `capnp-compiler compile --cli <dir>` (same generator, in one process,
  also together with `-o`). The runtime part `cligen::rt` needs no feature.
  (Deutsch: neu: Generator `cligen` fuer Befehlszeilen-Code aus einer Schnittstelle, Binary `capnpc-cli` als capnp-Plugin
  und Option `compile --cli <dir>`; der Code ist aus dem Crate `capnpc-cli` umgezogen, Historie bleibt.)
- Added (A1): `GenerationCommand::crates_provide_map(HashMap<u64, String>)`, the same map that is given to `capnpc`
  (file id to crate name): the types of such a file are named `::<crate>::<stem>_capnp::...` instead of
  `crate::<stem>_capnp::...`, so the command code of a contract that imports `/types.capnp` from `fs-types` compiles
  (`fs-interfaces` passes `fs_types`). Tests: the text equals the plain one with only that path replaced, a golden file,
  and the fixtures crate compiles the provided variant. The option `capnp_module` of the old plan is not needed.
  (Deutsch: A1 erledigt: `crates_provide_map` wie bei `capnpc`; Typen einer Datei aus einem anderen Crate heissen
  `::<crate>::<stem>_capnp::...`.)
- Changed (breaking for `default-features = false`): the feature `cli` (default) holds the command lines and the
  dependency `capnpc`; `capnp-compiler = { default-features = false }` is the library without `capnpc`, and has no
  module `cli` any more. Hence 0.2.0.
  (Deutsch: Feature `cli` (Standard) enthaelt die Befehlszeilen und `capnpc`; ohne Standard-Features keine Befehlszeile
  mehr: Bruch, deshalb 0.2.0.)
- Removed: the workspace crate `capnpc-cli` (never published) and its stub binary `capnpc-mcp` (it only exited with 1).
  The test crate is now `crates/cligen-fixtures`. Open: the MCP tool generator (AP4) and the option `capnp_module` (A1).
  (Deutsch: das Crate `capnpc-cli` und der Stummel `capnpc-mcp` fallen weg; AP4 und A1 sind offen.)
- CI: a workflow `slim-lib` (runs on every push: no `capnpc` in the tree without default features, the fixtures crate on the slim library, both binaries in the package), the Windows
  workflow also builds the slim library; `release.yml` packages both binaries.

- Added: the tool table generator `capnp_compiler::mcpgen` (`<name>_mcp.rs`: `TOOLS` with JSON Schemas as text, `LEFT`, switch off by
  default, AI label in every tool; MCP revision 2026-07-28). Written for `fs-mcp`; no protocol and no run time in it.
  Also the binary `capnpc-mcp` (plugin for `capnp compile -omcp:<dir>`, feature `cli` like `capnpc-cli`) and the option
  `capnp-compiler compile --mcp <dir>`; `MarkSource` has the new methods `risk`, `excluded` and `tool_name` (default: none).
  (Deutsch: neu: Generator `mcpgen` fuer Tool-Tabellen aus einer Schnittstelle, standardmaessig aus, KI-Kennzeichnung je Tool;
  Binary `capnpc-mcp` und Option `compile --mcp <dir>`.)

### Earlier entries of this unreleased section (before the move)

- Workspace crate `capnpc-cli` (now the module `cligen`, see above; with the test crate `capnpc-cli-fixtures`, now `cligen-fixtures`): a Cap'n Proto generator plugin that
  writes command line code (command tables and typed call functions) for an interface.
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
- Windows path fix (found by the new `windows` gate): the source lookup no longer uses `std::path` to take paths apart.
  Cause: `abs_components` dropped the drive and put a root without drive (`/w/a.capnp`, not absolute on Windows) below
  the current directory, and the in-memory loader looked files up by the host's `\`-joined path. A file on another
  drive than the current directory ("hello.capnp: no such file") and every in-memory file were not found. Now the
  new `AbsPath` parses path text the same way on every host (a `windows` flag selects the rules): root (`/`, `C:/`,
  `//server/share/`), components, `.` and `..`; the drive is kept, Windows names compare without regard to ASCII
  case. `MemoryLoader` normalises the names it stores and looks up (`/` and `\` separate, `.`/`..` resolved). New tests
  run the Windows rules on Linux (drives, UNC, `\`, `..`, imports below a `\` directory).
  (Deutsch: Windows-Pfadfehler behoben: neuer `AbsPath` zerlegt Pfadtext auf jedem Rechner gleich (Laufwerk bleibt,
  `\` und `/` trennen, `.`/`..` aufgeloest); `MemoryLoader` normalisiert Namen; Tests pruefen die Windows-Regeln auch
  unter Linux.)
- Windows path edge cases (review of the path fix): `\\?\UNC\srv\share\x` (what `canonicalize` returns on Windows) is the
  UNC path `//srv/share/x`, no longer a relative path below the current directory; a drive-relative `C:foo` is no longer
  turned into `C:\foo`: it is relative to the current directory of that drive when the base is on that drive, otherwise
  it stays drive-relative (root `C:`, the current directory of another drive is unknown and not guessed); a server
  without a share (`//srv`) is the root `//srv/`, not `//srv//`. New tests (run the Windows rules on Linux, too): a Unix
  host reads the source file with the Unix rules (`\` is a name character, `C:` no drive), a file that is an ancestor
  of the current directory does not panic, UNC with `/`, and each edge case above.
  (Deutsch: Windows-Randfaelle: `\\?\UNC\...` ist ein UNC-Pfad, `C:foo` bleibt laufwerk-relativ statt `C:\foo`, `//srv`
  ohne Freigabe ist die Wurzel `//srv/`; neue Tests, u. a. Unix-Regeln auf Linux und Datei als Vorfahr des
  Arbeitsverzeichnisses ohne Absturz.)

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
