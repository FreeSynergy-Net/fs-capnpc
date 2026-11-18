//! Test crate of capnpc-cli: the generated types and commands of the fixture schemas, compiled together.
//!
//! Pattern: Facade — only modules that include what `build.rs` wrote; the tests in `tests/` run the commands.
#![allow(clippy::all, clippy::pedantic, unused, missing_docs)]

macro_rules! generated {
    ($($name:ident),*) => {$(
        pub mod $name { include!(concat!(env!("OUT_DIR"), "/", stringify!($name), ".rs")); }
    )*};
}

generated!(
    tasks_capnp,
    common_capnp,
    types_capnp,
    features_capnp,
    capnpc_cli_capnp,
    tasks_cli,
    features_cli
);

/// The interface ids capnpc wrote (`pub(crate)` there), for the tests that check the tables against them.
pub mod ids {
    /// `Features`.
    pub const FEATURES: u64 = crate::features_capnp::features::_private::TYPE_ID;
    /// `Details`.
    pub const DETAILS: u64 = crate::features_capnp::details::_private::TYPE_ID;
    /// `Items` of tasks.
    pub const ITEMS: u64 = crate::tasks_capnp::items::_private::TYPE_ID;
}
