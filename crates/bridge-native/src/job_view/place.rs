//! Where he left off, kept between runs: the Job open, and for each Job the tab in front of its top pane,
//! the tab in front of its thread's pane and, where the thread was scrolled up, the row at its top.
//!
//! One plain file beside the list's own (`jobs::deck_file`), a line each and tab apart:
//! `open<TAB>job`, `top<TAB>job<TAB>zone`, `row<TAB>job<TAB>kind<TAB>key`, `scroll<TAB>job<TAB>item<TAB>offset`.
//! Written a moment after a change and read at start, never on mock data. A thread that follows its tail
//! keeps no line, so it opens following. A Job Fleet no longer lists, or a row the thread no longer holds,
//! is passed over, and the screen opens as it does with nothing kept.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::Duration;

use gpui::{App, Context, ListOffset, SharedString};
use ipc::ThreadItem;

use crate::log::Note;
use super::side::Zone;
use super::JobView;

/// The file's name, beside `native-deck`.
const FILE: &str = "native-place";

/// How long after the last change the file is written, so a scroll is one write.
const SETTLE: Duration = Duration::from_millis(300);

/// Where one Job was left.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Spot {
    /// The tab in front of the top pane, by the zone's id.
    pub top: Option<String>,
    /// The tab in front of the thread's pane where it is not the thread: its kind and key.
    pub row: Option<(String, String)>,
    /// The thread's top item by id and how far into it, where it was scrolled up.
    pub scroll: Option<(String, i32)>,
}

/// What is kept: the Job open, and each Job's spot.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Kept {
    pub open: Option<String>,
    pub jobs: BTreeMap<String, Spot>,
}

/// A word that fits on a line of the file.
fn plain(word: &str) -> bool {
    !word.is_empty() && !word.contains(['\t', '\n', '\r'])
}

impl Kept {
    pub(crate) fn text(&self) -> String {
        let mut lines = Vec::new();
        if let Some(open) = self.open.as_ref().filter(|open| plain(open)) {
            lines.push(format!("open\t{open}"));
        }
        for (job, spot) in self.jobs.iter().filter(|(job, _)| plain(job)) {
            if let Some(top) = spot.top.as_ref().filter(|top| plain(top)) {
                lines.push(format!("top\t{job}\t{top}"));
            }
            if let Some((kind, key)) = spot.row.as_ref().filter(|(kind, key)| plain(kind) && plain(key)) {
                lines.push(format!("row\t{job}\t{kind}\t{key}"));
            }
            if let Some((item, offset)) = spot.scroll.as_ref().filter(|(item, _)| plain(item)) {
                lines.push(format!("scroll\t{job}\t{item}\t{offset}"));
            }
        }
        lines.join("\n")
    }

    /// What `text` holds; a line it cannot read is passed over.
    pub(crate) fn from_text(text: &str) -> Kept {
        let mut kept = Kept::default();
        for line in text.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            match parts.as_slice() {
                ["open", job] if plain(job) => kept.open = Some(job.to_string()),
                ["top", job, zone] if plain(job) && plain(zone) => kept.jobs.entry(job.to_string()).or_default().top = Some(zone.to_string()),
                ["row", job, kind, key] if plain(job) && plain(kind) && plain(key) => kept.jobs.entry(job.to_string()).or_default().row = Some((kind.to_string(), key.to_string())),
                ["scroll", job, item, offset] if plain(job) && plain(item) => {
                    if let Ok(offset) = offset.parse::<i32>() {
                        kept.jobs.entry(job.to_string()).or_default().scroll = Some((item.to_string(), offset));
                    }
                }
                _ => {}
            }
        }
        kept
    }

    /// What the file holds; nothing where it holds none or cannot be read.
    pub(crate) fn read() -> Kept {
        file().and_then(|path| std::fs::read_to_string(path).ok()).map(|text| Kept::from_text(&text)).unwrap_or_default()
    }
}

fn file() -> Option<PathBuf> {
    crate::jobs::deck_file().map(|deck| deck.with_file_name(FILE))
}

fn write(text: &str) {
    let Some(path) = file() else { return };
    if let Err(why) = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| std::fs::write(&path, text)) {
        Note::warn("where he left off was not kept").field("why", why.to_string()).emit();
    }
}

/// The place a walk's reopen hands to the screen it builds, as the file hands it to a restart.
#[derive(Clone)]
pub(crate) struct Reopened(pub Kept);

impl gpui::Global for Reopened {}

/// The place a walk holds for the screen it builds next, left in place for the Jobs list to read.
pub(crate) fn reopened(cx: &App) -> Option<Kept> {
    cx.try_global::<Reopened>().map(|held| held.0.clone())
}

