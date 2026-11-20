//! What Now leads with: the act this second, then the last five before it, newest first, then the subagents at work. The
//! act this second is the agent's row (`now.rs`); this file is the five that came before it.

use gpui::{div, prelude::*, AnyElement, Role, SharedString};
use ipc::{SessionRecord, SessionRow};

use super::now::{beat, lead, named, since, words};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};

/// How many acts Now keeps behind the one under way.
pub(super) const KEPT: usize = 5;

/// One act that is over: the call's own words and when it began.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Act {
    pub id: String,
    pub said: String,
    pub at: String,
}

/// The last [`KEPT`] calls of the thread, newest first. The call still open is the act this second, so it is not one of them;
/// a call that started a subagent is that subagent's row, and a message sent to another Job is not an act of this one.
pub(super) fn last_acts(rows: &[SessionRow], open: bool) -> Vec<Act> {
    let calls = rows.iter().filter_map(|row| match row {
        SessionRow::Tool { subagent: Some(_), .. } | SessionRow::Tool { message: Some(_), .. } => None,
        SessionRow::Tool { id, at, text, description, .. } => {
            let said = match description.as_deref().map(str::trim).filter(|said| !said.is_empty()) {
                Some(said) => said.to_string(),
                None => super::calls::call_line(text, false),
            };
            Some(Act { id: id.clone(), said, at: at.as_str().to_string() })
        }
        _ => None,
    });
    let mut done: Vec<Act> = calls.collect();
    if open {
        done.pop();
    }
    done.into_iter().rev().take(KEPT).collect()
}

/// Since when the Job has been quiet: the last time it was heard from, else when its last act began. None where Fleet
/// gave neither, so Now has no time to say.
pub(super) fn idle_since(heard: Option<&str>, acts: &[Act]) -> Option<String> {
    let at = heard.or_else(|| acts.first().map(|act| act.at.as_str()))?;
    let at = chrono::DateTime::parse_from_rfc3339(at).ok()?;
    Some(format!("Idle since {}", at.with_timezone(&chrono::Local).format("%H:%M")))
}

impl JobView {
    /// The row Now leads with when no act is under way: when the Job went quiet, in the muted ink. None for a Job that is
    /// finished, and where there is no time to give.
    pub(super) fn now_idle(&self, id: &str, job: &SessionRecord, acts: &[Act], look: &Look, cx: &gpui::App) -> Option<AnyElement> {
        if self.closed(id, cx) {
            return None;
        }
        let said = idle_since(job.last_heard_at.as_ref().map(|at| at.as_str()), acts)?;
        Some(labelled(named("idle", div().flex().items_center().min_h(look.length(token::H_ROW)).child(words(look, said.clone(), token::FG_MUTED, false, false, false))), Role::Label, said).into_any_element())
    }

