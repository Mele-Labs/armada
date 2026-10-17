//! Running a Check or Command by hand, as the fake answers it: it runs nothing.


use super::FakeDaemon;
use crate::{Refusal, Rehearsing};

impl Rehearsing for FakeDaemon {
    async fn start_checkout_run(
        self: std::sync::Arc<Self>,
        _run: ipc::StartCheckoutRun,
        _manifest_id: Option<ipc::ManifestId>,
        _repository: Option<String>,
    ) -> Result<ipc::CheckoutRunUnderway, Refusal> {
        self.runs_nothing_here()
    }
    async fn stop_checkout_run(
        &self,
        _run: ipc::NamedRun,
        _manifest_id: Option<ipc::ManifestId>,
        _repository: Option<String>,
    ) -> Result<ipc::CheckoutRunRecord, Refusal> {
        self.runs_nothing_here()
    }
    async fn undo_checkout_run(
        &self,
        _run: ipc::NamedRun,
        _manifest_id: Option<ipc::ManifestId>,
        _repository: Option<String>,
    ) -> Result<ipc::CheckoutRunRecord, Refusal> {
        self.runs_nothing_here()
    }
    async fn start_checkout_verify(
        self: std::sync::Arc<Self>,
        _asked: ipc::StartCheckoutVerify,
        _manifest_id: Option<ipc::ManifestId>,
        _repository: Option<String>,
    ) -> Result<ipc::CheckoutVerify, Refusal> {
        self.runs_nothing_here()
    }
}
