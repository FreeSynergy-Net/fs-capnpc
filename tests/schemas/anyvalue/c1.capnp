@0xdbb9ad1f14bf0e01;
struct P { a @0 :Int32; b @1 :Text; }
const p :P = (a = 5, b = "x");
const q :AnyPointer = .p;