    /// The last five acts as rows, each with how long ago it began. None where the Job has done none.
    pub(super) fn now_last_acts(&self, acts: &[Act], look: &Look) -> Option<AnyElement> {
        if acts.is_empty() {
            return None;
        }
        let rows = acts.iter().map(|act| {
            let key = format!("act:{}", act.id);
            labelled(
                named(&key, beat(look, lead(look, &key, "check", token::FG_SUBTLE, "Done", false), words(look, act.said.clone(), token::FG_MUTED, false, false, false), since(look, &key, &act.at, "Began"), None)),
                Role::ListItem,
                act.said.clone(),
            )
            .into_any_element()
        });
        Some(labelled(div().id(SharedString::from("now-last-acts")).flex().flex_col().children(rows), Role::List, "Last acts").into_any_element())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::JobView;

    fn call(id: &str, text: &str) -> SessionRow {
        SessionRow::Tool { id: id.into(), at: ipc::Instant::carried("2026-11-20T10:00:00Z"), text: text.into(), description: None, subagent: None, image: None, message: None }
    }

    fn said(acts: &[Act]) -> Vec<&str> {
        acts.iter().map(|act| act.id.as_str()).collect()
    }

    #[test]
    fn now_keeps_the_last_five_acts_newest_first_and_leaves_the_open_call_to_the_act_this_second() {
        let rows: Vec<SessionRow> = (1..=8).map(|n| call(&format!("c{n}"), "Read /a")).collect();
        assert_eq!(said(&last_acts(&rows, false)), ["c8", "c7", "c6", "c5", "c4"]);
        assert_eq!(said(&last_acts(&rows, true)), ["c7", "c6", "c5", "c4", "c3"], "c8 is under way, so it is the act this second");
        assert_eq!(said(&last_acts(&rows[..2], false)), ["c2", "c1"], "fewer than five are all there are");
        assert!(last_acts(&[], true).is_empty());
    }

    #[test]
    fn an_act_says_the_calls_own_phrase_where_it_has_one() {
        let mut rows = vec![call("c1", "Read /a"), call("c2", "Read /b")];
        if let SessionRow::Tool { description, .. } = &mut rows[1] {
            *description = Some("Reads the sync code".into());
        }
        let acts = last_acts(&rows, false);
        assert_eq!(acts[0].said, "Reads the sync code", "the call's own phrase where it has one");
    }

    #[test]
    fn a_quiet_job_says_since_when_and_with_no_time_says_nothing() {
        let local = |at: &str| chrono::DateTime::parse_from_rfc3339(at).expect("a time").with_timezone(&chrono::Local).format("%H:%M").to_string();
        assert_eq!(idle_since(Some("2026-11-20T10:00:00Z"), &[]), Some(format!("Idle since {}", local("2026-11-20T10:00:00Z"))), "last heard from");
        let acts = last_acts(&[call("c1", "Read /a")], false);
        assert_eq!(idle_since(None, &acts), Some(format!("Idle since {}", local("2026-11-20T10:00:00Z"))), "else when its last act began");
        assert_eq!(idle_since(None, &[]), None, "no time to give, no words");
        assert_eq!(idle_since(Some("not a time"), &[]), None);
    }

    /// His Jobs keep the column they were started with: For you, Open, Done and Deferred as tabs over Now. Opened so, the four tabs go out
    /// of it and the Now below draws no lamp on a row of its own and no empty row with acts: its words, or nothing.
    #[gpui::test]
    fn a_job_kept_with_the_old_column_opens_without_the_four_tabs_and_with_no_lone_dot(cx: &mut gpui::TestAppContext) {
        use super::super::parts_fold::{drawn, drawn_like, redraw};
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            super::super::pane_store::write(&super::super::pane_store::job_key("mock-01"), &super::super::pane_tree::legacy_arrangement(), cx);
        });
        let jobs = crate::mock::jobs("job-workflow").expect("the fixture decodes");
        let threads = crate::mock::threads("job-workflow").expect("the threads decode");
        let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
        let (view, cx) = cx.add_window_view({
            let list = list.clone();
            move |window, cx| JobView::new(list, Some(threads), window, cx)
        });
        cx.simulate_resize(gpui::size(gpui::px(1400.), gpui::px(1200.)));
        list.update(cx, |list, cx| list.open("mock-01".into(), cx));
        redraw(cx);
        view.read_with(cx, |view, _| {
            let arrangement = view.arrangement_of("mock-01").expect("the Job is placed");
            for gone in ["for_you", "open", "done", "deferred"] {
                assert!(!super::super::pane_tree::has_kind(arrangement, gone), "{gone} is the band's now");
            }
            assert!(super::super::pane_tree::has_kind(arrangement, "now") && super::super::pane_tree::has_kind(arrangement, "thread"), "the rest of his layout stays");
        });
        assert!(drawn(cx, gpui::Role::Group, "Step and Needs you"), "the band is there once");
        assert!(drawn_like(cx, gpui::Role::Tab, "Job column: For you").is_empty() && drawn_like(cx, gpui::Role::Tab, "Job column: Open").is_empty(), "and no second pane of tabs under it");
        super::super::column_strip::tests::drawn::fresh(cx);
        let column: Vec<(gpui::Role, String)> = cx.update(|window, cx| {
            cx.global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().filter(|one| one.bounds.origin.x > gpui::px(872.)).map(|one| (one.role, one.name.to_string())).collect()
        });
        let has = |role: gpui::Role, prefix: &str| column.iter().any(|(one, name)| *one == role && name.starts_with(prefix));
        assert!(has(gpui::Role::Image, "On this call for") && has(gpui::Role::Image, "Began"), "Now says the act this second and the acts before it");
        for older in [(gpui::Role::Image, "Fleet"), (gpui::Role::Button, "Plan group"), (gpui::Role::Group, "Moving the Job"), (gpui::Role::Image, "Walk waiting on you"), (gpui::Role::Button, "Workflow ")] {
            assert!(!has(older.0, older.1), "{} is in its own tab, not in Now", older.1);
        }
        assert!(has(gpui::Role::Button, "Ask for an update") && has(gpui::Role::Button, "Plan"), "Now's acts stand at the right of its first row");
        assert!(!column.iter().any(|(role, name)| *role == gpui::Role::Image && matches!(name.as_str(), "Waiting on you" | "Drafting" | "Live" | "Failed")), "no lone dot");
        assert!(drawn_like(cx, gpui::Role::Button, "Chip reads").is_empty() && drawn_like(cx, gpui::Role::ListItem, "Read behind_by").is_empty(), "the plan's group and its tasks are not in Now: they have their own tabs");
    }

    /// Now's acts never stand on a row with nothing in it: on every Job, they are drawn only beside a row that says something.
    #[gpui::test]
    fn now_draws_its_acts_only_beside_a_row_with_words(cx: &mut gpui::TestAppContext) {
        use super::super::column_strip::tests::drawn::{fresh, opened};
        for job in ["mock-03", "mock-12", "mock-10", "mock-04", "mock-02"] {
            let (_, cx) = opened(cx, job);
            fresh(cx);
            let column: Vec<(gpui::Role, String)> = cx.update(|window, cx| {
                cx.global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().filter(|one| one.bounds.origin.x > gpui::px(872.)).map(|one| (one.role, one.name.to_string())).collect()
            });
            let has = |role: gpui::Role, prefix: &str| column.iter().any(|(one, name)| *one == role && name.starts_with(prefix));
            let words = has(gpui::Role::Label, "Idle since") || has(gpui::Role::Image, "Began") || has(gpui::Role::Image, "On this call") || column.iter().any(|(role, name)| *role == gpui::Role::Group && name != "Step and Needs you");
            assert!(!has(gpui::Role::Button, "Ask for an update") || words, "{job}: acts on an empty row: {column:?}");
        }
    }
}