/// The thread's top as a row and how far into it, where it was scrolled up. `item_ix` and `offset` are the
/// list's top, `skip` the elements above the Job's own first item, and `id_at` the id of the Job's item at a place.
/// A thread that follows its tail has none, and nor has a top that is past its items.
pub(crate) fn anchor_of(following: bool, item_ix: usize, offset: f32, skip: usize, id_at: impl Fn(usize) -> Option<String>) -> Option<(String, i32)> {
    if following {
        return None;
    }
    let id = id_at(item_ix.saturating_sub(skip))?;
    let into = if item_ix < skip { 0 } else { offset.max(0.).round() as i32 };
    Some((id, into))
}

/// Where a kept top is in the thread now: the place of its item, and how far into it. None where the item is gone.
pub(crate) fn resolve(anchor: &(String, i32), place_of: impl Fn(&str) -> Option<usize>) -> Option<(usize, i32)> {
    place_of(&anchor.0).map(|at| (at, anchor.1))
}

/// What a Job view holds of the place: what was kept, what is still to be put back, and the write waiting.
#[derive(Default)]
pub(super) struct Place {
    pub(super) kept: Kept,
    /// Whether a change is written: not on mock data.
    remember: bool,
    /// The tab in front of a Job's thread pane, put back once its thread is held.
    pub(super) wants_row: HashMap<SharedString, (String, String)>,
    /// A Job's scrolled place, put back once its thread is first laid out.
    pub(super) wants_scroll: HashMap<SharedString, (String, i32)>,
    waiting: Option<gpui::Task<()>>,
}

impl Place {
    /// A walk's reopen hands over what the last screen held; a live app reads the file; a walk otherwise starts bare.
    pub(super) fn begin(remember: bool, cx: &mut App) -> Place {
        let kept = if remember {
            Kept::read()
        } else {
            let held = cx.has_global::<Reopened>().then(|| cx.remove_global::<Reopened>().0);
            held.unwrap_or_default()
        };
        let wants_row = kept.jobs.iter().filter_map(|(job, spot)| spot.row.clone().map(|row| (SharedString::from(job.clone()), row))).collect();
        let wants_scroll = kept.jobs.iter().filter_map(|(job, spot)| spot.scroll.clone().map(|scroll| (SharedString::from(job.clone()), scroll))).collect();
        Place { kept, remember, wants_row, wants_scroll, waiting: None }
    }
}

fn zone_of(id: &str) -> Option<Zone> {
    Zone::ALL.into_iter().find(|zone| zone.id() == id)
}

impl JobView {
    /// The shown Job's spot as the screen has it now, over what was kept of it where that is still to be put back.
    fn spot_now(&self, id: &SharedString, cx: &App) -> Spot {
        let mut spot = self.place.kept.jobs.get(id.as_ref()).cloned().unwrap_or_default();
        spot.top = self.zone_shown().map(|zone| zone.id().to_string());
        if !self.place.wants_row.contains_key(id) {
            spot.row = self.row_front(id).and_then(|tab| super::row_tabs::kind_id(tab.kind).map(|kind| (kind.to_string(), tab.key.clone())));
        }
        if !self.place.wants_scroll.contains_key(id) {
            if let (Some(page), Some(held)) = (self.pages.get(id), self.thread(id, cx)) {
                if page.version.is_some() {
                    let top = page.list.logical_scroll_top();
                    spot.scroll = anchor_of(page.list.is_following_tail(), top.item_ix, f32::from(top.offset_in_item), page.lead + page.before, |at| held.items.get(at).map(|item| item.id().to_string()));
                }
            }
        }
        spot
    }

