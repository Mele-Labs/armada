//! A Job view with the Operator's thread beside it over a stand-in Fleet, for the tests of what the Operator's thread offers.

use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, Context, Entity, Render, TestAppContext, VisualTestContext, Window};

use super::beside::OperatorPane;
use super::parts_fold::redraw;
use super::JobView;
use crate::fleet::FleetConnection;

/// The Operator's session, which holds no Job's record.
pub(super) const OPERATOR: &str = "operator-session";
/// The one message in the Operator's thread, so words can be selected in it.
pub(super) const OPERATOR_ROW: &str = "op-1";
/// The Job the view shows, whose record is a live one.
pub(super) const JOB: &str = "mock-01";

fn operator_thread() -> ipc::JobThread {
    let row = format!(r#"{{"kind":"row","row":{{"kind":"message","id":"{OPERATOR_ROW}","at":"2026-11-20T09:00:00Z","from":{{"kind":"you"}},"text":"hello"}}}}"#);
    let wire = format!(r#"{{"session_id":"{OPERATOR}","head":{{"trail":{{"started":"2026-11-20T08:00:00Z"}}}},"items":[{row}]}}"#);
    ipc::decode::<ipc::JobThread>("thread", wire.as_bytes()).expect("a thread")
}

/// His projects as Fleet would list them: each `(id, name, mac)`, with `mac` set on a copy another Mac keeps.
fn projects_of(held: &[(&str, &str, Option<&str>)]) -> ipc::Projects {
    let each: Vec<String> = held
        .iter()
        .map(|(id, name, mac)| {
            let mac = mac.map(|mac| format!(r#","mac":"{mac}""#)).unwrap_or_default();
            format!(r#"{{"id":"{id}","name":"{name}","rows":[],"members":[],"place":{{"place":"this_mac"}}{mac}}}"#)
        })
        .collect();
    let wire = format!(r#"{{"projects":[{}],"loose":[],"left_out":[]}}"#, each.join(","));
    ipc::decode::<ipc::Projects>("projects", wire.as_bytes()).expect("projects")
}

/// The window's root: the Job view, and over it the selection bar of the words a test names, drawn as the thread draws it.
pub(super) struct Stage {
    job: Entity<JobView>,
    /// Draws the bar a test asks for from the view: the Job's thread's, or the Operator's.
    pub bar: Option<Rc<dyn Fn(&mut JobView, &mut Context<JobView>) -> Option<AnyElement>>>,
}

impl Render for Stage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let bar: Option<AnyElement> = self.bar.clone().and_then(|draw| self.job.update(cx, |job, cx| draw(job, cx)));
        div().size_full().child(self.job.clone()).children(bar)
    }
}

/// The Job view showing a Job, the Operator's thread pinned beside it, and Fleet listing `projects`.
pub(super) fn open<'a>(cx: &'a mut TestAppContext, projects: &[(&str, &str, Option<&str>)]) -> (Entity<JobView>, &'a mut VisualTestContext) {
    let (view, _, cx) = staged(cx, projects);
    (view, cx)
}

/// As [`open`], with the window's root that draws a selection bar.
pub(super) fn staged<'a>(cx: &'a mut TestAppContext, projects: &[(&str, &str, Option<&str>)]) -> (Entity<JobView>, Entity<Stage>, &'a mut VisualTestContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
        crate::features::chosen::forget_all(cx);
    });
    let jobs = crate::mock::jobs("job-for-you").expect("the fixture decodes");
    let threads = crate::mock::threads("job-for-you").expect("the threads decode");
    let fleet = cx.new(FleetConnection::idle);
    fleet.update(cx, |fleet, _| {
        fleet.hold_threads(threads.into_values().chain([operator_thread()]));
        fleet.hold_projects(projects_of(projects));
    });
    let list = cx.new(|cx| crate::jobs::JobsList::mock_over(&jobs, fleet, cx));
    let (stage, cx) = cx.add_window_view({
        let list = list.clone();
        move |window, cx| Stage { job: cx.new(|cx| JobView::new(list, None, window, cx)), bar: None }
    });
    let view = stage.read_with(cx, |stage, _| stage.job.clone());
    cx.simulate_resize(gpui::size(gpui::px(1400.), gpui::px(900.)));
    list.update(cx, |list, cx| list.open(JOB.to_string().into(), cx));
    redraw(cx);
    view.update_in(cx, |view, window, cx| {
        let pane = OperatorPane {
            session: OPERATOR.into(),
            record: None,
            mock: None,
            raised: Vec::new(),
            draw: Rc::new(|_, _, _| gpui::Empty.into_any_element()),
            say: Rc::new(|_, _, _, _| {}),
            leave: Rc::new(|_, _| {}),
            room: gpui::px(200.),
        };
        view.pin_beside(pane, window, cx);
    });
    redraw(cx);
    (view, stage, cx)
}
