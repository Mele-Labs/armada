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
        assert_eq!(names, ["Sketch", "Diagram", "Takes", "Document", "Browser", "Terminal", "Features", "API Client"]);
        assert_eq!(rows.iter().map(|row| row.group).collect::<Vec<_>>(), [Some("Ask the agent for"), None, None, None, None, None, None, None]);
        assert_eq!(rows[2].says, "options side by side; you pick one");
        assert_eq!(rows[2].pick, Pick::Tag { kind: TagKind::Tool, id: "takes".into(), title: "Takes".into(), url: None });
    }

    #[test]
    fn each_tool_has_a_hue_and_a_glyph_no_other_tool_has() {
        let glyphs: Vec<&str> = ask_tools().iter().map(|tool| tool.glyph).collect();
        assert_eq!(glyphs, ["layout-template", "chart-network", "columns-2", "file-text", "globe", "monitor-play", "blocks", "arrow-right-left"]);
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
}
