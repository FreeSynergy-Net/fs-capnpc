@0xdbb9ad1f14bf0e03;
struct P { a @0 :Int32; b @1 :Text; }
const p :P = (a = 5, b = "x");
struct S { f @0 :AnyPointer; }
const s :S = (f = .p);
annotation tag(struct) :S;
struct T $tag(f = .p) { a @0 :Int32; }
