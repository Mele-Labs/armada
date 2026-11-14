//! The menus a row opens: a right press on it, the ContextMenu key or Shift+F10 while it has focus.
//! A Fleet notice offers Teach and the three mutes; a command, a check and a draft offer Teach.
//! The menu opens at the pointer for a press and under the row for a key, and focus returns to
//! the row when it shuts.

use std::rc::Rc;

use gpui::{canvas, prelude::*, Context, Div, EventEmitter, FocusHandle, KeyDownEvent, MouseButton, MouseDownEvent, SharedString, Stateful, Window};
use ipc::{Instant, Notice, NoticeScope, SetNoticeMute};

use super::keys::Spot;
use super::{JobView, Page};
use crate::dropdown::{under, Entry};

/// What a menu runs when an entry is chosen.
pub(super) type Pick = Rc<dyn Fn(&mut JobView, &'static str, &mut Window, &mut Context<JobView>)>;

/// A person asked to teach Armada from this row: what it was, its words, when, and the notices it names.
#[derive(Clone, Debug)]
pub struct Teach {
    pub kind: &'static str,
    pub text: String,
    pub at: Option<Instant>,
    pub ids: Vec<String>,
}

impl EventEmitter<Teach> for JobView {}

/// Whether a key press is the one that opens a row's menu.
fn opens_menu(event: &KeyDownEvent) -> bool {
    let key = event.keystroke.key.as_str();
    key == "menu" || (key == "f10" && event.keystroke.modifiers.shift)
}

/// The mute choices and the id each carries.
const MUTES: [(&str, &str); 3] = [("job", "For job"), ("repo", "For repo"), ("hour", "For 1 hour")];

/// What each notice is called in the menu's heading, which reads "Disable <name>" (`NOTICE_NAMES`).
pub(super) fn disable_label(word: &str) -> Option<&'static str> {
    Some(match word {
        "behind_base" => "Disable slot behind notice",
        "pr_checks_failed" => "Disable checks failed",
        "pr_conflicts" => "Disable conflicts",
        "pr_dropped" => "Disable left the queue",
        "pr_unmergeable" => "Disable unmergeable",
        "pr_merged" => "Disable merged",
        "pr_auto_merge" => "Disable auto-merge on",
        "pr_ready_to_merge" => "Disable ready to merge",
        "pr_queued" => "Disable in the queue",
        "pr_handed" => "Disable pull request handed over",
        "pr_comments" => "Disable review comments",
        "slot_free" => "Disable slot free",
        "slot_kept" => "Disable slot kept",
        "build_moved" => "Disable build moved",
        "write_clash" => "Disable write clash",
        "slot_stranded" => "Disable slot stranded",
        "main_red" => "Disable main red",
        "rail_nudge" => "Disable rail nudge",
        "rail_step" => "Disable step instructions",
        "stop_nudge" => "Disable stop nudge",
        "usage_trip" => "Disable usage notice",
        "limit_wake" => "Disable limit wake",
        "sign_in_wake" => "Disable sign-in wake",
        "plan_approved" => "Disable plan approved",
        "auto_land_refused" => "Disable auto land refused",
        "retro_note" => "Disable retro note",
        "reply_claim" => "Disable reply claim",
        "context_handoff" => "Disable context handoff",
        "walk_failed" => "Disable walk failed",
        "workflow_changed" => "Disable workflow changed",
        "delivery_asked" => "Disable delivery question",
        "delivery_chosen" => "Disable delivery chosen",
        "watch_saw" => "Disable watch",
        "taught" => "Disable taught",
        "issue_lost" => "Disable issue lost",
        "document_sketch" => "Disable document sketch",
        "task_marked" => "Disable task mark",
        "lifecycle_rule" => "Disable rule",
        "lifecycles_clear" => "Disable nothing holds it open",
        "operator" => "Disable operator",
        "started_job_ended" => "Disable ended by its starter",
        "gate_closed" => "Disable a gate closed",
        "gate_hold" => "Disable a gate holds",
        "claims" => "Disable claims",
        _ => return None,
    })
}

/// The entries a Fleet notice's menu holds: Teach first, then a heading and the three mutes when
/// the notice names a kind that can be turned off.
pub(super) fn notice_entries(word: Option<&str>) -> Vec<Entry> {
    let mut entries = vec![Entry::Act { id: "teach", label: "Teach".into(), icon: Some("graduation-cap"), danger: false }];
    if let Some(heading) = word.and_then(disable_label) {
        entries.push(Entry::Rule);
        entries.push(Entry::Label(heading));
        entries.extend(MUTES.iter().map(|&(id, label)| Entry::Act { id, label: label.into(), icon: None, danger: false }));
    }
    entries
}

/// The menu of a row that only teaches.
pub(super) fn teach_entries() -> Vec<Entry> {
    vec![Entry::Act { id: "teach", label: "Teach".into(), icon: Some("graduation-cap"), danger: false }]
}

