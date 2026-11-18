//! A1: the command code generated with `crates_provide_map` compiles (it names `::fs_types::types_capnp`, resolved to
//! `types_capnp` of this crate) and is the same table as the one without.

use capnpc_cli_fixtures::{tasks_cli, tasks_cli_provided};

#[test]
fn the_provided_variant_has_the_same_commands() {
    assert_eq!(
        tasks_cli_provided::COMMANDS.len(),
        tasks_cli::COMMANDS.len()
    );
    for (a, b) in tasks_cli_provided::COMMANDS.iter().zip(tasks_cli::COMMANDS) {
        assert_eq!(a.key, b.key);
    }
}
