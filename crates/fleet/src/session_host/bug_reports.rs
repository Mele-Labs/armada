//! `report_bug`: a job says Armada itself got something wrong. The report is kept in the store on
//! this Mac and posted nowhere; Bridge lists, keeps and drops them, and is told by
//! `bug_reports.changed`.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{BugReport, BugReportDropped, BugReported, BugReports, Instant, ReportBug, WireError};
use store::KeptBugReport;

use crate::daemon::Fleet;

const BUG_REPORT_REFUSED: &str = "fleet.bug_report_refused";
const NO_SUCH_BUG_REPORT: &str = "fleet.no_such_bug_report";

fn wire(kept: KeptBugReport) -> BugReport {
    BugReport {
        id: kept.id,
        session_id: kept.session_id,
        job_title: kept.job_title,
        title: kept.title,
        what: kept.what,
        steps: kept.steps,
        errors: kept.errors,
        logs: kept.logs,
        at: Instant::carried(kept.at),
        kept: kept.kept.map(Instant::carried),
    }
}

/// The lines a report sent, each trimmed, with the blank ones left out.
fn lines(sent: Vec<String>) -> Vec<String> {
    sent.into_iter().map(|line| line.trim().to_string()).filter(|line| !line.is_empty()).collect()
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Keep a fault in Armada a job reported. Refused for a call that places no job, and for an
    /// empty `title` or `what`. A job that has ended is served, and no notice is sent to it.
    pub(crate) async fn bug_reported(
        &self,
        caller: Option<api::Caller>,
        report: ReportBug,
    ) -> Result<BugReported, Refusal> {
        let session = self.waiting_session(caller, report.session_id.as_ref())?;
        let title = report.title.trim();
        let what = report.what.trim();
        if title.is_empty() || what.is_empty() {
            return Err(self.hosted_refusal(
                BUG_REPORT_REFUSED,
                "a bug report needs a `title`, one line naming the fault, and `what` went wrong",
            ));
        }
        let report_id = self.mint().ulid().as_str().to_lowercase();
        let reports = {
            let mut store = self.store().lock().await;
            let job_title = store
                .session(&session)
                .map_err(|why| self.ledger_fault(why))?
                .and_then(|kept| kept.title)
                .unwrap_or_default();
            let kept = KeptBugReport {
                id: report_id.clone(),
                session_id: session,
                job_title,
                title: title.to_string(),
                what: what.to_string(),
                steps: lines(report.steps),
                errors: lines(report.errors),
                logs: lines(report.logs),
                at: self.now().as_str().to_string(),
                kept: None,
            };
            store.keep_bug_report(&kept).map_err(|why| self.ledger_fault(why))?;
            store.bug_reports().map_err(|why| self.ledger_fault(why))?
        };
        self.bug_reports_published(reports);
        Ok(BugReported { report_id })
    }

    /// Every report on this Mac, newest first.
    pub(crate) async fn bug_reports_listed(&self) -> Result<BugReports, Refusal> {
        let kept = self.store().lock().await.bug_reports().map_err(|why| self.ledger_fault(why))?;
        Ok(BugReports { reports: kept.into_iter().map(wire).collect() })
    }

    /// The owner keeps report `id`. A report already kept keeps its first time.
    pub(crate) async fn bug_report_kept(&self, id: String) -> Result<BugReport, Refusal> {
        let (report, reports) = {
            let mut store = self.store().lock().await;
            let now = self.now().as_str().to_string();
            let report = store.mark_bug_report_kept(&id, &now).map_err(|why| self.ledger_fault(why))?;
            (report, store.bug_reports().map_err(|why| self.ledger_fault(why))?)
        };
        let Some(report) = report else { return Err(self.no_such_bug_report(&id)) };
        self.bug_reports_published(reports);
        Ok(wire(report))
    }

    /// The owner drops report `id`, a kept one too.
    pub(crate) async fn bug_report_dropped(&self, id: String) -> Result<BugReportDropped, Refusal> {
        let (dropped, reports) = {
            let mut store = self.store().lock().await;
            let dropped = store.drop_bug_report(&id).map_err(|why| self.ledger_fault(why))?;
            (dropped, store.bug_reports().map_err(|why| self.ledger_fault(why))?)
        };
        if !dropped {
            return Err(self.no_such_bug_report(&id));
        }
        self.bug_reports_published(reports);
        Ok(BugReportDropped { id })
    }

    fn no_such_bug_report(&self, id: &str) -> Refusal {
        Refusal::NotFound(WireError::raised(NO_SUCH_BUG_REPORT, format!("no bug report {id}"), self.run_id()))
    }

    /// Tell Bridge the reports moved, with all of them.
    fn bug_reports_published(&self, reports: Vec<KeptBugReport>) {
        self.publish(ipc::Event::BugReportsChanged(BugReports { reports: reports.into_iter().map(wire).collect() }));
    }
}
