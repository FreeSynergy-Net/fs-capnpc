# Three interfaces that nothing leads to: three roots in one file, so one module for each.
@0xd4e5f60718293a4b;
interface Gamma { third @0 () -> (); }
interface Alpha { first @0 () -> (); }
interface Beta { second @0 (n :UInt8) -> (); }
