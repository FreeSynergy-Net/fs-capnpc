# The navigation word `serve` becomes `method-serve`; so does the command `methodServe`: a group and a command with
# the same words are an error too.
@0xc1d2e3f405162737;
interface Collide2 { serve @0 () -> (sub :Sub2); methodServe @1 () -> (); }
interface Sub2 { x @0 () -> (); }
