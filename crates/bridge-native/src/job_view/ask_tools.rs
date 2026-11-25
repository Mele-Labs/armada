//! Armada's own tools, asked for from the message box: the + button in its bar, or `ask_for_tool` (⌘/), lists them over
//! the box, and the one picked goes in at the caret as a chip of its hue. Sent, the chip is a `tool` tag, and Fleet adds
//! the line that tells the agent which tool to call; his words stay as typed.
//!
//! The list is the box's own offer list, so ↑ and ↓ move, Enter and Tab pick, and Esc puts it away with the keys still in
//! the box. → on API Client's row opens its own items in the same list (`requests.rs`), and ← with nothing searched goes back.

mod requests;

use gpui::{prelude::*, AnyElement, Context, Keystroke, Role, SharedString, Task, Window};
use ipc::ask_tools::{ask_tool, ask_tools};
use ipc::{Requests, TagKind};

use super::offer::{Held, Offer, Pick, Row};
pub(crate) use requests::{lead, search, Within};
use super::JobView;
use crate::labels::labelled;
use crate::look::{Look, Token};

/// The list's heading, and its name to a screen reader.
pub(crate) const ASK_FOR: &str = "Ask the agent for";

/// The heading of API Client's own items.
const ASK_FOR_API_CLIENT: &str = "Ask the agent for › API Client";

/// API Client's id in `ask_tools.toml`, which a request's chip takes its glyph and hue from.
pub(crate) const API_CLIENT: &str = "api-client";

/// What a request not yet made is called.
const NEW_REQUEST: &str = "New request";

/// Whether `label` is the tools list at either level.
fn is_tools(label: &str) -> bool {
    label == ASK_FOR || label == ASK_FOR_API_CLIENT
}

/// A request chip's words: its tool and the request, as `API Client · <name>`.
pub(crate) fn request_words(title: &str) -> String {
    format!("{} · {title}", ask_tool(API_CLIENT).map_or("API Client", |tool| tool.name))
}

/// The request's own name, from the words its chip reads.
pub(crate) fn request_title(words: &str) -> &str {
    words.strip_prefix(request_words("").as_str()).unwrap_or(words)
}

/// The key the registry gives `ask_for_tool`, for the button's tooltip.
fn key() -> Option<gpui::Keystroke> {
    let row = crate::generated::actions::act("ask_for_tool")?;
    let chord = crate::keymap::parse_chord(row.shortcut)?;
    let modifiers = gpui::Modifiers { control: chord.ctrl, alt: chord.alt, shift: chord.shift, platform: chord.meta, function: false };
    Some(gpui::Keystroke { modifiers, key: chord.key, key_char: None })
}

/// A tool's hue by its id; none for an id this build does not list.
pub(crate) fn hue(id: &str) -> Option<Token> {
    ask_tool(id).map(|tool| Token(tool.hue))
}

/// Each tool's id, glyph and hue as `look` draws it, for a chip drawn where no `Look` is held.
pub(crate) fn painted(look: &Look) -> Vec<(&'static str, &'static str, gpui::Hsla)> {
    ask_tools().iter().map(|tool| (tool.id, tool.glyph, look.color(Token(tool.hue)))).collect()
}

/// The list's rows, in the order `ask_tools.toml` lists them, the heading over the first.
pub(crate) fn rows() -> Vec<Row> {
    ask_tools()
        .iter()
        .enumerate()
        .map(|(at, tool)| Row {
            group: (at == 0).then_some(ASK_FOR),
            glyph: tool.glyph,
            name: tool.name.to_string(),
            says: tool.says.to_string(),
            pick: Pick::Tag { kind: TagKind::Tool, id: tool.id.to_string(), title: tool.name.to_string(), url: None },
        })
        .collect()
}

