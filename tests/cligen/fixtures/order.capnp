# Members written in another order than their ordinals: the comment belongs to the member it stands behind.
@0xa1b2c3d4e5f60718;
struct Odd {
  second @1 :Text;   # the second one
  first  @0 :Text;   # the first one
  third  @2 :Text;   # the third one
}
interface Out {
  later  @1 () -> ();   # runs later
  sooner @0 () -> ();   # runs sooner
}
