//! The side panes while a page is read: with the reader open the Jobs list and the right column fold to narrow rails, so
//! the page takes the room. **A fold is a state of the drawing, never of the arrangement**: nothing here is written to the
//! store, so each side keeps its width and its front tab, and closing the reader draws both as they were. A press on a rail
//! (or Enter or Space on it) opens its side again over the reader; the rail, Esc or a press on the reader shuts it.

use gpui::{div, prelude::*, px, AnyElement, App, Context, FocusHandle, KeyDownEvent, Role, SharedString, Stateful, Window};

use super::pane_tree::{self as tree, Dir, Node, Split, Tab};
use super::panes_place::{front_of, row_kind_of};
use super::row_tabs::RowKind;
use super::rows::tip;
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};

/// A side that folds to a rail.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Side {
    Left,
    Right,
}

/// What a rail says, by side and by whether its side stands open over the reader: its tooltip and its name.
pub(crate) fn rail_said(side: Side, open: bool) -> &'static str {
    match (side, open) {
        (Side::Left, false) => "Show the Jobs list",
        (Side::Left, true) => "Hide the Jobs list",
        (Side::Right, false) => "Show the right column",
        (Side::Right, true) => "Hide the right column",
    }
}

/// The frame of a rail: a column `--space-12` wide in the panel's own look, its one glyph at the top, the whole of it the press.
pub(crate) fn rail(look: &Look, id: &'static str, side: Side, open: bool, focus: &FocusHandle, window: &Window) -> Stateful<gpui::Div> {
    let (glyph, said) = (if side == Side::Left { "panel-left-open" } else { "panel-right" }, rail_said(side, open));
    let hovered = look.color(token::BG_HOVER);
    let frame = div()
        .id(id)
        .track_focus(focus)
        .tab_stop(true)
        .h_full()
        .flex_none()
        .w(look.length(token::SPACE_12))
        .flex()
        .flex_col()
        .items_center()
        .pt(look.length(token::SPACE_3))
        .bg(look.color(token::BG_RAISED))
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_t(look.length(token::BORDER_WIDTH) * 2.)
        .border_color(look.color(token::BORDER_DEFAULT))
        .cursor_pointer()
        .hover(move |style| style.bg(hovered))
        .tooltip(tip(said))
        .child(gpui::svg().path(crate::assets::icon_path(glyph)).size(look.length(token::SPACE_4)).text_color(look.color(if open { token::ACCENT } else { token::FG_MUTED })));
    crate::settings::view::ringed(look, frame, focus, window)
}

/// Whether a key presses a rail: Enter and Space.
pub(crate) fn presses(event: &KeyDownEvent) -> bool {
    !event.keystroke.modifiers.modified() && matches!(event.keystroke.key.as_str(), "enter" | "space")
}

/// Whether a key shuts the side opened over the reader.
pub(crate) fn shuts(event: &KeyDownEvent) -> bool {
    !event.keystroke.modifiers.modified() && event.keystroke.key == "escape"
}

impl JobView {
    /// Whether this tab, in front, is read in the reader: a page, a draft that proposes no change, or an artifact of parts.
    fn shows_reader(&self, id: &str, tab: &Tab, cx: &App) -> bool {
        let (Some(row), Some(key)) = (row_kind_of(&tab.kind), tab.row.as_deref()) else { return false };
        if self.row_tab_of(id, row, key).is_none() {
            return false;
        }
        match row {
            RowKind::Document => true,
            RowKind::Draft => self.thread(id, cx).and_then(|held| super::review::draft_row(&held.items, key)).is_some_and(|row| matches!(row, ipc::SessionRow::Draft { change: None, .. })),
            RowKind::Artifact => self.artifact_of(id, key).is_some_and(|one| !one.parts.is_empty()),
            _ => false,
        }
    }

    /// Whether a reader is in front in any pane of the open Job: the Jobs list and the right column fold while it is.
    pub(crate) fn reader_open(&self, cx: &App) -> bool {
        let Some(id) = self.shown.as_ref() else { return false };
        let arrangement = self.arrangement();
        tree::panes_of(&arrangement.root).into_iter().any(|pane| front_of(pane).is_some_and(|tab| self.shows_reader(id, tab, cx)))
    }

