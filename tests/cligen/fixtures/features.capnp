# Feature fixture of the capnpc-cli tests: one root interface that has every case of the generator.
@0xa1b2c3d4e5f60718;

using Cli = import "/capnpc_cli.capnp";

enum Colour { red @0; deepBlue @1; green @2; }

struct Point { x @0 :Int32; y @1 :Int32; }

struct Line {
  width   @0 :UInt16 = 1;        # default is not 0
  opacity @1 :Float32 = 0.5;
  name    @2 :Text = "line";
  colour  @3 :Colour = deepBlue;
  from :group { x @4 :Int32; y @5 :Int32; }
  to   :group { x @6 :Int32; y @7 :Int32; }
}

struct Shape {
  label @0 :Text;
  union {
    circle  @1 :UInt32;
    box     @2 :Point;
    nothing @3 :Void;
    names   @4 :List(Text);
  }
}

struct Page { shapes @0 :List(Shape); next @1 :Data; }

struct Node { name @0 :Text; children @1 :List(Node); }   # contains itself

interface Sink {
  event @0 (shape :Shape) -> stream;
  done  @1 (why :Text) -> ();
}

interface Details {
  rename @0 (name :Text) -> ();
}

interface Features {
  details    @0 () -> (details :Details);
  draw       @1 (line :Line) -> ();
  place      @2 (shape :Shape) -> (shape :Shape);
  lists      @3 (ints :List(UInt8), names :List(Text), points :List(Point), grid :List(List(Int32)), colours :List(Colour)) -> (count :UInt32);
  blob       @4 (data :Data, first :Bool) -> (data :Data);
  tree       @5 (root :Node) -> (root :Node);
  watch      @6 (sink :Sink) -> (subscription :Details);
  login      @7 (user :Text, password :Text $Cli.secret) -> ();
  rawValue   @8 (value :AnyPointer) -> ();
  danger     @9 () -> () $Cli.risk(destructive);
  page       @10 () -> (page :Page);
  reservedOk @11 () -> (version :UInt8, kinds :Colour);
  watchTwo   @13 (first :Sink, second :Sink) -> (subscription :Details);   # two listeners: the counters add up
  createAt   @12 (when :Int64, big :UInt64, small :Int8, f :Float64) -> ();
}
