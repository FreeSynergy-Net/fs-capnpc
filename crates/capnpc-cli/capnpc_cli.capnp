# Annotations of capnpc-cli / capnpc-mcp. Import with `using Cli = import "/capnpc_cli.capnp";`
# (add the folder of this file as an import path). The ids are fixed: generated code and tools find the marks by id.

@0xee70a4bcdcfac54d;

# A value that must never travel in a command line argument or a tool description (a password, a token).
annotation secret @0xefac3d5046acfeb0 (field, param) :Void;

# What a call does to the data of people; the tools of a program use it to decide what is offered.
enum Danger {
  read        @0;
  add         @1;
  change      @2;
  destructive @3;
  actsInName  @4;
  excluded    @5;
}
annotation risk @0xf0a9fc9d3d3d3fb5 (method, interface) :Danger;

# The type or method names a resource of a person (read: this data belongs to somebody).
annotation resource @0xcfcd09841994a63d (struct, interface, method) :Void;

# Why a mark was set (shown to the user, never used by the generator).
annotation reason @0x841e7906fa0703c5 (method, interface, field, param, struct) :Text;

# A hint for the run time which basic type a Text or struct field carries ("uuid", "timestamp", "email").
annotation kind @0xe63fc3253809b912 (field, param, struct) :Text;
