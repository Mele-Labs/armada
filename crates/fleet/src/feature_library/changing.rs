//! A feature merged into another, renamed or moved to another area, and an area made, renamed, split or merged, on the
//! owner's word or the Operator's once he has chosen on a card. **The old slug of a feature stays as an alias**, so nothing
//! a Job or an act still names is lost. His reason, where he gave one, is kept as a rule of the catalog in the same hold of
//! the store. `docs/concepts/feature-catalog.md`, *Confirming and merging* and *Areas*.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{AreaChange, AreaChanged, CatalogRuleAct, ChangeArea, ChangeFeature, FeatureChanged, WireError};

use super::catalog_rules::About;
use super::pending::PENDING_AREA;
use super::reading::Loaded;
use super::writing::{slug_ok, SLUG};
use crate::daemon::Fleet;

const CHANGE: &str = "fleet.feature_change";
const TAKEN: &str = "fleet.feature_slug_taken";
/// An area act that names too little, or one that would change nothing.
const AREA_CHANGE: &str = "fleet.area_change";
/// A new area slug the project keeps already.
const AREA_TAKEN: &str = "fleet.area_slug_taken";
use super::writing::AREA_UNKNOWN as AREA;

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
        self.owner_or_operator_called(caller)?;
        let project = change.project.trim().to_string();
        let blank = |one: &Option<String>| one.as_deref().map(str::trim).filter(|one| !one.is_empty()).map(str::to_string);
        let (into, slug, title, area) = (blank(&change.into), blank(&change.slug), blank(&change.title), blank(&change.area));
        if into.is_some() && (slug.is_some() || title.is_some() || area.is_some()) {
            return Err(self.hosted_refusal(CHANGE, "a feature is merged into another, or renamed or moved, and a call does one of them"));
        }
        if into.is_none() && slug.is_none() && title.is_none() && area.is_none() {
            return Err(self.hosted_refusal(CHANGE, "name `into` to merge it, `slug` and `title` to rename it, or `area` to move it"));
        }
        if let Some(slug) = slug.as_deref().filter(|slug| !slug_ok(slug)) {
            return Err(self.hosted_refusal(SLUG, &format!("`{slug}` is not a slug: lowercase letters, digits and dashes")));
        }
        let mut store = self.store().lock().await;
        let loaded = Loaded::read(&store, &project).map_err(|why| self.ledger_fault(why))?;
        if let Some(area) = area.as_deref().filter(|area| !loaded.areas.iter().any(|kept| kept.slug == *area)) {
            return Err(self.hosted_refusal(AREA, &format!("the project keeps no area `{area}`; keep it first with `upsert_features`")));
        }
        let from = loaded.feature_named(change.feature.trim()).ok_or_else(|| self.feature_unknown(change.feature.trim()))?.slug.clone();
        match (into, slug) {
            (Some(into), _) => {
                let target = loaded.feature_named(&into).ok_or_else(|| self.feature_unknown(&into))?.slug.clone();
                if target == from {
                    return Err(self.hosted_refusal(CHANGE, "a feature cannot be merged into itself"));
                }
                store.merge_feature(&project, &from, &target).map_err(|why| self.ledger_fault(why))?;
                let about = About { features: vec![from, target.clone()], ..About::default() };
                let rule = self.rule_kept(&mut store, &project, change.why.as_deref(), CatalogRuleAct::MergeFeatures, about).map_err(|why| self.ledger_fault(why))?;
                Ok(FeatureChanged { feature: target.clone(), merged_into: Some(target), rule })
            }
            (None, slug) => {
                let renamed = slug.is_some() || title.is_some();
                let to = slug.unwrap_or_else(|| from.clone());
                if to != from && loaded.feature(&to).is_some() {
                    return Err(Refusal::IllegalMove(WireError::raised(TAKEN, format!("a feature is named `{to}` already"), self.run_id())));
                }
                if renamed {
                    store.rename_feature(&project, &from, &to, title.as_deref()).map_err(|why| self.ledger_fault(why))?;
                }
                if let Some(area) = &area {
                    store.set_feature_area(&project, &to, area).map_err(|why| self.ledger_fault(why))?;
                }
                let (made_by, features) = match renamed {
                    true if to != from => (CatalogRuleAct::RenameFeature, vec![from, to.clone()]),
                    true => (CatalogRuleAct::RenameFeature, vec![to.clone()]),
                    false => (CatalogRuleAct::MoveFeature, vec![to.clone()]),
                };
                let about = About { features, areas: area.into_iter().collect(), ..About::default() };
                let rule = self.rule_kept(&mut store, &project, change.why.as_deref(), made_by, about).map_err(|why| self.ledger_fault(why))?;
                Ok(FeatureChanged { feature: to, merged_into: None, rule })
            }
        }
    }

    /// `change_area`. **Nothing is kept on a refusal**, and a move of features files every Job's writes again, as an upsert does.
    pub(super) async fn area_changed(&self, caller: Option<api::Caller>, change: ChangeArea) -> Result<AreaChanged, Refusal> {
        self.owner_or_operator_called(caller)?;
        let project = change.project.trim().to_string();
        let area = change.area.trim().to_string();
        let blank = |one: &Option<String>| one.as_deref().map(str::trim).filter(|one| !one.is_empty()).map(str::to_string);
        let mut store = self.store().lock().await;
        let loaded = Loaded::read(&store, &project).map_err(|why| self.ledger_fault(why))?;
        let kept = |slug: &str| loaded.areas.iter().find(|one| one.slug == slug);
        let unknown = |slug: &str| self.hosted_refusal(AREA, &format!("the project keeps no area `{slug}`"));
        let free = |slug: &str| -> Result<(), Refusal> {
            if !slug_ok(slug) {
                return Err(self.hosted_refusal(SLUG, &format!("`{slug}` is not a slug: lowercase letters, digits and dashes")));
            }
            if slug == PENDING_AREA || kept(slug).is_some() {
                return Err(Refusal::IllegalMove(WireError::raised(AREA_TAKEN, format!("an area is named `{slug}` already"), self.run_id())));
            }
            Ok(())
        };
        let why = change.why.as_deref();
        let (now, made_by, about) = match change.change {
            AreaChange::Make { title } => {
                let title = title.trim();
                if title.is_empty() {
                    return Err(self.hosted_refusal(AREA_CHANGE, "a new area needs a title"));
                }
                free(&area)?;
                store.make_feature_area(&project, &area, title).map_err(|why| self.ledger_fault(why))?;
                (area.clone(), CatalogRuleAct::MakeArea, About { areas: vec![area], ..About::default() })
            }
            AreaChange::Rename { slug, title } => {
                kept(&area).ok_or_else(|| unknown(&area))?;
                let (slug, title) = (blank(&slug).filter(|slug| *slug != area), blank(&title));
                if slug.is_none() && title.is_none() {
                    return Err(self.hosted_refusal(AREA_CHANGE, "name the area's new `slug`, its new `title`, or both"));
                }
                if let Some(slug) = &slug {
                    free(slug)?;
                }
                let to = slug.unwrap_or_else(|| area.clone());
                store.rename_feature_area(&project, &area, &to, title.as_deref()).map_err(|why| self.ledger_fault(why))?;
                let areas = if to == area { vec![to.clone()] } else { vec![area, to.clone()] };
                (to, CatalogRuleAct::RenameArea, About { areas, ..About::default() })
            }
            AreaChange::Split { slug, title, features } => {
                kept(&area).ok_or_else(|| unknown(&area))?;
                let (slug, title) = (slug.trim().to_string(), title.trim());
                if title.is_empty() {
                    return Err(self.hosted_refusal(AREA_CHANGE, "the new area needs a title"));
                }
                free(&slug)?;
                let mut moved: Vec<String> = Vec::new();
                for one in features.iter().map(|one| one.trim()).filter(|one| !one.is_empty()) {
                    let feature = loaded.feature_named(one).ok_or_else(|| self.feature_unknown(one))?;
                    if feature.area != area {
                        return Err(self.hosted_refusal(AREA_CHANGE, &format!("`{}` is not in the area `{area}`", feature.slug)));
                    }
                    if !moved.contains(&feature.slug) {
                        moved.push(feature.slug.clone());
                    }
                }
                if moved.is_empty() {
                    return Err(self.hosted_refusal(AREA_CHANGE, "name the features that go to the new area"));
                }
                store.split_feature_area(&project, &area, &slug, title, &moved).map_err(|why| self.ledger_fault(why))?;
                (slug.clone(), CatalogRuleAct::SplitArea, About { features: moved, areas: vec![area, slug], ..About::default() })
            }
            AreaChange::Merge { into } => {
                kept(&area).ok_or_else(|| unknown(&area))?;
                let into = into.trim().to_string();
                kept(&into).ok_or_else(|| unknown(&into))?;
                if into == area {
                    return Err(self.hosted_refusal(AREA_CHANGE, "an area cannot be merged into itself"));
                }
                store.merge_feature_area(&project, &area, &into).map_err(|why| self.ledger_fault(why))?;
                (into.clone(), CatalogRuleAct::MergeAreas, About { areas: vec![area, into], ..About::default() })
            }
        };
        let rule = self.rule_kept(&mut store, &project, why, made_by, about).map_err(|why| self.ledger_fault(why))?;
        drop(store);
        if made_by != CatalogRuleAct::MakeArea {
            self.feature_writes_refiled().await;
        }
        Ok(AreaChanged { project, area: now, rule })
    }
}
