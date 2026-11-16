//! The To do strip over the message box: the first To do the agent listed, one at a time, so the thread and
//! the box stay in reach while it is open. Needs you no longer lists a To do (his word, 16 Nov).

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, KeyDownEvent, Role, SharedString};
use ipc::NeedsYouRow;

use super::dock::{is_todo, Thing};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook};

impl JobView {
    /// The Job's open To do items in Fleet's order, which is the order the agent listed them in.
    pub(super) fn todos<'a>(&'a self, id: &str, cx: &'a gpui::App) -> Vec<&'a NeedsYouRow> {
        let hidden = self.dock_of(id).map(|dock| &dock.hidden);
        self.needs_of(id, cx)
            .into_iter()
            .flat_map(|view| view.sections.iter().flat_map(|section| &section.rows))
            .filter(|row| is_todo(row) && !hidden.is_some_and(|hidden| hidden.contains(&row.id)))
            .collect()
    }

    /// The strip between the thread and the box: the first To do, its place among them where there is more than one,
    /// and its row as Needs you drew it, Done, the remove control and the steps under it.
    pub(super) fn render_todo_strip(&self, id: &SharedString, cx: &Context<Self>) -> Option<AnyElement> {
        let todos = self.todos(id, cx);
        let first = *todos.first()?;
        let look = cx.look();
        let place = (todos.len() > 1).then(|| {
            let said = SharedString::from(format!("1 of {}", todos.len()));
            labelled(div().id("todo-place").child(said.clone()), Role::Label, said)
        });
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_1))
            .text_size(look.length(token::TEXT_2XS))
            .line_height(look.length(token::LEADING_2XS))
            .text_color(look.color(token::FG_MUTED))
            .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child("To do"))
            .children(place);
        Some(
            labelled(
                div()
                    .id("todo-strip")
                    .flex_none()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .p(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .bg(look.color(token::BG_RAISED))
                    // The note box under the row is the strip's own: Esc puts it away, as it does in Needs you.
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        let open = this.shown.as_ref().and_then(|id| this.dock_of(id)).is_some_and(|dock| dock.row_box.is_some());
                        if event.keystroke.key == "escape" && open {
                            this.close_row_box(window, cx);
                            cx.stop_propagation();
                        }
                    }))
                    .child(head)
                    .child(self.thing_row(id, &Thing::Row(first), look, cx)),
                Role::Region,
                "To do",
            )
            .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::dock::Thing;
    use super::super::parts_fold::{drawn, drawn_like, redraw};
    use super::super::JobView;
    use gpui::{prelude::*, Role};
    use ipc::{NeedsYouAct, WaitingItem};

    const JOB: &str = "tool-edit";

    fn item(id: &str, text: &str, extra: &str) -> WaitingItem {
        let wire = format!(r#"{{"id":"{id}","text":"{text}","since":"2026-11-16T10:00:00Z","source":"agent"{extra}}}"#);
        ipc::decode::<WaitingItem>("item", wire.as_bytes()).expect("an item decodes")
    }

    fn todos() -> Vec<WaitingItem> {
        vec![
            item("restart", "Restart Fleet", r#","steps":["Quit Bridge","Run `scripts/restart`"]"#),
            item("grant", "Grant Local Network", ""),
            item("sign", "Sign in to GitHub", ""),
        ]
    }

    fn question() -> WaitingItem {
        item("pick", "Which colour", r#","act":{"kind":"answer","target":"pick"},"options":[{"label":"Blue"},{"label":"Green"}]"#)
    }

    /// The Job on screen with these items waiting on him, and nothing else of the scenario's.
    fn on_screen<'a>(cx: &'a mut gpui::TestAppContext, waiting: Vec<WaitingItem>) -> (gpui::Entity<JobView>, &'a mut gpui::VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let mut jobs = crate::mock::jobs("ready-to-try").expect("the fixture decodes");
        jobs.iter_mut().find(|job| job.id.as_str() == JOB).expect("the scenario holds the Job").waiting_for = waiting;
        let threads = crate::mock::threads("ready-to-try").expect("the threads decode");
        let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
        let (view, cx) = cx.add_window_view({
            let list = list.clone();
            move |window, cx| JobView::new(list, Some(threads), window, cx)
        });
        cx.simulate_resize(gpui::size(gpui::px(1400.), gpui::px(1200.)));
        list.update(cx, |list, cx| list.open(JOB.into(), cx));
        redraw(cx);
        (view, cx)
    }

    /// Draw again with the book emptied first: a test window has no label root to clear it, so a name once
    /// drawn would stand in it for good, and each frame of `redraw` adds its own.
    fn fresh(cx: &mut gpui::VisualTestContext) {
        redraw(cx);
        cx.update(|_, cx| cx.set_global(crate::labels::LabelBook::default()));
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    /// What the window drew under this name, each with the names it was drawn in.
    fn where_drawn(cx: &mut gpui::VisualTestContext, name: &str) -> Vec<(Role, Vec<gpui::SharedString>)> {
        cx.update(|window, cx| {
            let book = cx.try_global::<crate::labels::LabelBook>();
            book.map(|book| book.drawn(window.window_handle().window_id()).iter().filter(|one| one.name == name).map(|one| (one.role, one.path.clone())).collect()).unwrap_or_default()
        })
    }

    #[gpui::test]
    fn the_strip_shows_only_the_first_to_do_and_where_it_stands(cx: &mut gpui::TestAppContext) {
        let (_view, cx) = on_screen(cx, todos());
        assert!(drawn(cx, Role::Region, "To do"), "the strip stands over the box");
        assert!(drawn(cx, Role::Label, "1 of 3"));
        assert!(drawn(cx, Role::Button, "Open Restart Fleet"), "the first the agent listed");
        assert!(!drawn(cx, Role::Button, "Open Grant Local Network"), "one at a time");
        assert!(!drawn(cx, Role::Button, "Open Sign in to GitHub"));
        assert!(drawn(cx, Role::Button, "Done"));
    }

    #[gpui::test]
    fn one_to_do_alone_has_no_count(cx: &mut gpui::TestAppContext) {
        let (_view, cx) = on_screen(cx, vec![item("grant", "Grant Local Network", "")]);
        assert!(drawn(cx, Role::Region, "To do"));
        assert!(drawn_like(cx, Role::Label, "1 of").is_empty(), "no count for one");
    }

    #[gpui::test]
    fn done_on_the_strip_brings_the_next_to_do_and_the_last_takes_the_strip(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, todos());
        view.update_in(cx, |view, window, cx| view.done_row("restart", None, window, cx));
        fresh(cx);
        assert!(drawn(cx, Role::Button, "Open Grant Local Network"), "the next one shows");
        assert!(drawn(cx, Role::Label, "1 of 2"));
        assert!(!drawn(cx, Role::Button, "Open Restart Fleet"), "drawn at {:?}", where_drawn(cx, "Open Restart Fleet"));
        for id in ["grant", "sign"] {
            view.update_in(cx, |view, window, cx| view.done_row(id, None, window, cx));
            fresh(cx);
        }
        assert!(!drawn(cx, Role::Region, "To do"), "with none left the strip is gone");
    }

    #[gpui::test]
    fn needs_you_holds_no_to_do(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, todos().into_iter().chain([question()]).collect());
        view.read_with(cx, |view, cx| {
            let groups = view.groups(JOB, cx);
            assert!(!groups.is_empty(), "the question is still there");
            for group in groups {
                for thing in group.things {
                    assert!(!matches!(thing, Thing::Row(row) if matches!(row.act, NeedsYouAct::Done { .. })), "a To do is listed in {}", group.label);
                }
            }
        });
        view.update_in(cx, |view, window, cx| view.set_needs(true, None, window, cx));
        fresh(cx);
        assert!(drawn_like(cx, Role::Region, "To do").len() <= 1, "Needs you has no To do section beside the strip: {:?}", where_drawn(cx, "To do"));
        assert!(!drawn(cx, Role::Button, "Open Grant Local Network"), "the panel lists no To do");
    }

    #[gpui::test]
    fn to_dos_alone_raise_no_needs_you(cx: &mut gpui::TestAppContext) {
        let (_view, cx) = on_screen(cx, todos());
        assert!(!drawn(cx, Role::Button, "Needs you"), "nothing else waits, so no band");
        assert!(drawn(cx, Role::TextInput, "Message"), "the box is there to type in");
    }
}
