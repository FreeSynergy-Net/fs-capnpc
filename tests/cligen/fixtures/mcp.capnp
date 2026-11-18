# Fixture of the capnpc-mcp tests: the cases of the tool table that the other fixtures do not have (a method that takes a
# union only, a `kind` hint, risk marks, an excluded interface, a capability result). Doc comments follow the
# declaration, as capnp reads them.
@0x9a1b2c3d4e5f6071;

using Cli = import "/capnpc_cli.capnp";

struct Note {
  # A note somebody wrote.
  id   @0 :Text $Cli.kind("uuid");   # Identifies the note.
  body @1 :Text;
  at   @2 :Int64 $Cli.kind("timestamp");
}

struct Pick {
  # Which note: by id, by name, or all of them (a union only: one tool for each member).
  union {
    byId       @0 :Text;   # The id of the note.
    byName     @1 :Text;
    everything @2 :Void;
  }
}

enum Mood { calm @0; veryLoud @1; }

interface Vault $Cli.risk(excluded) {
  open @0 () -> (data :Data);
}

interface Sub {
  ping @0 () -> ();
}

interface Peek $Cli.risk(read) {
  # No mark of its own: the mark of the interface counts.
  peek @0 () -> (n :UInt32);
}

interface Inner {
  deep @0 () -> ();
}

interface Child {
  leaf @0 () -> ();
}

interface Outer $Cli.risk(excluded) {
  child @0 () -> (child :Child);
}

interface Notes {
  # The notes of a person.
  find   @0 Pick -> (notes :List(Note));   # Find notes.
  add    @1 (text :Text, tags :List(Text), mood :Mood, at :Int64 $Cli.kind("timestamp")) -> (note :Note) $Cli.risk(add);   # Add a note.
  wipe   @2 () -> () $Cli.risk(destructive);
  vault  @3 () -> (vault :Vault);
  count  @4 () -> (count :UInt32);
  login  @5 (user :Text, token :Text $Cli.secret) -> ();
  raw    @6 (value :AnyPointer) -> ();
  other  @7 (name :Text) -> (sub :Sub);
  hidden @8 () -> () $Cli.risk(excluded);
  peeker @9 () -> (peek :Peek);
  inner  @10 () -> (inner :Inner) $Cli.risk(excluded);   # a way to a tool that is excluded: nothing below it is a tool
  outer  @11 () -> (outer :Outer);
  voids  @12 (nothing :Void, n :UInt8) -> (done :Void, ok :Bool);
  ratio  @13 () -> (value :Float64, label :Text);   # a float and a text in the result, nothing free in the arguments
  token  @14 () -> (token :Text $Cli.secret);   # a secret in a result: no tool
}
