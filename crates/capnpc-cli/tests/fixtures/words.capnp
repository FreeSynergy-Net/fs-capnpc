# Reserved words: on the top level they get the prefix (also as navigation, the commands below are marked too),
# below the top level `version` is an ordinary word.
@0xc1d2e3f405162739;
interface Words {
  help  @0 () -> ();
  serve @1 () -> (sub :Sub);
  mcp   @2 () -> ();
  nav   @3 () -> (other :Sub);
}
interface Sub { version @0 () -> (); }