impl JobView {
    /// Open the tools over the box, the first marked, and put the keys in the box so the list takes them. Shut where it is
    /// open, so the button and its key open and shut it.
    pub(super) fn open_tools(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.reporting.is_some() || self.off_shown_to(super::composer::MESSAGE, cx).is_some() {
            return;
        }
        if self.composer.offer.as_ref().is_some_and(|offer| is_tools(offer.label)) {
            self.composer.offer = None;
        } else {
            let at = self.composer.input.read(cx).cursor();
            self.composer.offer = Some(Offer { label: ASK_FOR, rows: rows(), current: 0, from: at, to: at, query: String::new(), loading: false, within: None, held: Held::default() });
        }
        self.composer.input.update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    /// A key the tools list takes before the offer list does: → on API Client's row opens its items, and in them ← with nothing
    /// searched goes back and Backspace takes back a letter. True where taken.
    pub(super) fn tools_key(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        let Some(offer) = self.composer.offer.as_mut().filter(|offer| is_tools(offer.label)) else { return false };
        match (key, offer.within.is_some()) {
            ("right", false) => {
                let current = offer.current;
                return self.open_api_client_items(current, cx);
            }
            ("left", true) if offer.query.is_empty() => {
                let marked = rows().iter().position(|row| matches!(&row.pick, Pick::Tag { id, .. } if id == API_CLIENT)).unwrap_or_default();
                *offer = Offer { label: ASK_FOR, rows: rows(), current: marked, query: String::new(), loading: false, within: None, held: Held::default(), ..*offer };
                offer.keep_marked_in_view();
            }
            // The search holds the keys while the items are open: a letter taken back, and a caret that has nowhere to go.
            ("left", true) => {}
            ("backspace", true) => {
                offer.query.pop();
                Self::search_narrowed(offer);
            }
            // With nothing left to pick, Enter and Tab leave the list as it is.
            ("enter" | "tab", true) if offer.rows.is_empty() => {}
            _ => return false,
        }
        cx.notify();
        true
    }

    /// Open API Client's items over the tools list, from its row `at`: → on it, or a press. False where `at` is not that row.
    pub(super) fn open_api_client_items(&mut self, at: usize, cx: &mut Context<Self>) -> bool {
        let Some(offer) = self.composer.offer.as_mut().filter(|offer| offer.label == ASK_FOR && offer.within.is_none()) else { return false };
        if !offer.rows.get(at).is_some_and(|row| matches!(&row.pick, Pick::Tag { kind: TagKind::Tool, id, .. } if id == API_CLIENT)) {
            return false;
        }
        *offer = Offer { label: ASK_FOR_API_CLIENT, rows: Within::default().rows(""), current: 0, query: String::new(), loading: true, within: Some(Within::default()), held: Held::default(), ..*offer };
        self.want_requests(cx);
        cx.notify();
        true
    }

    /// A letter typed while API Client's items are open goes into the search, not the box. True where taken.
    pub(super) fn tools_typed(&mut self, keys: &Keystroke, cx: &mut Context<Self>) -> bool {
        let Some(offer) = self.composer.offer.as_mut().filter(|offer| offer.within.is_some()) else { return false };
        let Some(typed) = keys.key_char.as_deref().filter(|typed| !keys.modifiers.control && !keys.modifiers.platform && !typed.chars().any(char::is_control)) else {
            return false;
        };
        offer.query.push_str(typed);
        Self::search_narrowed(offer);
        cx.notify();
        true
    }

    /// The items the search names, the first marked.
    fn search_narrowed(offer: &mut Offer) {
        let Some(within) = offer.within.as_ref() else { return };
        offer.rows = within.rows(&offer.query);
        offer.current = 0;
        offer.keep_marked_in_view();
    }

    /// Read the Job's saved requests, off the main thread, for the items to list.
    fn want_requests(&mut self, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let read: Task<Result<Requests, String>> = if self.fleet.is_none() {
            Task::ready(Ok(crate::api_client::mock_read(&job)))
        } else {
            match self.port(cx) {
                Some(port) => crate::api_client::read(port, &job, cx),
                None => Task::ready(Err("Fleet is not connected".to_string())),
            }
        };
        cx.spawn(async move |this, cx| {
            let answer = read.await;
            let _ = this.update(cx, |this, cx| {
                if this.shown.as_ref() != Some(&job) {
                    return;
                }
                let Some(offer) = this.composer.offer.as_mut().filter(|offer| offer.within.is_some()) else { return };
                // A read that failed leaves New request alone.
                offer.within = Some(answer.map_or_else(|_| Within::default(), |read| Within::of(&read)));
                offer.loading = false;
                let held = offer.current;
                Self::search_narrowed(offer);
                offer.current = held.min(offer.rows.len().saturating_sub(1));
                cx.notify();
            });
        })
        .detach();
    }

    /// The + in the box's bar that lists the tools.
    pub(super) fn tools_button(&self, look: &Look, cx: &Context<Self>) -> AnyElement {
        let off = self.off_shown_to(super::composer::MESSAGE, cx);
        let open = self.composer.offer.as_ref().is_some_and(|offer| is_tools(offer.label));
        let said: SharedString = off.clone().unwrap_or_else(|| "Ask the agent for a sketch, takes, a page".into());
        let key = if off.is_none() { key() } else { None };
        // A press on the + is its own, so the list's wait to put itself away on a press outside is dropped.
        let away = self.composer.offer.as_ref().map(|offer| offer.held.away.clone());
        labelled(
            super::head::icon_frame(look, "ask-for-tool", "plus", open, off.is_some())
                .on_mouse_down(gpui::MouseButton::Left, move |_, _, _| {
                    if let Some(away) = &away {
                        away.set(false);
                    }
                })
                .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(said.clone()).key_binding(key.clone().map(gpui_component::kbd::Kbd::new)).build(window, cx))
                .when(off.is_none(), |button| button.on_click(cx.listener(|this, _, window, cx| this.open_tools(window, cx)))),
            Role::Button,
            "Ask the agent for a tool",
        )
        .into_any_element()
    }
}

