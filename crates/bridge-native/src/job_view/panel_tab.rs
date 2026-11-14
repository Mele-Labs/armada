//! The tab a Job's side panel opens in the thread's pane: a file, a Check, a pull request, an issue, a
//! bay, a clash's file, a watch, a rule or a mod. The panel's state stays in the Job view
//! (`panes.panel`, one open at a time, so opening another takes the tab's place); this view only draws
//! what the Job view draws for it, and holds the width its pane has, which the pull request's file list
//! needs to know whether it fits beside the body.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{canvas, div, prelude::*, Context, Pixels, Render, WeakEntity, Window};

use super::JobView;

pub(super) struct PanelTab {
    pub(super) job_view: WeakEntity<JobView>,
    width: Rc<Cell<Pixels>>,
}

impl PanelTab {
    pub(super) fn new(job_view: WeakEntity<JobView>) -> Self {
        PanelTab { job_view, width: Rc::new(Cell::new(gpui::px(0.))) }
    }
}

impl Render for PanelTab {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (me, width) = (cx.entity_id(), self.width.clone());
        let held = width.get();
        let drawn = self.job_view.update(cx, |job, cx| job.render_panel_tab(held, window, cx)).ok();
        // The pane's width is read as it paints; a change draws this tab again with the new one.
        let measure = canvas(
            |bounds, _, _| bounds.size.width,
            move |_, wide, _, cx| {
                if width.replace(wide) != wide {
                    cx.notify(me);
                }
            },
        )
        .absolute()
        .size_full();
        div().size_full().relative().child(measure).children(drawn)
    }
}