impl JobView {
    /// Give `row` a menu: a right press opens it at the pointer; the ContextMenu key or Shift+F10
    /// opens it under the row. `back` takes focus again when it shuts.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn menu_on(
        &self,
        row: Stateful<Div>,
        spot: &Spot,
        name: impl Into<SharedString>,
        back: FocusHandle,
        entries: Rc<dyn Fn() -> Vec<Entry>>,
        pick: Pick,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let name: SharedString = name.into();
        let (press_name, press_entries, press_pick, press_back) = (name.clone(), entries.clone(), pick.clone(), back.clone());
        let edges = spot.bounds.clone();
        let drawn = spot.bounds.clone();
        row.on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                let pick = press_pick.clone();
                this.open_pop(press_name.clone(), press_entries(), event.position, false, Some(press_back.clone()), move |this, id, window, cx| pick(this, id, window, cx), window, cx);
            }),
        )
        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
            if !opens_menu(event) {
                return;
            }
            cx.stop_propagation();
            let pick = pick.clone();
            this.open_pop(name.clone(), entries(), under(edges.get()), true, Some(back.clone()), move |this, id, window, cx| pick(this, id, window, cx), window, cx);
        }))
        .child(canvas(move |bounds, _, _| drawn.set(bounds), |_, _, _, _| {}).absolute().top_0().left_0().size_full())
    }

    /// The pick of a Fleet notice's menu: Teach, or a mute of the notice for the Job, its repository or an hour.
    pub(super) fn notice_pick(&self, row: &str, text: String, at: Option<Instant>, word: Option<String>) -> Pick {
        let row = row.to_string();
        Rc::new(move |this, id, window, cx| {
            if id == "teach" {
                this.teach_open(Teach { kind: "notice", text: text.clone(), at: at.clone(), ids: word.iter().cloned().collect() }, window, cx);
                return;
            }
            let Some(job) = this.shown.clone() else { return };
            let Some(notice) = word.as_deref().and_then(notice_of) else { return };
            let manifest = this.record(&job, cx).and_then(|record| record.manifest_id.as_ref().map(|id| id.as_str().to_string()));
            let scope = match id {
                "job" => NoticeScope::Job { session_id: job.to_string() },
                "hour" => NoticeScope::Hour { session_id: job.to_string() },
                "repo" => match manifest {
                    Some(manifest_id) => NoticeScope::Repo { manifest_id },
                    None => {
                        this.refuse_in(&job, &row, "This Job has no repository", cx);
                        return;
                    }
                },
                _ => return,
            };
            let body = ipc::encode(&SetNoticeMute { notice, scope, muted: true }).map_err(|why| format!("{why:?}"));
            let key = row.clone();
            let page_job = job.clone();
            this.post_with(
                job,
                "/notices/mute",
                body,
                move |this, why, cx| {
                    if let Some(page) = this.pages.get_mut(&page_job) {
                        page.cards.said.insert(key, why);
                    }
                    cx.notify();
                },
                window,
                cx,
            );
        })
    }

    /// The pick of a row that only teaches.
    pub(super) fn teach_pick(&self, kind: &'static str, text: String, at: Option<Instant>) -> Pick {
        Rc::new(move |this, id, window, cx| {
            if id == "teach" {
                this.teach_open(Teach { kind, text: text.clone(), at: at.clone(), ids: Vec::new() }, window, cx);
            }
        })
    }

    fn refuse_in(&mut self, job: &SharedString, key: &str, why: &'static str, cx: &mut Context<Self>) {
        if let Some(page) = self.pages.get_mut(job) {
            page.cards.said.insert(key.to_string(), why.into());
        }
        cx.notify();
    }

    /// The focus and edges of a row, kept on the Job's page between draws.
    pub(super) fn spot(&self, page: &Page, key: &str, cx: &Context<Self>) -> Spot {
        page.keys.spot(key, cx)
    }
}

/// The notice a stored word names.
fn notice_of(word: &str) -> Option<Notice> {
    ipc::decode::<Notice>("a notice", format!("\"{word}\"").as_bytes()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_notice_that_names_a_kind_offers_teach_then_the_three_mutes() {
        let entries = notice_entries(Some("pr_checks_failed"));
        let ids: Vec<_> = entries.iter().filter_map(|entry| match entry { Entry::Act { id, .. } => Some(*id), _ => None }).collect();
        assert_eq!(ids, ["teach", "job", "repo", "hour"]);
        assert!(matches!(entries[2], Entry::Label("Disable checks failed")));
    }

    #[test]
    fn a_notice_with_no_kind_offers_teach_alone() {
        assert_eq!(notice_entries(None).len(), 1);
        assert_eq!(notice_entries(Some("not_a_notice")).len(), 1);
    }

    #[test]
    fn every_notice_word_is_read_and_has_a_name() {
        for word in ["behind_base", "gate_hold", "pr_merged", "operator", "claims"] {
            assert!(notice_of(word).is_some(), "{word}");
            assert!(disable_label(word).is_some(), "{word}");
        }
    }
}
