//! The preview across his Macs: the ready offers a paired Mac shares, held beside this Mac's own, merged into
//! one refresh. `docs/concepts/fleet.md`, *Offering a branch for the preview*.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{PreviewOfferRow, PreviewOffers};

use crate::daemon::Fleet;

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
    /// What of `offers` crosses to another Mac.
    pub(crate) fn preview_offers_cut(&self, offers: PreviewOffers) -> PreviewOffers {
        offers
    }

    pub(crate) async fn preview_offers_held(self: &std::sync::Arc<Self>, _mac: &str, _offers: Vec<PreviewOfferRow>) -> Result<(), Refusal> {
        Ok(())
    }
}
