//! What Now leads with: the act this second, then the last five before it, newest first, then the subagents at work. The
//! act this second is the agent's row (`now.rs`); this file is the five that came before it.

use gpui::{div, prelude::*, AnyElement, Role, SharedString};
use ipc::{RunningKind, RunningWork, SessionRecord, SessionRow, ThreadItem};

use super::now::{beat, lead, named, since, words};
use super::rows::mono;
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

/// One subagent or background task running in a Job, as Now lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Live {
    pub key: String,
    pub what: String,
    pub since: String,
    /// The subagent's type, or "shell" for a background task. None where there is none to say.
    pub kind: Option<String>,
    pub background: bool,
}

/// What runs in the Job: the subagents the thread says are running, as the pill at its foot counts them, else those the record's
/// `running` lists; then the record's background tasks. Nothing here, and the Job is quiet.
pub(super) fn live_work(items: &[ThreadItem], running: &[RunningWork]) -> Vec<Live> {
    let mut live: Vec<Live> = super::pinned::running_subagents(items)
        .into_iter()
        .map(|one| Live { key: format!("subagent:{}", one.id), what: one.task.clone(), since: one.since.as_str().to_string(), kind: one.agent_type.clone(), background: false })
        .collect();
    let listed = |kind: RunningKind| running.iter().enumerate().filter(move |(_, one)| one.kind == kind);
    if live.is_empty() {
        live.extend(listed(RunningKind::Subagent).map(|(at, one)| Live { key: format!("running:{at}"), what: one.what.clone(), since: one.since.as_str().to_string(), kind: None, background: false }));
    }
    live.extend(listed(RunningKind::BackgroundTask).map(|(at, one)| Live { key: format!("background:{at}"), what: one.what.clone(), since: one.since.as_str().to_string(), kind: Some("shell".into()), background: true }));
    live
}

/// The background tasks Now has room for under the subagents: its rows stay as many as the acts it keeps.
fn room_for_tasks(subagents: usize) -> usize {
    KEPT.saturating_sub(subagents)
}

impl JobView {
    /// What runs in the Job now, from the sources the pill at the thread's foot reads.
    pub(super) fn live_work_of(&self, id: &str, cx: &gpui::App) -> Vec<Live> {
        let items = self.thread(id, cx).map(|held| held.items.as_slice()).unwrap_or_default();
        live_work(items, self.record(id, cx).map(|job| job.running.as_slice()).unwrap_or_default())
    }

    /// Under the last five acts: each subagent running with what it does and its time running, then the background tasks that fit. No
    /// count beside them.
    pub(super) fn now_running(&self, id: &str, look: &Look, cx: &gpui::App) -> Option<AnyElement> {
        let live = self.live_work_of(id, cx);
        let (subagents, tasks): (Vec<Live>, Vec<Live>) = live.into_iter().partition(|one| !one.background);
        let tasks: Vec<Live> = tasks.into_iter().take(room_for_tasks(subagents.len())).collect();
        let list = |name: &'static str, rows: Vec<Live>, prefix: &'static str| {
            (!rows.is_empty()).then(|| {
                let rows = rows.into_iter().map(|one| {
                    let kind = one.kind.clone().map(|kind| div().flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(kind).into_any_element());
                    let last = [kind, since(look, &one.key, &one.since, "Running for")].into_iter().flatten().collect::<Vec<_>>();
                    let last = (!last.is_empty()).then(|| div().flex_none().flex().items_center().gap(look.length(token::SPACE_2)).children(last).into_any_element());
                    labelled(
                        named(&one.key, beat(look, lead(look, &one.key, "split", token::STATUS_RUNNING, "Running", true), words(look, one.what.clone(), token::FG_DEFAULT, false, false, false), last, None)),
                        Role::Group,
                        format!("{prefix}{}", one.what),
                    )
                    .into_any_element()
                });
                labelled(div().id(SharedString::from(format!("now-running:{name}"))).flex().flex_col().children(rows), Role::List, name).into_any_element()
            })
        };
        let rows: Vec<AnyElement> = list("Subagents", subagents, "").into_iter().chain(list("Background tasks", tasks, "Background task ")).collect();
        (!rows.is_empty()).then(|| div().id("now-live").flex().flex_col().children(rows).into_any_element())
    }