/// The + as another box draws it, for a view that is not a Job's: `keyed` puts the act's key in its tooltip, where the
/// box's scope hears it.
pub(crate) fn plus_button<T: 'static>(
    look: &Look,
    open: bool,
    keyed: bool,
    dress: impl FnOnce(gpui::Stateful<gpui::Div>) -> gpui::Stateful<gpui::Div>,
    cx: &Context<T>,
    press: impl Fn(&mut T, &mut Window, &mut Context<T>) + 'static,
) -> AnyElement {
    let key = if keyed { key() } else { None };
    labelled(
        dress(super::head::icon_frame(look, "ask-for-tool", "plus", open, false))
            .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new("Ask the agent for a sketch, takes, a page").key_binding(key.clone().map(gpui_component::kbd::Kbd::new)).build(window, cx))
            .on_click(cx.listener(move |this, _: &gpui::ClickEvent, window, cx| press(this, window, cx))),
        Role::Button,
        "Ask the agent for a tool",
    )
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tools_are_listed_in_their_order_under_one_heading() {
        let rows = rows();
        let names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, ["Sketch", "Prototype", "Diagram", "Takes", "Document", "Browser", "Terminal", "Features", "API Client"]);
        assert_eq!(rows.iter().map(|row| row.group).collect::<Vec<_>>(), [Some("Ask the agent for"), None, None, None, None, None, None, None, None]);
        assert_eq!(rows[3].says, "options side by side; you pick one");
        assert_eq!(rows[3].pick, Pick::Tag { kind: TagKind::Tool, id: "takes".into(), title: "Takes".into(), url: None });
    }

    #[test]
    fn each_tool_has_a_hue_and_a_glyph_no_other_tool_has() {
        let glyphs: Vec<&str> = ask_tools().iter().map(|tool| tool.glyph).collect();
        assert_eq!(glyphs, ["layout-template", "app-window", "chart-network", "columns-2", "file-text", "globe", "monitor-play", "blocks", "arrow-right-left"]);
        let hues: std::collections::BTreeSet<&str> = ask_tools().iter().map(|tool| tool.hue).collect();
        assert_eq!(hues.len(), ask_tools().len(), "no two tools share a hue");
        assert!(hues.iter().all(|hue| hue.starts_with("--ask-")), "every hue is an ask tool's own, none a status's");
    }

    #[test]
    fn the_key_that_lists_the_tools_is_no_other_acts() {
        let act = crate::generated::actions::act("ask_for_tool").expect("the act is registered");
        assert_eq!((act.shortcut, act.verb), ("⌘/", "Ask the agent for a tool"));
        let same: Vec<&str> = crate::generated::actions::ACTIONS.iter().filter(|one| one.shortcut == "⌘/").map(|one| one.id).collect();
        assert_eq!(same, ["ask_for_tool"], "no other act has the chord");
    }

    use gpui::{point, Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext};

    use super::super::parts_fold::redraw;
    use crate::job_view::offer::mention_of;

    /// A Job on the mock Fleet with its box drawn, so the + is on screen and a click can land.
    fn open<'a>(cx: &'a mut TestAppContext) -> (Entity<JobView>, &'a mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let jobs = crate::mock::jobs("job-for-you").expect("the fixture decodes");
        let threads = crate::mock::threads("job-for-you").expect("the threads decode");
        let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
        let (view, cx) = cx.add_window_view({
            let list = list.clone();
            move |window, cx| JobView::new(list, Some(threads), window, cx)
        });
        cx.simulate_resize(gpui::size(gpui::px(1400.), gpui::px(900.)));
        list.update(cx, |list, cx| list.open("mock-01".to_string().into(), cx));
        redraw(cx);
        (view, cx)
    }

    /// The tools list open, as the + opens it.
    fn open_list(view: &Entity<JobView>, cx: &mut VisualTestContext) {
        view.update_in(cx, |view, window, cx| view.open_tools(window, cx));
        redraw(cx);
        assert!(view.read_with(cx, |view, _| view.composer.offer.as_ref().is_some_and(|offer| offer.label == ASK_FOR)), "the tools list is open");
    }

    fn api_client_at() -> usize {
        rows().iter().position(|row| matches!(&row.pick, Pick::Tag { id, .. } if id == API_CLIENT)).expect("API Client is listed")
    }

    fn mark(view: &Entity<JobView>, at: usize, cx: &mut VisualTestContext) {
        view.update(cx, |view, _| view.composer.offer.as_mut().expect("a list").current = at);
    }

    fn key(view: &Entity<JobView>, key: &'static str, cx: &mut VisualTestContext) -> bool {
        let taken = view.update_in(cx, |view, window, cx| view.offer_key(key, window, cx));
        redraw(cx);
        taken
    }

    /// The level the list is at: its heading, whether the items are open, the search, and the names of its rows.
    fn level(view: &Entity<JobView>, cx: &mut VisualTestContext) -> (&'static str, bool, String, Vec<String>) {
        view.read_with(cx, |view, _| {
            let offer = view.composer.offer.as_ref().expect("a list");
            (offer.label, offer.within.is_some(), offer.query.clone(), offer.rows.iter().map(|row| row.name.clone()).collect())
        })
    }

    /// The chips the box holds, read back as the mentions they send.
    fn chips(view: &Entity<JobView>, cx: &mut VisualTestContext) -> Vec<ipc::SessionTag> {
        view.read_with(cx, |view, cx| view.kept(cx).tags.iter().filter_map(|tag| mention_of(&tag.id, &tag.label)).collect())
    }

    fn type_in(view: &Entity<JobView>, typed: &str, cx: &mut VisualTestContext) -> bool {
        let keys = Keystroke { modifiers: Default::default(), key: typed.to_string(), key_char: Some(typed.to_string()) };
        view.update(cx, |view, cx| view.tools_typed(&keys, cx))
    }

    #[gpui::test]
    fn the_right_arrow_on_the_api_client_row_opens_its_items_and_on_no_other_row_does_not(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);
        open_list(&view, cx);
        for at in (0..rows().len()).filter(|at| *at != api_client_at()) {
            mark(&view, at, cx);
            assert!(!key(&view, "right", cx), "row {at} is not API Client's");
            let (label, within, query, names) = level(&view, cx);
            assert_eq!((label, within, query.as_str(), names.len()), (ASK_FOR, false, "", rows().len()), "→ left the tools list where it was");
        }
        mark(&view, api_client_at(), cx);
        assert!(key(&view, "right", cx));
        let (label, within, query, names) = level(&view, cx);
        assert_eq!((label, within, query.as_str()), (ASK_FOR_API_CLIENT, true, ""));
        assert_eq!(names, ["New request", "Health", "List users", "Create user"]);
    }

    #[gpui::test]
    fn the_left_arrow_with_nothing_searched_goes_back_to_the_tools_with_api_client_marked(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);
        open_list(&view, cx);
        mark(&view, api_client_at(), cx);
        key(&view, "right", cx);
        view.update(cx, |view, _| view.composer.offer.as_mut().expect("a list").current = 2);
        assert!(key(&view, "left", cx));
        let (label, within, query, names) = level(&view, cx);
        assert_eq!((label, within, query.as_str(), names.len()), (ASK_FOR, false, "", rows().len()));
        assert_eq!(view.read_with(cx, |view, _| view.composer.offer.as_ref().map(|offer| offer.current)), Some(api_client_at()), "the row he came in by is marked");
    }

    #[gpui::test]
    fn the_left_arrow_with_a_search_leaves_the_level_alone_and_backspace_takes_back_a_letter(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);
        open_list(&view, cx);
        mark(&view, api_client_at(), cx);
        key(&view, "right", cx);
        assert!(type_in(&view, "l", cx));
        assert!(type_in(&view, "i", cx));
        assert_eq!(level(&view, cx).3, ["List users"], "the letters narrowed the items");
        assert!(key(&view, "left", cx), "the search takes the key");
        let (label, within, query, names) = level(&view, cx);
        assert_eq!((label, within, query.as_str()), (ASK_FOR_API_CLIENT, true, "li"), "← did nothing to the level or the search");
        assert_eq!(names, ["List users"]);
        assert!(key(&view, "backspace", cx));
        assert_eq!(level(&view, cx).2, "l");
        key(&view, "backspace", cx);
        let (label, within, query, names) = level(&view, cx);
        assert_eq!((label, within, query.as_str(), names.len()), (ASK_FOR_API_CLIENT, true, "", 4), "an empty search lists every item again");
    }

    #[gpui::test]
    fn the_search_in_the_items_finds_new_request_by_its_words(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);
        open_list(&view, cx);
        mark(&view, api_client_at(), cx);
        key(&view, "right", cx);
        for typed in ["n", "e", "w"] {
            type_in(&view, typed, cx);
        }
        assert_eq!(level(&view, cx).3, ["New request"]);
        type_in(&view, "x", cx);
        assert!(level(&view, cx).3.is_empty(), "a word nothing has leaves no rows");
        assert!(key(&view, "enter", cx), "with nothing to pick, Enter is still the list's");
        assert!(view.read_with(cx, |view, _| view.composer.offer.is_some()), "and it leaves the list open");
    }

    #[gpui::test]
    fn enter_on_the_api_client_row_in_the_tools_list_gives_the_plain_tool_chip(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);
        open_list(&view, cx);
        mark(&view, api_client_at(), cx);
        assert!(key(&view, "enter", cx));
        assert!(view.read_with(cx, |view, _| view.composer.offer.is_none()), "the list is put away");
        assert_eq!(chips(&view, cx), [ipc::SessionTag { kind: TagKind::Tool, id: "api-client".into(), title: "API Client".into(), url: None }]);
    }

    #[gpui::test]
    fn enter_on_a_saved_request_in_the_items_gives_a_request_chip_that_reads_api_client_and_its_name(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);
        open_list(&view, cx);
        mark(&view, api_client_at(), cx);
        key(&view, "right", cx);
        key(&view, "down", cx);
        assert!(key(&view, "enter", cx));
        assert!(view.read_with(cx, |view, _| view.composer.offer.is_none()));
        assert_eq!(chips(&view, cx), [ipc::SessionTag { kind: TagKind::Request, id: "service/health".into(), title: "Health".into(), url: None }]);
        let label = view.read_with(cx, |view, cx| view.kept(cx).tags[0].label.clone());
        assert_eq!(label, "API Client · Health", "the chip reads its tool and the request");
    }

    #[gpui::test]
    fn a_press_on_the_api_client_row_opens_its_items_and_a_press_on_another_row_picks_it(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);
        open_list(&view, cx);
        // The list holds eight rows under a heading and scrolls, so a row is pressed where it shows: marked, it is scrolled into view.
        let at = |view: &Entity<JobView>, cx: &mut VisualTestContext, name: &str| {
            view.update(cx, |view, _| {
                let offer = view.composer.offer.as_mut().expect("a list");
                offer.current = offer.rows.iter().position(|row| row.name == name).expect("the row is listed");
                offer.keep_marked_in_view();
            });
            redraw(cx);
            // The book keeps every frame's elements, oldest first; the row's place is where the last frame drew it.
            cx.update(|window, cx| {
                let book = cx.global::<crate::labels::LabelBook>();
                book.drawn(window.window_handle().window_id()).iter().rev().find(|one| one.role == Role::ListItem && one.name.starts_with(name)).map(|one| one.shown.center())
            })
            .expect("the row is drawn")
        };
        let api = at(&view, cx, "API Client");
        cx.simulate_click(api, Modifiers::none());
        redraw(cx);
        let (label, within, query, names) = level(&view, cx);
        assert_eq!((label, within, query.as_str()), (ASK_FOR_API_CLIENT, true, ""), "the press opened the items");
        assert_eq!(names.first().map(String::as_str), Some("New request"));
        assert!(chips(&view, cx).is_empty(), "and picked nothing");

        let (view, cx) = open_fresh(view, cx);
        let sketch = at(&view, cx, "Sketch");
        cx.simulate_click(sketch, Modifiers::none());
        redraw(cx);
        assert!(view.read_with(cx, |view, _| view.composer.offer.is_none()), "a press on another row puts the list away");
        assert_eq!(chips(&view, cx), [ipc::SessionTag { kind: TagKind::Tool, id: "sketch".into(), title: "Sketch".into(), url: None }]);
    }

    /// Put the list away and open the tools again, from the top.
    fn open_fresh<'a>(view: Entity<JobView>, cx: &'a mut VisualTestContext) -> (Entity<JobView>, &'a mut VisualTestContext) {
        view.update(cx, |view, _| view.composer.offer = None);
        open_list(&view, cx);
        (view, cx)
    }

    #[gpui::test]
    fn a_press_outside_the_list_puts_it_away_and_a_press_on_the_plus_leaves_it(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);
        open_list(&view, cx);
        let plus = cx
            .update(|window, cx| cx.global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), Role::Button, "Ask the agent for a tool").map(|one| one.bounds.center()))
            .expect("the + is drawn");
        cx.simulate_mouse_down(plus, MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
        assert!(view.read_with(cx, |view, _| view.composer.offer.as_ref().is_some_and(|offer| !offer.held.away.get())), "the + took the press as its own: the list stays and nothing waits");

        cx.simulate_mouse_down(point(gpui::px(2.), gpui::px(2.)), MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
        assert!(view.read_with(cx, |view, _| view.composer.offer.is_none()), "a press anywhere else puts the list away");
    }
}
