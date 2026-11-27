//! The thread's rows, each drawn as a view of its own. A pulse asks gpui for the next frame on behalf of the
//! view it is drawn in: drawn straight in `JobView`, every pulse of a working Job drew the whole thread again
//! each frame and held a core. In its own view a pulse draws its row again, and the rest come from the cache.
//!
//! A cached view is sized by its style alone, not by what it holds, so a row is drawn from the cache only at
//! the size it last came to: any change to the view draws every row whole again, and is measured.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use gpui::{
    div, prelude::*, AnyElement, AnyView, App, Bounds, Context, ElementId, Entity, GlobalElementId, InspectorElementId, LayoutId, Pixels,
    SharedString, Size, StyleRefinement, WeakEntity, Window,
};

use super::JobView;
use crate::labels::{kept, KeptView, LabelBook};

/// One row: `JobView::render_entry` drawn inside a view of its own.
pub(super) struct RowView {
    job: WeakEntity<JobView>,
    ix: usize,
    drawn: Rc<Cell<u64>>,
    /// A row of the Operator's thread, drawn with its parts in place (`beside`).
    side: bool,
}

impl Render for RowView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.drawn.set(self.drawn.get() + 1);
        let (ix, side) = (self.ix, self.side);
        self.job.update(cx, |job, cx| job.as_side(side, |job| job.render_entry(ix, window, cx))).ok().flatten().unwrap_or_else(|| div().into_any_element())
    }
}

struct Row {
    view: Entity<RowView>,
    kept: Rc<RefCell<KeptView>>,
    drawn: Rc<Cell<u64>>,
    /// Where it was last drawn whole: the size a frame from the cache gives it.
    size: Rc<Cell<Option<Size<Pixels>>>>,
}

/// The shown Job's row views, by place in the list.
#[derive(Default)]
pub(super) struct Rows {
    job: Option<SharedString>,
    held: HashMap<usize, Row>,
    /// The view was told it changed since it was last drawn.
    changed: bool,
    /// This frame draws every row whole.
    whole: bool,
    /// The views of the marks outside the thread that ask for frames of their own (`own_frames`).
    frames: RefCell<HashMap<SharedString, Entity<Frames>>>,
    /// The Operator's thread's rows (`beside`).
    pub(super) side: bool,
}

/// A mark that asks for frames of its own, such as a spinner, drawn as a view so its frames draw it alone.
pub(super) struct Frames {
    draw: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>,
}

impl Render for Frames {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        (self.draw.clone())(window, cx)
    }
}

impl Rows {
    /// The rows of the Operator's thread: each draws with its parts in place.
    pub(super) fn beside() -> Self {
        Rows { side: true, ..Rows::default() }
    }

    /// `JobView` was told something changed: its next frame draws the rows whole.
    pub(super) fn changed(&mut self) {
        self.changed = true;
    }

    /// At the top of each `JobView` draw.
    pub(super) fn begin(&mut self, shown: &Option<SharedString>) {
        self.whole = std::mem::take(&mut self.changed);
        if &self.job != shown {
            self.held.clear();
            self.job = shown.clone();
        }
    }
}

impl JobView {
    /// `draw`, a mark that animates outside the thread, in a view of its own: drawn in this view, each of its
    /// frames would tell this view it changed, and the thread would be drawn whole each frame.
    pub(super) fn own_frames(&self, key: impl Into<SharedString>, draw: impl Fn(&mut Window, &mut App) -> AnyElement + 'static, cx: &mut App) -> AnyElement {
        let draw: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement> = Rc::new(draw);
        let view = self.rows.frames.borrow_mut().entry(key.into()).or_insert_with(|| cx.new(|_| Frames { draw: draw.clone() })).clone();
        view.update(cx, |frames, _| frames.draw = draw);
        AnyView::from(view).into_any_element()
    }

    /// The list's row at `ix`: whole on a frame where the view changed, else from its row view's cache.
    pub(super) fn thread_row(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        // An overlay's labels cannot be put back from a cache (`JobsScreen::job_area`).
        let overlaid = cx.try_global::<LabelBook>().is_some_and(|book| book.overlaid(window.window_handle().window_id()));
        let job = cx.entity().downgrade();
        let side = self.rows.side;
        // A card the Operator raised is the page's, which changes with no word to this view: it is drawn each frame.
        if side && self.raised_at(ix) {
            return self.render_entry(ix, window, cx);
        }
        let row = self.rows.held.entry(ix).or_insert_with(|| {
            let drawn = Rc::new(Cell::new(0));
            Row { view: cx.new(|_| RowView { job: job.clone(), ix, drawn: drawn.clone(), side }), kept: Rc::default(), drawn, size: Rc::default() }
        });
        let size = row.size.get();
        let child = match size {
            Some(size) if !self.rows.whole && !overlaid => div()
                .w_full()
                .h(size.height)
                .child(kept(row.view.clone().cached(StyleRefinement::default().size_full()), row.kept.clone(), row.drawn.clone()))
                .into_any_element(),
            _ => AnyView::from(row.view.clone()).into_any_element(),
        };
        Slot { child, size: row.size.clone(), whole: size.is_none() || self.rows.whole || overlaid, job }.into_any_element()
    }
}

