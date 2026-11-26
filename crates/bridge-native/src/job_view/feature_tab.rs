//! A feature's page as a tab beside the Job's thread. A feature chip on the step line and the rows of the Story tab all open it
//! here, in the pane that holds the thread; pressing the same feature again brings its tab forward. The tab draws the
//! [`FeatureOverlay`] the Story tab used to put over itself, and Back on its first page closes the tab.
//!
//! The tab keeps the feature's slug in `row` and its title in `label`, so a kept layout names them and a Job reopened draws them again.

use std::collections::HashMap;

use gpui::{div, prelude::*, AnyElement, Context, Entity, EntityId, Global, SharedString, Subscription, Window};

use super::pane_tree::{self as tree, Arrangement, Extras, Out};
use super::JobView;
use crate::features::overlay::{FeatureOverlay, OverlayEvent};

/// The tab's kind in a Job's panes.
pub(super) const KIND: &str = "feature";

/// Each Job view's feature tabs, by Job and then by tab id, held app-wide as the Loadout's are.
#[derive(Default)]
struct Pages {
    by_view: HashMap<(EntityId, String), HashMap<String, (Entity<FeatureOverlay>, Subscription)>>,
}

impl Global for Pages {}

/// The id of the tab that holds feature `slug`, where one is open.
pub(super) fn open_tab(arrangement: &Arrangement, slug: &str) -> Option<String> {
    tree::panes_of(&arrangement.root).into_iter().flat_map(|pane| pane.tabs.iter()).find(|tab| tab.kind == KIND && tab.row.as_deref() == Some(slug)).map(|tab| tab.id.clone())
}

/// The feature's tab in front: the one open already, else a new one at the end of the pane that holds the thread (the focused pane
/// where none does).
pub(super) fn place<'a>(arrangement: &'a Arrangement, slug: &str, title: &str) -> Out<'a> {
    if let Some(open) = open_tab(arrangement, slug) {
        return tree::activate(arrangement, &open);
    }
    let pane = tree::holder_of(arrangement, "thread").map_or_else(|| arrangement.focus.clone(), |pane| pane.id.clone());
    tree::add_tab(arrangement, &pane, KIND, Extras { label: Some(title.to_string()), row: Some(slug.to_string()), ..Extras::default() })
}

impl JobView {
    /// Feature `slug` of Job `job` opens in a tab of the thread's pane, in front and with the keys.
    pub(super) fn open_feature_tab(&mut self, job: &str, slug: String, title: String, window: &mut Window, cx: &mut Context<Self>) {
        let job = SharedString::from(job.to_string());
        let (wanted, named) = (slug.clone(), title);
        self.rearrange_in(&job, None, true, move |arrangement| place(arrangement, &wanted, &named), cx);
        let Some(arrangement) = self.arrangement_of(&job).cloned() else { return };
        let Some(tab) = open_tab(&arrangement, &slug) else { return };
        if let Some(pane) = tree::pane_of_tab(&arrangement, &tab) {
            self.focus_in_pane(&pane.id.clone(), window, cx);
        }
        self.focus_feature(&tab, window, cx);
        self.want_pane_reads(window, cx);
        cx.notify();
    }

    /// The keys go to the page the tab draws, once it is made.
    pub(super) fn focus_feature(&mut self, tab: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let Some(page) = self.page_of(&job, tab, window, cx) else { return };
        window.defer(cx, move |window, cx| page.update(cx, |page, cx| page.focus_top(window, cx)));
    }

    /// The body of a feature tab in front: its page, made as it first draws.
    pub(super) fn feature_body(&mut self, job: &SharedString, tab: &tree::Tab, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let page = self.page_of(job, &tab.id, window, cx)?;
        Some(div().id(SharedString::from(format!("pane-body:{}", tab.id))).flex_1().min_h_0().flex().flex_col().child(div().size_full().child(page)).into_any_element())
    }

