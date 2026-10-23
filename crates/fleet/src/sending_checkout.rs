//! A held checkout's uncommitted files, sent up as a pull request on the owner's
//! press instead of being stashed by a pull.
//!
//! **The checkout is the owner's and is not touched.** The commit is made from
//! objects on top of `origin`'s tip of the base (`Delivery::send_checkout_as_pull_request`),
//! so the working tree, the index and `HEAD` stay as they were. Fleet then keeps
//! the pull request's number beside the hold until the hold clears, and
//! `catching_up` catches the checkout up by itself once the change is on main.

use adapter_traits::{AgentHarness, CheckoutToSend, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{BackgroundAct, WireError};

use crate::catching_up::{why_not_pull, CHECKOUT_NOT_HELD};
use crate::daemon::Fleet;
use crate::pull_requesting::number_of;
use crate::refusing::NOT_SERVED;

/// Pressed for a hold that already carries a pull request. A 409: the card was
/// drawn before the first press answered, or a second window pressed again.
const CHECKOUT_ALREADY_SENT: &str = "fleet.checkout_already_sent";
/// The commit, the push or the pull request would not go. A 500, with the
/// tool's words; the hold is unchanged and the press may be made again.
const CHECKOUT_NOT_SENT: &str = "fleet.checkout_not_sent";

/// The title a pull request from these paths carries: the path itself where
/// there is one, else how many.
fn titled(paths: &[String]) -> String {
    match paths {
        [only] => only.clone(),
        many => format!("{} files", many.len()),
    }
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
    /// `checkout_to_pull_request`: the hold's uncommitted files as a pull request
    /// into the base, auto-merge on, recorded on the hold.
    ///
    /// **Answered once the send can be tried**: the repository is served and names a base, nothing
    /// was sent already, and the hold lists files to send. The commit, the push and the pull request
    /// finish in a task of their own, which the hub shows by the pull request on the hold, and a
    /// refusal there is `act.failed`.
    pub(crate) async fn checkout_sent_as_pull_request(
        self: std::sync::Arc<Self>,
        asked: ipc::CheckoutToPullRequest,
    ) -> Result<ipc::CheckoutSentAsPullRequest, Refusal> {
        let Some(served) = self.repositories().serving(asked.manifest_id.as_str()) else {
            return Err(Refusal::Unacceptable(self.send_refusal(
                NOT_SERVED,
                format!("this Fleet does not serve {}", asked.manifest_id.as_str()),
            )));
        };
        let root = served.root().to_string();
        let Some(base) = served.manifest().base().map(str::to_string) else {
            return Err(Refusal::IllegalMove(self.send_refusal(
                CHECKOUT_NOT_HELD,
                String::from("the repository names no base to send a pull request into"),
            )));
        };
        self.checkout_to_send(&root, &base).await?;
        let manifest_id = asked.manifest_id;
        tokio::spawn(async move {
            if let Err(refusal) = self.checkout_sent(&root, base).await {
                self.act_failed(BackgroundAct::CheckoutToPullRequest, Some(manifest_id), None, refusal);
            }
        });
        Ok(ipc::CheckoutSentAsPullRequest {})
    }

    fn send_refusal(&self, code: &'static str, said: String) -> WireError {
        WireError::raised(code, said, self.run_id())
    }

    /// What the hold lists to send, or why it lists nothing. Refused as `catch_up_checkout` is,
    /// and where this hold already carries a pull request.
    async fn checkout_to_send(&self, root: &str, base: &str) -> Result<CheckoutToSend, Refusal> {
        if let Some(sent) = self.sweeping().lock().await.checkout_sent.get(root) {
            return Err(Refusal::IllegalMove(self.send_refusal(
                CHECKOUT_ALREADY_SENT,
                format!("these files already went up as #{}", sent.number),
            )));
        }
        let held = self
            .forge_asked(root, {
                let base = base.to_string();
                move |vcs: &V, root: &str| vcs.held_back(root, &base)
            })
            .await;
        if let Some((code, said)) = why_not_pull(held.as_ref()) {
            return Err(Refusal::IllegalMove(self.send_refusal(code, said)));
        }
        let held = held.expect("why_not_pull refuses where there is no hold");
        Ok(CheckoutToSend {
            message: titled(&held.uncommitted),
            paths: held.uncommitted,
            branch: format!("armada/checkout-{:08x}", rand::random::<u32>()),
            base: base.to_string(),
        })
    }

    /// The send, with the hold read again under the merge end: it holds the merge end while the
    /// commit is made, because the pull `checkout_caught_up` runs stashes these very files for a
    /// moment and the commit would read them gone.
    async fn checkout_sent(&self, root: &str, base: String) -> Result<(), Refusal> {
        let _at_the_merge_end = self.merge_end().lock().await;
        let sending = self.checkout_to_send(root, &base).await?;
        let sent = self
            .forge_asked(root, move |vcs: &V, root: &str| vcs.send_checkout_as_pull_request(root, &sending))
            .await
            .map_err(|why| Refusal::Fault(self.send_refusal(CHECKOUT_NOT_SENT, why.said())))?;
        let number = number_of(&sent.pull_request).and_then(|number| u32::try_from(number).ok()).ok_or_else(|| {
            Refusal::Fault(self.send_refusal(
                CHECKOUT_NOT_SENT,
                format!("the pull request opened, but `{}` names no number", sent.pull_request),
            ))
        })?;
        let pull_request = ipc::CheckoutPullRequest { number, auto_merge: sent.auto_merge.is_ok() };
        self.sweeping().lock().await.checkout_sent.insert(root.to_string(), pull_request);
        Ok(())
    }
}
