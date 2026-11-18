//! T6: every generated command of the real tasks contract calls exactly its own method (interface and ordinal) of a
//! fake server, and the values arrive.

mod support;

use std::cell::RefCell;
use std::future::{Future, ready};
use std::rc::Rc;

use capnpc_cli_fixtures::tasks_capnp::{boards, items, tasks};
use capnpc_cli_fixtures::tasks_cli as cli;
use capnpc_cli_fixtures::types_capnp::type_catalog;
use support::{In, Rec, run};

type Log = Rc<RefCell<Vec<String>>>;
type Done = std::future::Ready<Result<(), capnp::Error>>;

struct FakeItems(Log);
struct FakeTasks(Log);
struct FakeBoards;
struct FakeTypes(Log);

fn note(log: &Log, key: &str) -> Done {
    log.borrow_mut().push(key.to_owned());
    ready(Ok(()))
}

/// One method that only notes its key.
macro_rules! method {
    ($name:ident, $key:expr, $p:ty, $r:ty) => {
        fn $name(
            self: capnp::capability::Rc<Self>,
            _: $p,
            _: $r,
        ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
            note(&self.0, $key)
        }
    };
}

impl items::Server for FakeItems {
    method!(
        create,
        "tasks/Items.create",
        items::CreateParams,
        items::CreateResults
    );
    method!(
        edit,
        "tasks/Items.edit",
        items::EditParams,
        items::EditResults
    );
    method!(
        assign,
        "tasks/Items.assign",
        items::AssignParams,
        items::AssignResults
    );
    method!(
        create_relation,
        "tasks/Items.createRelation",
        items::CreateRelationParams,
        items::CreateRelationResults
    );
    method!(
        assign_label,
        "tasks/Items.assignLabel",
        items::AssignLabelParams,
        items::AssignLabelResults
    );
    method!(get, "tasks/Items.get", items::GetParams, items::GetResults);
    method!(
        get_by_uid,
        "tasks/Items.getByUid",
        items::GetByUidParams,
        items::GetByUidResults
    );
    method!(
        delete,
        "tasks/Items.delete",
        items::DeleteParams,
        items::DeleteResults
    );
    method!(
        list,
        "tasks/Items.list",
        items::ListParams,
        items::ListResults
    );
    method!(
        query,
        "tasks/Items.query",
        items::QueryParams,
        items::QueryResults
    );
    method!(
        import_ics,
        "tasks/Items.importIcs",
        items::ImportIcsParams,
        items::ImportIcsResults
    );
    method!(
        export_ics,
        "tasks/Items.exportIcs",
        items::ExportIcsParams,
        items::ExportIcsResults
    );
    method!(
        readiness,
        "tasks/Items.readiness",
        items::ReadinessParams,
        items::ReadinessResults
    );
    method!(
        blocked_by,
        "tasks/Items.blockedBy",
        items::BlockedByParams,
        items::BlockedByResults
    );
    method!(
        dependents,
        "tasks/Items.dependents",
        items::DependentsParams,
        items::DependentsResults
    );
    method!(
        create_relation_with_gap,
        "tasks/Items.createRelationWithGap",
        items::CreateRelationWithGapParams,
        items::CreateRelationWithGapResults
    );

    fn edit_if(
        self: capnp::capability::Rc<Self>,
        params: items::EditIfParams,
        _: items::EditIfResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        let r = (|| {
            let p = params.get()?;
            let id = p.get_id()?;
            self.0.borrow_mut().push("tasks/Items.editIf".to_owned());
            self.0.borrow_mut().push(format!(
                "detail: id={}:{} revision={} jscalendar={}",
                id.get_hi(),
                id.get_lo(),
                p.get_revision(),
                p.get_jscalendar()?.to_str()?
            ));
            Ok(())
        })();
        ready(r)
    }
}

impl boards::Server for FakeBoards {}

impl type_catalog::Server for FakeTypes {
    method!(
        describe,
        "types/TypeCatalog.describe",
        type_catalog::DescribeParams,
        type_catalog::DescribeResults
    );
}