    /// The overlay a feature tab draws, made once; those of tabs that have closed go.
    fn page_of(&mut self, job: &SharedString, tab: &str, window: &mut Window, cx: &mut Context<Self>) -> Option<Entity<FeatureOverlay>> {
        let me = cx.entity_id();
        let arrangement = self.arrangement_of(job)?.clone();
        let held = tree::tab_by_id(&arrangement, tab)?.clone();
        let key = (me, job.to_string());
        let all = cx.default_global::<Pages>();
        let pages = all.by_view.entry(key.clone()).or_default();
        pages.retain(|id, _| tree::tab_by_id(&arrangement, id).is_some());
        if let Some((page, _)) = pages.get(tab) {
            return Some(page.clone());
        }
        let (slug, title) = (held.row.clone()?, held.label.clone().unwrap_or_default());
        let story = self.story_of(job, window, cx);
        let (source, project, from) = story.read(cx).feature_source(cx)?;
        let page = cx.new(|cx| FeatureOverlay::on_page(source, project, from, slug, title, window, cx));
        let (shown, closing) = (job.clone(), tab.to_string());
        let sub = cx.subscribe_in(&page, window, move |this, _, event: &OverlayEvent, window, cx| match event {
            OverlayEvent::Closed => this.close_feature_tab(&shown, &closing, window, cx),
        });
        cx.default_global::<Pages>().by_view.entry(key).or_default().insert(tab.to_string(), (page.clone(), sub));
        Some(page)
    }

    /// Back on the page's first layer: the tab closes, and the keys go to the thread.
    fn close_feature_tab(&mut self, job: &SharedString, tab: &str, window: &mut Window, cx: &mut Context<Self>) {
        let closing = tab.to_string();
        self.rearrange_in(job, None, true, move |arrangement| tree::close_tab(arrangement, &closing), cx);
        let key = (cx.entity_id(), job.to_string());
        if let Some(pages) = cx.default_global::<Pages>().by_view.get_mut(&key) {
            pages.remove(tab);
        }
        let focus = self.arrangement().focus.clone();
        self.focus_in_pane(&focus, window, cx);
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job_view::pane_tree::{build, panes_of, Dir, Shape};

    fn arranged() -> Arrangement {
        build(&Shape::split(Dir::Row, 0.62, Shape::pane(&["thread", "story"]), Shape::pane(&["now"])), None, &[])
    }

    fn feature_tabs(arrangement: &Arrangement) -> Vec<(String, Option<String>, Option<String>)> {
        panes_of(&arrangement.root).into_iter().flat_map(|pane| pane.tabs.iter()).filter(|tab| tab.kind == KIND).map(|tab| (tab.id.clone(), tab.label.clone(), tab.row.clone())).collect()
    }

    #[test]
    fn a_feature_opens_in_the_pane_of_the_thread_in_front() {
        let arrangement = arranged();
        let opened = place(&arrangement, "start-a-job", "Start a Job").into_owned();
        let holder = tree::holder_of(&opened, "thread").expect("the thread's pane");
        let tab = holder.tabs.last().expect("a tab");
        assert_eq!((tab.kind.as_str(), tab.label.as_deref(), tab.row.as_deref()), (KIND, Some("Start a Job"), Some("start-a-job")));
        assert_eq!(holder.active.as_deref(), Some(tab.id.as_str()));
        assert_eq!(opened.focus, holder.id);
    }

    #[test]
    fn the_same_feature_again_brings_its_tab_forward_and_adds_none() {
        let once = place(&arranged(), "start-a-job", "Start a Job").into_owned();
        let thread = tree::holder_of(&once, "thread").and_then(|pane| pane.tabs.iter().find(|tab| tab.kind == "thread")).map(|tab| tab.id.clone()).expect("thread");
        let behind = tree::activate(&once, &thread).into_owned();
        let again = place(&behind, "start-a-job", "Start a Job").into_owned();
        assert_eq!(feature_tabs(&again).len(), 1);
        let held = open_tab(&again, "start-a-job").expect("its tab");
        assert_eq!(tree::holder_of(&again, KIND).and_then(|pane| pane.active.clone()), Some(held));
    }

    #[test]
    fn another_feature_gets_its_own_tab() {
        let two = place(&place(&arranged(), "a", "A").into_owned(), "b", "B").into_owned();
        assert_eq!(feature_tabs(&two).iter().map(|one| one.2.clone().unwrap()).collect::<Vec<_>>(), ["a", "b"]);
    }

    #[test]
    fn a_kept_layout_with_a_feature_tab_is_revived() {
        let opened = place(&arranged(), "a", "A").into_owned();
        let kept = serde_json::to_value(&opened).expect("kept");
        assert!(tree::revive(&kept).is_some());
    }
}