/// Notes the size of a row drawn whole, and where a row from the cache stands at another width (the
/// thread was resized), draws the rows whole on the next frame.
struct Slot {
    child: AnyElement,
    size: Rc<Cell<Option<Size<Pixels>>>>,
    whole: bool,
    job: WeakEntity<JobView>,
}

impl IntoElement for Slot {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Slot {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(&mut self, _id: Option<&GlobalElementId>, _inspector_id: Option<&InspectorElementId>, window: &mut Window, cx: &mut App) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.prepaint(window, cx);
        if self.whole {
            self.size.set(Some(bounds.size));
        } else if self.size.get().is_some_and(|size| size.width != bounds.size.width) {
            self.size.set(None);
            let job = self.job.clone();
            window.on_next_frame(move |_, cx| {
                job.update(cx, |_, cx| cx.notify()).ok();
            });
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut (),
        _prepaint: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}

#[cfg(test)]
mod tests {
    use gpui::{prelude::*, Entity, TestAppContext, VisualTestContext};

    use super::JobView;

    fn drawn(view: &Entity<JobView>, cx: &mut VisualTestContext) -> Vec<(usize, u64)> {
        let mut drawn: Vec<(usize, u64)> = view.read_with(cx, |view, _| view.rows.held.iter().map(|(ix, row)| (*ix, row.drawn.get())).collect());
        drawn.sort();
        drawn
    }

    #[gpui::test]
    fn a_pulse_in_one_row_draws_that_row_and_the_rest_come_from_the_cache(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let jobs = crate::mock::jobs("job-view").expect("the fixture decodes");
        let threads = crate::mock::threads("job-view").expect("the threads decode");
        let longest = threads.iter().max_by_key(|(_, thread)| thread.items.len()).map(|(id, _)| id.clone()).expect("a thread");
        let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
        let (view, cx) = cx.add_window_view(|window, cx| JobView::new(list.clone(), Some(threads), window, cx));
        cx.simulate_resize(gpui::size(gpui::px(1200.), gpui::px(900.)));
        list.update(cx, |list, cx| assert!(list.open_job(&longest, cx), "the Job opens"));
        cx.run_until_parked();
        let rows = drawn(&view, cx);
        let (shown, loaded) = view.read_with(cx, |view, cx| (view.shown.clone(), view.shown.as_ref().map(|id| (view.pages.contains_key(id), view.thread(id, cx).map(|held| held.items.len())))));
        assert!(rows.len() >= 3, "the thread draws its rows, each in a view of its own: {rows:?} {shown:?} {loaded:?}");
        // The thread follows its end: the last row is on screen.
        let pulsed = rows[rows.len() - 1].0;
        let row = view.read_with(cx, |view, _| view.rows.held[&pulsed].view.clone());
        let pulse = |cx: &mut VisualTestContext| {
            row.update(cx, |_, cx| cx.notify());
            cx.run_until_parked();
        };
        // The first frame after a whole one fills the cache of each row on screen; the rest were only measured.
        pulse(cx);
        let before = drawn(&view, cx);
        let first: std::collections::HashMap<usize, u64> = rows.into_iter().collect();
        let shown: Vec<usize> = before.iter().filter(|(ix, now)| first.get(ix).is_some_and(|then| now > then)).map(|(ix, _)| *ix).collect();
        assert!(shown.len() >= 2, "rows stand on screen: {shown:?}");
        for _ in 0..3 {
            pulse(cx);
        }
        for ((ix, then), (_, now)) in before.iter().zip(drawn(&view, cx)) {
            if *ix == pulsed {
                assert_eq!(now, then + 3, "the pulsing row is drawn each frame");
            } else {
                assert_eq!(now, *then, "row {ix} comes from the cache while another pulses");
            }
        }
        let before = drawn(&view, cx);
        view.update(cx, |_, cx| cx.notify());
        cx.run_until_parked();
        for ((ix, then), (_, now)) in before.iter().zip(drawn(&view, cx)).filter(|((ix, _), _)| shown.contains(ix)) {
            assert!(now > *then, "row {ix} is drawn whole when the view changes");
        }
    }
}