impl tasks::Server for FakeTasks {
    method!(
        capabilities,
        "tasks/Tasks.capabilities",
        tasks::CapabilitiesParams,
        tasks::CapabilitiesResults
    );
    method!(
        subscribe,
        "tasks/Tasks.subscribe",
        tasks::SubscribeParams,
        tasks::SubscribeResults
    );
    method!(
        support_report,
        "tasks/Tasks.supportReport",
        tasks::SupportReportParams,
        tasks::SupportReportResults
    );

    fn items(
        self: capnp::capability::Rc<Self>,
        _: tasks::ItemsParams,
        mut results: tasks::ItemsResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        results
            .get()
            .set_items(capnp_rpc::new_client(FakeItems(self.0.clone())));
        ready(Ok(()))
    }

    fn boards(
        self: capnp::capability::Rc<Self>,
        _: tasks::BoardsParams,
        mut results: tasks::BoardsResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        results.get().set_boards(capnp_rpc::new_client(FakeBoards));
        ready(Ok(()))
    }

    fn types(
        self: capnp::capability::Rc<Self>,
        _: tasks::TypesParams,
        mut results: tasks::TypesResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        results
            .get()
            .set_catalog(capnp_rpc::new_client(FakeTypes(self.0.clone())));
        ready(Ok(()))
    }
}

fn call(words: &[&str], pairs: &[(&str, &str)]) -> Vec<String> {
    let log: Log = Rc::default();
    let root: tasks::Client = capnp_rpc::new_client(FakeTasks(log.clone()));
    let mut out = Rec {
        sink: Some(Rc::default()),
        ..Rec::default()
    };
    let go = cli::dispatch(words).unwrap_or_else(|| panic!("no command {words:?}"));
    run(go(&root, &mut In::of(pairs), &mut out)).unwrap_or_else(|e| panic!("{words:?}: {e}"));
    std::mem::take(&mut *log.borrow_mut())
}

#[test]
fn t6_every_command_calls_exactly_its_own_method() {
    // M2 / O1: a wrong ordinal reaches another method of the fake, and the log shows it
    let mut seen = Vec::new();
    for c in cli::COMMANDS {
        let calls: Vec<String> = call(c.words, &[])
            .into_iter()
            .filter(|l| !l.starts_with("detail:"))
            .collect();
        assert_eq!(calls, [c.key], "{:?}", c.words);
        seen.push(c.key);
    }
    assert!(seen.len() >= 21, "{}", seen.len());
    let mut sorted = seen.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        seen.len(),
        "no two commands reach the same method"
    );
}

#[test]
fn t6_the_values_arrive() {
    let calls = call(
        &["items", "edit-if"],
        &[
            ("id.hi", "1"),
            ("id.lo", "2"),
            ("revision", "7"),
            ("jscalendar", "x"),
        ],
    );
    assert_eq!(
        calls,
        [
            "tasks/Items.editIf",
            "detail: id=1:2 revision=7 jscalendar=x"
        ]
    );
}

#[test]
fn t6_the_table_rows_reach_their_methods_by_interface_and_ordinal() {
    // O1 / M2: a row with a wrong ordinal or interface id: the call by (id, ordinal) lands on another method
    let mut n = 0;
    for c in cli::COMMANDS {
        let log: Log = Rc::default();
        let root: tasks::Client = capnp_rpc::new_client(FakeTasks(log.clone()));
        let client = if c.key.starts_with("tasks/Tasks.") {
            root.client.clone()
        } else if c.key.starts_with("tasks/Items.") {
            root.items_request().send().pipeline.get_items().client
        } else {
            root.types_request().send().pipeline.get_catalog().client
        };
        let req = client.new_call::<capnp::any_pointer::Owned, capnp::any_pointer::Owned>(
            c.iface_id, c.ordinal, None,
        );
        run(req.send().promise).unwrap_or_else(|e| panic!("{}: {e}", c.key));
        let calls: Vec<String> = log
            .borrow()
            .iter()
            .filter(|l| !l.starts_with("detail:"))
            .cloned()
            .collect();
        assert_eq!(
            calls,
            [c.key],
            "row {} ({:#x}, {})",
            c.key,
            c.iface_id,
            c.ordinal
        );
        n += 1;
    }
    assert_eq!(n, cli::COMMANDS.len());
}