    /// Note where he is, and write it a moment after the last change. Cheap where nothing moved.
    pub(super) fn keep_place(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let spot = self.spot_now(&id, cx);
        let known = self.place.kept.jobs.get(id.as_ref()) == Some(&spot) && self.place.kept.open.as_deref() == Some(id.as_ref());
        if known {
            return;
        }
        self.place.kept.open = Some(id.to_string());
        self.place.kept.jobs.insert(id.to_string(), spot);
        // Once Fleet has answered for this Job, a Job it no longer lists takes its line away.
        if self.record(&id, cx).is_some() {
            let list = self.list.read(cx);
            self.place.kept.jobs.retain(|job, _| list.record(job).is_some());
        }
        if !self.place.remember {
            return;
        }
        self.place.waiting = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SETTLE).await;
            let Ok(text) = this.read_with(cx, |this, _| this.place.kept.text()) else { return };
            cx.background_executor().spawn(async move { write(&text) }).await;
        }));
    }

    /// What is kept now, for a walk's reopen.
    pub(crate) fn place_now(&mut self, cx: &mut Context<Self>) -> Kept {
        self.keep_place(cx);
        self.place.kept.clone()
    }

    /// A Job comes on screen: its top pane's tab comes forward as it was left, where one was kept.
    pub(super) fn place_arrive(&mut self, id: &SharedString) {
        let Some(zone) = self.place.kept.jobs.get(id.as_ref()).and_then(|spot| spot.top.as_deref()).and_then(zone_of) else { return };
        self.bring_zone(id, zone);
        self.zone = zone;
    }

    /// The scrolled place kept for a Job goes back once its thread is laid out; none is kept for one that followed.
    pub(super) fn place_scroll(&mut self, id: &SharedString, cx: &App) {
        let Some(anchor) = self.place.wants_scroll.remove(id) else { return };
        let Some(held) = self.thread(id, cx) else { return };
        let Some((at, offset)) = resolve(&anchor, |item| held.items.iter().position(|one: &ThreadItem| one.id() == item)) else { return };
        if let Some(page) = self.pages.get(id) {
            page.list.scroll_to(ListOffset { item_ix: page.entry(at), offset_in_item: gpui::px(offset as f32) });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kept() -> Kept {
        let mut kept = Kept { open: Some("job-b".into()), ..Default::default() };
        kept.jobs.insert("job-a".into(), Spot { top: Some("plan".into()), row: Some(("draft".into(), "dra-1".into())), scroll: Some(("row-40".into(), 120)) });
        kept.jobs.insert("job-b".into(), Spot { top: Some("done".into()), row: None, scroll: None });
        kept
    }

    #[test]
    fn the_place_file_comes_back_as_it_was_written() {
        let text = kept().text();
        assert_eq!(Kept::from_text(&text), kept());
        assert_eq!(text.lines().next(), Some("open\tjob-b"));
    }

    #[test]
    fn the_tab_comes_back_for_each_job() {
        let back = Kept::from_text(&kept().text());
        assert_eq!(back.jobs["job-a"].top.as_deref().and_then(zone_of), Some(Zone::Plan));
        assert_eq!(back.jobs["job-b"].top.as_deref().and_then(zone_of), Some(Zone::Done));
        assert_eq!(back.jobs["job-a"].row, Some(("draft".into(), "dra-1".into())));
    }

    #[test]
    fn a_line_that_cannot_be_read_is_passed_over() {
        let back = Kept::from_text("open\tjob-a\nscroll\tjob-a\titem\tnot-a-number\ntop\tjob-a\nsomething else\ntop\tjob-a\tdone");
        assert_eq!(back.open.as_deref(), Some("job-a"));
        assert_eq!(back.jobs["job-a"], Spot { top: Some("done".into()), ..Default::default() });
        assert_eq!(Kept::from_text(""), Kept::default());
    }

    #[test]
    fn a_zone_no_longer_built_is_no_tab() {
        assert_eq!(zone_of("plan"), Some(Zone::Plan));
        assert_eq!(zone_of("retired"), None);
    }

    #[test]
    fn a_job_fleet_no_longer_lists_is_left_for_the_screen_to_pass_over() {
        let back = Kept::from_text(&kept().text());
        let listed = ["job-b"];
        let found = back.open.as_deref().filter(|open| listed.contains(open));
        assert_eq!(found, Some("job-b"));
        let gone = Kept::from_text("open\tjob-gone\ntop\tjob-gone\tplan");
        assert_eq!(gone.open.as_deref().filter(|open| listed.contains(open)), None);
    }

    #[test]
    fn a_thread_following_its_tail_keeps_no_place() {
        let ids = ["a", "b", "c"];
        assert_eq!(anchor_of(true, 1, 30., 0, |at| ids.get(at).map(|id| id.to_string())), None);
        let text = Kept { open: Some("j".into()), jobs: BTreeMap::from([("j".to_string(), Spot { top: None, row: None, scroll: anchor_of(true, 1, 30., 0, |at| ids.get(at).map(|id| id.to_string())) })]) }.text();
        assert!(!text.contains("scroll"), "{text}");
    }

    #[test]
    fn a_scrolled_thread_gets_its_top_back_by_the_item_and_how_far_into_it() {
        let ids = ["a", "b", "c", "d"];
        // The list has a Show earlier row above the items, so its element 3 is the item at 2.
        let anchor = anchor_of(false, 3, 41.6, 1, |at| ids.get(at).map(|id| id.to_string())).expect("scrolled up");
        assert_eq!(anchor, ("c".to_string(), 42));
        // Items put in front since: the same item is found further down.
        let now = ["x", "y", "a", "b", "c", "d"];
        assert_eq!(resolve(&anchor, |item| now.iter().position(|one| *one == item)), Some((4, 42)));
        // An item the thread no longer holds leaves the thread as it opens.
        assert_eq!(resolve(&anchor, |item| ["a", "b"].iter().position(|one| *one == item)), None);
    }

    #[test]
    fn a_top_above_the_items_or_past_them_keeps_no_row() {
        let ids = ["a", "b"];
        assert_eq!(anchor_of(false, 0, 9., 1, |at| ids.get(at).map(|id| id.to_string())), Some(("a".to_string(), 0)));
        assert_eq!(anchor_of(false, 7, 0., 0, |at| ids.get(at).map(|id| id.to_string())), None);
    }
}
