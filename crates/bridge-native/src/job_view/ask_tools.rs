//! Armada's own tools, asked for from the message box: the + button in its bar, or `ask_for_tool` (⌘/), lists them over
//! the box, and the one picked goes in at the caret as a chip of its hue. Sent, the chip is a `tool` tag, and Fleet adds
//! the line that tells the agent which tool to call; his words stay as typed.
//!
//! The list is the box's own offer list, so ↑ and ↓ move, Enter and Tab pick, and Esc puts it away with the keys still in
//! the box.

use gpui::{prelude::*, AnyElement, Context, Role, SharedString, Window};
use ipc::ask_tools::{ask_tool, ask_tools};
use ipc::TagKind;

use super::offer::{Offer, Pick, Row};
use super::JobView;
use crate::labels::labelled;
use crate::look::{Look, Token};

/// The list's heading, and its name to a screen reader.
const ASK_FOR: &str = "Ask the agent for";

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
pub(super) fn rows() -> Vec<Row> {
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
        if self.composer.offer.as_ref().is_some_and(|offer| offer.label == ASK_FOR) {
            self.composer.offer = None;
        } else {
            let at = self.composer.input.read(cx).cursor();
            self.composer.offer = Some(Offer { label: ASK_FOR, rows: rows(), current: 0, from: at, to: at, query: String::new(), loading: false });
        }
        self.composer.input.update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    /// The + in the box's bar that lists the tools.
    pub(super) fn tools_button(&self, look: &Look, cx: &Context<Self>) -> AnyElement {
        let off = self.off_shown_to(super::composer::MESSAGE, cx);
        let open = self.composer.offer.as_ref().is_some_and(|offer| offer.label == ASK_FOR);
        let said: SharedString = off.clone().unwrap_or_else(|| "Ask the agent for a sketch, takes, a page".into());
        let key = if off.is_none() { key() } else { None };
        labelled(
            super::head::icon_frame(look, "ask-for-tool", "plus", open, off.is_some())
                .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(said.clone()).key_binding(key.clone().map(gpui_component::kbd::Kbd::new)).build(window, cx))
                .when(off.is_none(), |button| button.on_click(cx.listener(|this, _, window, cx| this.open_tools(window, cx)))),
            Role::Button,
            "Ask the agent for a tool",
        )
        .into_any_element()
    }
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
