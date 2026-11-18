# Fixture of the capnpc-mcp tests: the cases of the tool table that the other fixtures do not have (a method that takes a
# union only, a `kind` hint, risk marks, an excluded interface, a capability result).
@0x9a1b2c3d4e5f6071;

using Cli = import "/capnpc_cli.capnp";

# A note somebody wrote.
struct Note {
  # Identifies the note.
  id   @0 :Text $Cli.kind("uuid");
  body @1 :Text;
  at   @2 :Int64 $Cli.kind("timestamp");
}

# Which note: by id, by name, or all of them (a union only: one tool for each member).
struct Pick {
  union {
    # The id of the note.
    byId       @0 :Text;
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

# The notes of a person.
interface Notes {
  # Find notes.
  find   @0 Pick -> (notes :List(Note));
  # Add a note.
  add    @1 (text :Text, tags :List(Text), mood :Mood, at :Int64 $Cli.kind("timestamp")) -> (note :Note) $Cli.risk(add);
  wipe   @2 () -> () $Cli.risk(destructive);
  vault  @3 () -> (vault :Vault);
  count  @4 () -> (count :UInt32);
  login  @5 (user :Text, token :Text $Cli.secret) -> ();
  raw    @6 (value :AnyPointer) -> ();
  other  @7 (name :Text) -> (sub :Sub);
  hidden @8 () -> () $Cli.risk(excluded);
}
