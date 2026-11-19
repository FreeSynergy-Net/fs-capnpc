# Sensitive data (a person's health data): the marks `$Cli.sensitive` on a result field, on an argument, and none.
@0xb7a1c3d5e7f90214;
using Cli = import "/capnpc_cli.capnp";

struct Finding { diagnosis @0 :Text $Cli.sensitive; date @1 :Text; }

interface Records {
  # The records of a person.
  diagnose @0 () -> (finding :Finding, note :Text $Cli.sensitive) $Cli.risk(read);   # Read the finding.
  share    @1 (with :Text, detail :Text $Cli.sensitive) -> () $Cli.risk(add);         # Share a detail.
  plain    @2 (name :Text) -> (count :UInt32, name :Text) $Cli.risk(read);            # Nothing marked.
  secret   @3 () -> (key :Text $Cli.secret, hint :Text $Cli.sensitive) $Cli.risk(read);   # A secret wins: no tool.
}
