//! The fake server of the tasks contract: every method notes its key; `editIf` also notes the values it got.
#![allow(dead_code)]

use std::cell::RefCell;
use std::future::{Future, ready};
use std::rc::Rc;

use capnpc_cli_fixtures::tasks_capnp::{boards, items, tasks};
use capnpc_cli_fixtures::types_capnp::type_catalog;

pub type Log = Rc<RefCell<Vec<String>>>;
type Done = std::future::Ready<Result<(), capnp::Error>>;

pub struct FakeItems(pub Log);
pub struct FakeTasks(pub Log);
struct FakeBoards;
pub struct FakeTypes(pub Log);

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
