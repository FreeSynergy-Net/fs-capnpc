@0xdbb9ad1f14bf0e02;
struct P { a @0 :Int32; b @1 :Text; }
const p :P = (a = 5, b = "x");
annotation note(*) :AnyPointer;
struct S $note(.p) { a @0 :Int32; }
