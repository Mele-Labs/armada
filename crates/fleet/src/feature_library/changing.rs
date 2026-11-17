//! A feature merged into another or renamed, on the Operator's word once the owner has chosen on a card. **The old slug stays
//! as an alias**, so nothing a Job or an act still names is lost. `docs/concepts/feature-catalog.md`, *Confirming and merging*.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ChangeFeature, FeatureChanged, WireError};

use super::reading::Loaded;
use super::writing::{slug_ok, SLUG};
use crate::daemon::Fleet;

const CHANGE: &str = "fleet.feature_change";
const TAKEN: &str = "fleet.feature_slug_taken";

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
    /// `change_feature`. **Nothing is kept on a refusal.**
    pub(super) async fn feature_changed(&self, caller: Option<api::Caller>, change: ChangeFeature) -> Result<FeatureChanged, Refusal> {
        self.operator_called(caller)?;
        let project = change.project.trim().to_string();
        let blank = |one: &Option<String>| one.as_deref().map(str::trim).filter(|one| !one.is_empty()).map(str::to_string);
        let (into, slug, title) = (blank(&change.into), blank(&change.slug), blank(&change.title));
        if into.is_some() && (slug.is_some() || title.is_some()) {
            return Err(self.hosted_refusal(CHANGE, "a feature is merged into another or renamed, and a call does one"));
        }
        if into.is_none() && slug.is_none() && title.is_none() {
            return Err(self.hosted_refusal(CHANGE, "name `into` to merge it, or `slug` and `title` to rename it"));
        }
        if let Some(slug) = slug.as_deref().filter(|slug| !slug_ok(slug)) {
            return Err(self.hosted_refusal(SLUG, &format!("`{slug}` is not a slug: lowercase letters, digits and dashes")));
        }
        let mut store = self.store().lock().await;
        let loaded = Loaded::read(&store, &project).map_err(|why| self.ledger_fault(why))?;
        let from = loaded.feature_named(change.feature.trim()).ok_or_else(|| self.feature_unknown(change.feature.trim()))?.slug.clone();
        match (into, slug) {
            (Some(into), _) => {
                let target = loaded.feature_named(&into).ok_or_else(|| self.feature_unknown(&into))?.slug.clone();
                if target == from {
                    return Err(self.hosted_refusal(CHANGE, "a feature cannot be merged into itself"));
                }
                store.merge_feature(&project, &from, &target).map_err(|why| self.ledger_fault(why))?;
                Ok(FeatureChanged { feature: target.clone(), merged_into: Some(target) })
            }
            (None, slug) => {
                let to = slug.unwrap_or_else(|| from.clone());
                if to != from && loaded.feature(&to).is_some() {
                    return Err(Refusal::IllegalMove(WireError::raised(TAKEN, format!("a feature is named `{to}` already"), self.run_id())));
                }
                store.rename_feature(&project, &from, &to, title.as_deref()).map_err(|why| self.ledger_fault(why))?;
                Ok(FeatureChanged { feature: to, merged_into: None })
            }
        }
    }
}