    /// The pane the focus view stands on while an artifact is in front: its front tab is a reader, no pane is zoomed and he has not
    /// left it by Esc or Back. Nothing else of the Job is drawn then, so the composer, the Needs you band and the pane's tab
    /// strip are gone with the other panes. A state of the drawing, never of the arrangement.
    pub(super) fn focus_pane<'a>(&self, arrangement: &'a tree::Arrangement, cx: &App) -> Option<&'a super::pane_tree::Pane> {
        let id = self.shown.as_ref()?;
        if self.zoomed().is_some() {
            return None;
        }
        let pane = tree::panes_of(&arrangement.root).into_iter().find(|pane| front_of(pane).is_some_and(|tab| self.shows_reader(id, tab, cx)))?;
        let tab = front_of(pane)?;
        (self.arranged.focus_left.as_deref() != Some(tab.id.as_str())).then_some(pane)
    }

    /// Esc or Back: the focus view ends, and the Job is drawn as he left it, the reader still in front.
    pub(crate) fn leave_focus(&mut self, cx: &mut Context<Self>) {
        let arrangement = self.arrangement().into_owned();
        let Some(tab) = self.focus_pane(&arrangement, cx).and_then(front_of).map(|tab| tab.id.clone()) else { return };
        self.arranged.focus_left = Some(tab);
        self.arranged.over = None;
        cx.notify();
    }

    /// The side he opened over the reader, where one is folded and so can be.
    pub(crate) fn over(&self, cx: &App) -> Option<Side> {
        self.arranged.over.filter(|_| self.reader_open(cx))
    }

    /// A press on a rail: its side opens over the reader, or shuts if it was open.
    pub(crate) fn press_rail(&mut self, side: Side, cx: &mut Context<Self>) {
        self.arranged.over = if self.arranged.over == Some(side) { None } else { Some(side) };
        cx.notify();
    }

    pub(crate) fn shut_over(&mut self, cx: &mut Context<Self>) {
        if self.arranged.over.take().is_some() {
            cx.notify();
        }
    }

    /// The root's split whose first side holds the reader and whose second side is the right column, while the reader is open and
    /// no pane is zoomed. Where he has no right column, or the reader stands in it, nothing here folds.
    pub(super) fn right_fold<'a>(&self, arrangement: &'a tree::Arrangement, cx: &App) -> Option<&'a Split> {
        let id = self.shown.as_ref()?;
        if self.zoomed().is_some() {
            return None;
        }
        let Node::Split(split) = &arrangement.root else { return None };
        if split.dir != Dir::Row {
            return None;
        }
        tree::panes_of(&split.a).into_iter().any(|pane| front_of(pane).is_some_and(|tab| self.shows_reader(id, tab, cx))).then_some(split)
    }

    /// The right column's rail.
    pub(super) fn right_rail(&mut self, id: &SharedString, window: &Window, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let focus = self.tab_handle(id, "rail:right", "rail", cx);
        let open = self.arranged.over == Some(Side::Right);
        let (press, keys) = (focus.clone(), focus.clone());
        labelled(
            rail(look, "rail-right", Side::Right, open, &focus, window)
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.press_rail(Side::Right, cx);
                    window.focus(&press, cx);
                }))
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    if presses(event) {
                        cx.stop_propagation();
                        this.press_rail(Side::Right, cx);
                        window.focus(&keys, cx);
                    }
                })),
            Role::Button,
            rail_said(Side::Right, open),
        )
        .into_any_element()
    }

    /// Esc on the side opened over the reader shuts it, and the keys go back to its rail.
    pub(super) fn shut_key(&mut self, id: &SharedString, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.arranged.over == Some(Side::Right) && shuts(event) {
            cx.stop_propagation();
            self.shut_over(cx);
            let focus = self.tab_handle(id, "rail:right", "rail", cx);
            window.focus(&focus, cx);
        } else if shuts(event) && self.arranged.focus_now && !window.context_stack().iter().any(|context| context.contains(crate::keymap::TEXT_BOX)) {
            // Nothing under it took Esc: it leaves the focus view.
            cx.stop_propagation();
            self.leave_focus(cx);
        }
    }

    /// The quiet way out of the focus view, at the reader's top left: back to the Job's own panes and tabs.
    pub(super) fn focus_back(&self, job: &SharedString, look: &Look, cx: &Context<Self>) -> AnyElement {
        let title = self.title_of(job, cx).map_or_else(|| "the Job".to_string(), |title| title.to_string());
        let said = SharedString::from(format!("Back to {title}"));
        let key = gpui::Keystroke::parse("escape").ok();
        labelled(
            div()
                .id("focus-back")
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .min_w_0()
                .h(look.length(token::H_CONTROL_SM))
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_MUTED))
                .cursor_pointer()
                .tab_stop(true)
                .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new("Back to the Job").key_binding(key.clone().map(gpui_component::kbd::Kbd::new)).build(window, cx))
                .child(super::rows::glyph(look, "arrow-left", super::rows::Size::Small, look.color(token::FG_MUTED)))
                .child(div().min_w_0().truncate().child(said.clone()))
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                    if presses(event) {
                        cx.stop_propagation();
                        this.leave_focus(cx);
                    }
                }))
                .on_click(cx.listener(|this, _, _, cx| this.leave_focus(cx))),
            Role::Button,
            said,
        )
        .into_any_element()
    }

    /// The right column laid over the reader, at the share it keeps: a scrim that shuts it, and the column on top.
    pub(super) fn right_over(&mut self, column: AnyElement, split: &Split, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let area = self.arranged.area.get().map_or(0., |area| area.w as f32);
        let share = (1. - split.ratio as f32).clamp(0.3, 0.6);
        let wide = if area > 0. { px(area * share) } else { px(480.) };
        div()
            .id("over-scrim")
            .absolute()
            .inset_0()
            .occlude()
            .on_click(cx.listener(|this, _, _, cx| this.shut_over(cx)))
            .child(div().id("over-right").absolute().top_0().bottom_0().right_0().w(wide).occlude().shadow(look.shadows(token::SHADOW_OVERLAY)).child(column))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job_view::pane_store;

    #[test]
    fn a_rail_says_which_side_and_whether_it_is_open() {
        assert_eq!(rail_said(Side::Left, false), "Show the Jobs list");
        assert_eq!(rail_said(Side::Right, true), "Hide the right column");
    }

    #[gpui::test]
    fn a_reader_in_front_folds_both_sides_and_closing_it_leaves_the_arrangement_as_it_was(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let jobs = crate::mock::jobs("reader").expect("the fixture decodes");
        let threads = crate::mock::threads("reader").expect("the threads decode");
        let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
        let (view, cx) = cx.add_window_view({
            let list = list.clone();
            move |window, cx| JobView::new(list, Some(threads), window, cx)
        });
        list.update(cx, |list, cx| list.open("draft-job".into(), cx));
        cx.run_until_parked();
        let before = view.read_with(cx, |view, cx| (view.reader_open(cx), view.arrangement().into_owned()));
        assert!(!before.0, "no reader is open, so nothing is folded");

        view.update_in(cx, |view, window, cx| view.open_document_tab("design", window, cx));
        cx.run_until_parked();
        let open = view.read_with(cx, |view, cx| (view.reader_open(cx), view.arrangement().into_owned()));
        assert!(open.0, "a page in front is a reader open");
        let kept = |cx: &mut gpui::VisualTestContext| {
            let task = cx.update(|_, cx| pane_store::read(&pane_store::job_key("draft-job"), cx));
            cx.foreground_executor().block_test(task)
        };
        let stored = kept(cx);

        // A press on a rail opens its side over the reader, and another shuts it: the arrangement and the store are not touched.
        view.update(cx, |view, cx| view.press_rail(Side::Left, cx));
        assert_eq!(view.read_with(cx, |view, cx| view.over(cx)), Some(Side::Left));
        view.update(cx, |view, cx| view.press_rail(Side::Right, cx));
        assert_eq!(view.read_with(cx, |view, cx| view.over(cx)), Some(Side::Right), "one side opens at a time");
        cx.run_until_parked();
        assert_eq!(view.read_with(cx, |view, _| view.arrangement().into_owned()), open.1, "folding changes no pane");
        assert_eq!(kept(cx), stored, "the store is not rewritten by a fold");

        // The reader closes: nothing stays folded, and no side is left open.
        view.update_in(cx, |view, window, cx| {
            let tab = view.arrangement().into_owned();
            let id = tree::panes_of(&tab.root).into_iter().flat_map(|pane| pane.tabs.iter()).find(|tab| tab.kind == "document").map(|tab| tab.id.clone()).expect("the page's tab");
            view.close_tab_id(&id, window, cx);
        });
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        let (closed, over, now) = view.read_with(cx, |view, cx| (!view.reader_open(cx), view.over(cx), view.arrangement().into_owned()));
        assert!(closed && over.is_none(), "closing the reader restores both sides");
        assert_eq!((now.root, now.focus), (before.1.root, before.1.focus), "the panes are as they were before the reader opened");
    }

    /// The artifact-parts Job on screen under the label root, its thread read and drawn.
    fn artifact_open<'a>(cx: &'a mut gpui::TestAppContext) -> (gpui::Entity<JobView>, &'a mut gpui::VisualTestContext) {
        crate::job_view::parts_fold::rooted_job(cx, "artifact-parts", crate::mock_parts::JOB, gpui::size(gpui::px(1280.), gpui::px(800.)), |_| {})
    }

    fn drawn_now(cx: &mut gpui::VisualTestContext) {
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    use crate::job_view::parts_fold::drawn;
    use gpui::Role;

    #[gpui::test]
    fn an_artifact_in_front_is_drawn_alone_and_leaving_it_draws_the_job_as_he_left_it(cx: &mut gpui::TestAppContext) {
        let (view, cx) = artifact_open(cx);
        let job = crate::mock_parts::JOB;
        let back = view.read_with(cx, |view, cx| format!("Back to {}", view.title_of(job, cx).expect("the Job has a title")));
        drawn_now(cx);
        assert!(drawn(cx, Role::TextInput, "Message") && drawn(cx, Role::TabList, "Tabs"), "the Job is drawn whole before the artifact opens");
        assert!(!drawn(cx, Role::Button, &back), "there is no way back from a Job that shows no artifact");

        view.update_in(cx, |view, window, cx| assert!(view.open_named(crate::mock_parts::ID, window, cx)));
        drawn_now(cx);
        let open = view.read_with(cx, |view, _| view.arrangement().into_owned());
        let kept = {
            let task = cx.update(|_, cx| pane_store::read(&pane_store::job_key(job), cx));
            cx.foreground_executor().block_test(task)
        };
        assert!(drawn(cx, Role::Button, &back), "a quiet way back names the Job");
        assert!(!drawn(cx, Role::TextInput, "Message"), "the composer folds away");
        assert!(!drawn(cx, Role::TabList, "Tabs"), "the pane's tab strip folds away");
        assert!(drawn(cx, Role::Region, &format!("Reader: {}, Revision 2", crate::mock_parts::TITLE)), "the artifact is what is left");

        view.update(cx, |view, cx| view.leave_focus(cx));
        drawn_now(cx);
        assert!(!drawn(cx, Role::Button, &back), "the way back is gone");
        assert!(drawn(cx, Role::TabList, "Tabs"), "the pane's tab strip is back");
        assert!(drawn(cx, Role::Button, rail_said(Side::Right, false)), "the right column is a rail again");
        assert!(view.read_with(cx, |view, cx| view.reader_open(cx)), "the artifact is still the tab in front");
        assert_eq!(view.read_with(cx, |view, _| view.arrangement().into_owned()), open, "the panes and their tabs are as he left them");
        let task = cx.update(|_, cx| pane_store::read(&pane_store::job_key(job), cx));
        assert_eq!(cx.foreground_executor().block_test(task), kept, "going in and out wrote no arrangement");
    }

    #[gpui::test]
    fn escape_and_the_back_press_both_leave_the_focus_view_and_a_second_escape_does_nothing_more(cx: &mut gpui::TestAppContext) {
        let (view, cx) = artifact_open(cx);
        view.update_in(cx, |view, window, cx| assert!(view.open_named(crate::mock_parts::ID, window, cx)));
        drawn_now(cx);
        assert!(view.read_with(cx, |view, _| view.arranged.focus_now), "the focus view is up");
        cx.simulate_keystrokes("escape");
        drawn_now(cx);
        assert!(!view.read_with(cx, |view, _| view.arranged.focus_now), "Esc leaves it");
        assert!(view.read_with(cx, |view, cx| view.reader_open(cx)), "the artifact is still open");
        cx.simulate_keystrokes("escape");
        drawn_now(cx);
        assert!(view.read_with(cx, |view, cx| view.reader_open(cx)), "a second Esc closes nothing");

        // A press on Back does the same, after the artifact is opened afresh by its tab.
        view.update_in(cx, |view, window, cx| {
            view.arranged.focus_left = None;
            cx.notify();
            let _ = window;
        });
        drawn_now(cx);
        let back = view.read_with(cx, |view, cx| format!("Back to {}", view.title_of(crate::mock_parts::JOB, cx).expect("a title")));
        let at = cx.update(|window, cx| cx.global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), Role::Button, &back).map(|one| one.bounds.center())).expect("Back is drawn");
        cx.simulate_click(at, gpui::Modifiers::none());
        drawn_now(cx);
        assert!(!view.read_with(cx, |view, _| view.arranged.focus_now), "pressing Back leaves it");
    }

    #[gpui::test]
    fn a_reader_pins_its_acts_at_the_foot_while_the_focus_view_is_up(cx: &mut gpui::TestAppContext) {
        let (view, cx) = crate::job_view::parts_fold::rooted_job(cx, "reader", "draft-job", gpui::size(gpui::px(1280.), gpui::px(800.)), |_| {});
        view.update_in(cx, |view, window, cx| view.open_document_tab("design", window, cx));
        drawn_now(cx);
        let find = |cx: &mut gpui::VisualTestContext, role: Role, name: &str| {
            cx.update(|window, cx| cx.global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).map(|one| one.bounds)).unwrap_or_else(|| panic!("{name} was not drawn"))
        };
        let (foot, notes) = (find(cx, Role::Group, "Reader acts"), find(cx, Role::Switch, "Notes"));
        let named = crate::job_view::parts_fold::drawn_like(cx, Role::Region, "Reader: ").pop().expect("a reader is drawn");
        let region = find(cx, Role::Region, &named);
        assert!(foot.bottom() >= region.bottom() - gpui::px(1.), "the acts sit at the foot of the reader");
        assert!(notes.top() >= foot.top() && notes.bottom() <= foot.bottom(), "the notes toggle is one of them");

        view.update(cx, |view, cx| view.leave_focus(cx));
        drawn_now(cx);
        assert!(!drawn(cx, Role::Group, "Reader acts"), "out of the focus view they are back in the header");
        let (notes, region) = (find(cx, Role::Switch, "Notes"), find(cx, Role::Region, &named));
        assert!(notes.top() < region.top() + gpui::px(120.), "the toggle is at the head again");
    }

    #[gpui::test]
    fn an_artifact_of_parts_pins_what_settles_it_below_the_scrolling_column(cx: &mut gpui::TestAppContext) {
        let (view, cx) = artifact_open(cx);
        view.update_in(cx, |view, window, cx| assert!(view.open_named(crate::mock_parts::ID, window, cx)));
        drawn_now(cx);
        let find = |cx: &mut gpui::VisualTestContext, role: Role, name: &str| {
            cx.update(|window, cx| cx.global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).map(|one| one.bounds)).unwrap_or_else(|| panic!("{name} was not drawn"))
        };
        let (dismiss, region) = (find(cx, Role::Button, "Dismiss"), find(cx, Role::Region, &format!("Reader: {}, Revision 2", crate::mock_parts::TITLE)));
        assert!(dismiss.bottom() <= gpui::px(800.) && dismiss.bottom() > gpui::px(760.), "Dismiss sits at the foot of the window: {dismiss:?}");
        assert!(region.bottom() > dismiss.bottom(), "the card runs on below it, so it is pinned and not in the flow: {region:?}");
    }
}