    /// The row Now leads with when no act is under way: when the Job went quiet, in the muted ink. None for a Job that is
    /// finished, and where there is no time to give.
    pub(super) fn now_idle(&self, id: &str, job: &SessionRecord, acts: &[Act], look: &Look, cx: &gpui::App) -> Option<AnyElement> {
        // A turn that ended with subagents or tasks still at work is not a Job with nothing running.
        if self.closed(id, cx) || !self.live_work_of(id, cx).is_empty() {
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

    /// He opens Now from + in the Job's column, in front, as a press of + does once the Job's panes are placed.
    fn open_now(view: &gpui::Entity<JobView>, job: &str, cx: &mut gpui::VisualTestContext) {
        use super::super::pane_tree as tree;
        cx.run_until_parked();
        view.update(cx, |view, cx| {
            let column = tree::holder_of(view.arrangement_of(job).expect("the Job is placed"), "artifacts").expect("the column").id.clone();
            view.rearrange_in(&job.to_string().into(), None, true, |at| tree::add_tab(at, &column, "now", tree::Extras::default()), cx);
        });
        super::super::parts_fold::redraw(cx);
    }

    /// His Jobs keep the column they were started with: For you, Open, Done and Deferred as tabs over Now. Opened so, the four tabs go out
    /// of it and Now gives way to what the Job produced. Now opened from + draws no lamp on a row of its own and no empty row with acts.
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
            assert!(super::super::pane_tree::has_kind(arrangement, "thread"), "the rest of his layout stays");
            assert!(!super::super::pane_tree::has_kind(arrangement, "now"), "the thread shows what the Job is doing");
            for made in ["artifacts", "pulls", "issues"] {
                assert!(super::super::pane_tree::has_kind(arrangement, made), "{made} is what the column shows");
            }
        });
        assert!(drawn(cx, gpui::Role::Group, "Step and Needs you"), "the band is there once");
        assert!(drawn_like(cx, gpui::Role::Tab, "Job column: For you").is_empty() && drawn_like(cx, gpui::Role::Tab, "Job column: Open").is_empty(), "and no second pane of tabs under it");
        open_now(&view, "mock-01", cx);
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

    fn work(kind: RunningKind, what: &str) -> RunningWork {
        RunningWork { kind, what: what.into(), since: ipc::Instant::carried("2026-11-20T10:00:00Z") }
    }

    #[test]
    fn what_runs_is_the_subagents_then_the_tasks_and_nothing_running_is_empty() {
        assert!(live_work(&[], &[]).is_empty());
        let running = [work(RunningKind::BackgroundTask, "cargo build"), work(RunningKind::Subagent, "Read the docs")];
        let live = live_work(&[], &running);
        assert_eq!(live.iter().map(|one| (one.what.as_str(), one.background)).collect::<Vec<_>>(), [("Read the docs", false), ("cargo build", true)], "a subagent first, a task under it");
        assert_eq!(room_for_tasks(2), 3);
        assert_eq!(room_for_tasks(7), 0, "tasks wait where the subagents fill the room");
    }

    /// A Job whose turn is over with a subagent still at work is not idle: Now lists the subagent, and says nothing of idling.
    #[gpui::test]
    fn a_job_with_its_turn_over_and_one_subagent_running_shows_the_subagent_and_not_idle(cx: &mut gpui::TestAppContext) {
        use super::super::column_strip::tests::drawn::{fresh, opened};
        use super::super::parts_fold::{drawn, drawn_like};
        // Every Job's turn is over, nothing is under way in it, and it was last heard from a while ago.
        let (view, cx) = opened(cx, "mock-02");
        let list = view.read_with(cx, |view, _| view.list.clone());
        list.update(cx, |list, cx| {
            let mut jobs = list.records().to_vec();
            for one in &mut jobs {
                if let Some(hosted) = one.hosted.as_mut() {
                    hosted.turn = ipc::SessionTurn::Idle;
                }
                if let Some(mark) = one.mark.as_mut() {
                    mark.state = ipc::JobState::Idle;
                    mark.lamp = ipc::JobLamp::Idle;
                }
                one.stopped = None;
                one.activity = None;
                one.running = Vec::new();
                one.last_heard_at = Some(ipc::Instant::carried("2026-11-20T23:15:00Z"));
            }
            list.replace_records(jobs, cx);
        });
        // A Job whose thread holds no subagent at work says since when.
        let mut quiet = None;
        for job in ["mock-02", "mock-03", "mock-04", "mock-10", "mock-12", "mock-05", "mock-06"] {
            list.update(cx, |list, cx| list.open(job.into(), cx));
            open_now(&view, job, cx);
            fresh(cx);
            if drawn_like(cx, gpui::Role::Label, "Idle since").len() == 1 {
                quiet = Some(job);
                break;
            }
        }
        let job = quiet.expect("a Job with nothing running says since when it went quiet");
        assert!(!drawn(cx, gpui::Role::Group, "Read the migration notes"));
        list.update(cx, |list, cx| {
            let mut jobs = list.records().to_vec();
            jobs.iter_mut().find(|one| one.id.as_str() == job).expect("the Job").running = vec![work(RunningKind::Subagent, "Read the migration notes")];
            list.replace_records(jobs, cx);
        });
        fresh(cx);
        assert!(drawn(cx, gpui::Role::Group, "Read the migration notes"), "the subagent has its row");
        assert!(drawn_like(cx, gpui::Role::Label, "Idle since").is_empty(), "and the Job is not idle");
        assert!(drawn(cx, gpui::Role::List, "Subagents"));
    }
}
