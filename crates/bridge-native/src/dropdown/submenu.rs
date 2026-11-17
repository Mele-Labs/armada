//! A menu row that opens acts beside it, attached to it: the settings picker's project being edited, with Merge and
//! Delete. The row draws tinted with a bar and a chevron, the submenu sits flush against the menu with its top-left
//! corner square and its first entry level with the row, and a tint runs across the join so the row reads as
//! continuing into it. → enters from the row, ← and Esc go back, ↑ and ↓ move inside, ↩ picks.
//!
//! Only acts and rows stand in a submenu. The row's place is worked out from the entries' heights, which a row with
//! a second line, or a menu scrolled by its length, does not keep.

use gpui::{div, prelude::*, px, svg, AnyElement, Context, Div, KeyDownEvent, Pixels, Role, SharedString, Stateful};
use ipc::MacColour;

use super::{Dropdown, Entry, Picked, Row};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};
use gpui::DismissEvent;

/// A row's chip and submenu.
#[derive(Clone, Default)]
pub struct Beside {
    /// A chip at the right: a Mac's name in its colour, or neutral where the row is this Mac's own.
    chip: Option<(SharedString, Option<MacColour>)>,
    /// The acts that open beside the row. None, or none left, is a plain row.
    entries: Vec<Entry>,
    /// The label and glyph in the accent: an act offered beside the plain ones.
    accent: bool,
    /// Where the keys are in the submenu, kept on the row it opens beside.
    keys: Keys,
}

impl Row {
    pub fn chip(mut self, name: impl Into<SharedString>, colour: Option<MacColour>) -> Row {
        self.beside.chip = Some((name.into(), colour));
        self
    }

    pub fn submenu(mut self, entries: Vec<Entry>) -> Row {
        self.beside.entries = entries;
        self
    }

    pub fn accent(mut self) -> Row {
        self.beside.accent = true;
        self
    }

    /// The acts beside the row, empty for none.
    #[cfg(test)]
    pub fn acts(&self) -> &[Entry] {
        &self.beside.entries
    }

    /// The chip's words, and its Mac's colour where the row is another Mac's.
    #[cfg(test)]
    pub fn chip_of(&self) -> Option<(&str, Option<MacColour>)> {
        self.beside.chip.as_ref().map(|(name, colour)| (name.as_ref(), *colour))
    }
}

/// Where the keys are in the submenu.
#[derive(Clone, Copy, Default)]
pub struct Keys {
    inside: bool,
    cursor: Option<usize>,
}

impl Dropdown {
    /// The row whose submenu is open, with its entries.
    fn opened(&self) -> Option<(usize, &[Entry])> {
        self.entries.iter().enumerate().find_map(|(at, entry)| match entry {
            Entry::Row(Row { beside, .. }) if !beside.entries.is_empty() => Some((at, beside.entries.as_slice())),
            _ => None,
        })
    }

    /// Where the keys are in the open submenu.
    fn sub(&self) -> Keys {
        self.entries.iter().find_map(|entry| match entry {
            Entry::Row(Row { beside, .. }) if !beside.entries.is_empty() => Some(beside.keys),
            _ => None,
        }).unwrap_or_default()
    }

    fn sub_mut(&mut self) -> Option<&mut Keys> {
        self.entries.iter_mut().find_map(|entry| match entry {
            Entry::Row(Row { beside, .. }) if !beside.entries.is_empty() => Some(&mut beside.keys),
            _ => None,
        })
    }

    fn sub_pickable(&self) -> Vec<usize> {
        self.opened().map(|(_, entries)| entries.iter().enumerate().filter(|(_, entry)| entry.pickable()).map(|(at, _)| at).collect()).unwrap_or_default()
    }

    fn sub_step(&mut self, by: isize) {
        let ids = self.sub_pickable();
        if ids.is_empty() {
            return;
        }
        let now = self.sub().cursor.and_then(|at| ids.iter().position(|&id| id == at)).unwrap_or(0);
        let next = ids[(now as isize + by).clamp(0, ids.len() as isize - 1) as usize];
        if let Some(keys) = self.sub_mut() {
            keys.cursor = Some(next);
        }
    }

    /// A pick in the submenu shuts the whole menu, as a pick in the menu does.
    fn pick_sub(&mut self, at: usize, cx: &mut Context<Self>) {
        let id = match self.opened().and_then(|(_, entries)| entries.get(at)) {
            Some(Entry::Act { id, .. }) | Some(Entry::Item { id, off: None, .. }) | Some(Entry::Row(Row { id, off: None, .. })) => *id,
            _ => return,
        };
        cx.emit(Picked::Entry { id, keep: false });
        cx.emit(DismissEvent);
    }

    /// The keys while a row has a submenu: → on its row goes in, and inside, ← and Esc come out, ↑ ↓ move, ↩ picks.
    /// `true` where the key was taken.
    pub(super) fn sub_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) -> bool {
        let held = event.keystroke.modifiers;
        let Some((row, _)) = self.opened() else { return false };
        if held.platform || held.control || held.alt {
            return false;
        }
        let taken = match (event.keystroke.key.as_str(), self.sub().inside) {
            ("right", false) if self.cursor == Some(row) => {
                let first = self.sub_pickable().first().copied();
                if let Some(keys) = self.sub_mut() {
                    *keys = Keys { inside: first.is_some(), cursor: first };
                }
                first.is_some()
            }
            ("left" | "escape", true) => {
                if let Some(keys) = self.sub_mut() {
                    *keys = Keys::default();
                }
                true
            }
            ("down", true) => {
                self.sub_step(1);
                true
            }
            ("up", true) => {
                self.sub_step(-1);
                true
            }
            ("enter", true) => {
                if let Some(at) = self.sub().cursor {
                    self.pick_sub(at, cx);
                }
                true
            }
            _ => false,
        };
        if taken {
            cx.stop_propagation();
            cx.notify();
        }
        taken
    }

    /// The row the submenu opens beside: tinted, a bar at its left, its right edge square; and a Mac's chip and a
    /// chevron after what the row says. Every row of a menu that has a submenu keeps the chevron's width, so the
    /// chips stand in one column.
    pub(super) fn dress(&self, line: Stateful<Div>, row: &Row, look: &Look, glyph: Pixels) -> Stateful<Div> {
        let open = !row.beside.entries.is_empty();
        let chevron = self.opened().is_some();
        let chip = row.beside.chip.clone().map(|(name, colour)| {
            let hue = match colour {
                Some(colour) => crate::settings::macs::colour_of(look, colour),
                None => look.color(token::FG_MUTED),
            };
            div().flex_none().child(crate::pairing::pill_in(look, SharedString::from(format!("chip-{}", row.id)), &name, hue))
        });
        let tint = look.color(token::ACCENT_MUTED);
        line.when(open, |line| {
            line.relative().rounded_tr(px(0.)).rounded_br(px(0.)).bg(tint).hover(|style| style.bg(tint)).child(
                div()
                    .absolute()
                    .left_0()
                    .top(look.length(token::SPACE_1))
                    .bottom(look.length(token::SPACE_1))
                    .w(look.length(token::BORDER_WIDTH) * 2.)
                    .rounded_full()
                    .bg(look.color(token::ACCENT)),
            )
        })
        .children(chip)
        .when(chevron, |line| line.child(div().flex_none().size(glyph).when(open, |slot| slot.child(svg().path(icon_path("chevron-right")).size(glyph).text_color(look.color(token::ACCENT))))))
    }

    /// The menu with its submenu beside it, if a row has one.
    pub(super) fn with_submenu(&self, look: &Look, cx: &Context<Self>, menu: impl IntoElement) -> Div {
        let wrap = div().relative().flex().items_start().child(menu);
        let Some((at, entries)) = self.opened() else { return wrap };
        let edge = look.length(token::BORDER_WIDTH);
        let pad = look.length(token::SPACE_1);
        let row = look.length(token::H_MENU_ITEM);
        let above = self.visible().0.into_iter().filter(|&place| place < at).fold(px(0.), |sum, place| {
            sum + match &self.entries[place] {
                Entry::Rule => edge + pad * 2.,
                Entry::Search { .. } => look.length(token::H_CONTROL) + pad,
                _ => row,
            }
        });
        let detailed = self.entries.iter().any(|entry| matches!(entry, Entry::Row(Row { detail: Some(_), .. })));
        let width = look.length(token::W_MENU) + if detailed { look.length(token::SPACE_12) } else { px(0.) };
        let glyph = look.length(token::SPACE_4);
        let first = entries.iter().position(|entry| entry.pickable());
        let items = entries.iter().enumerate().map(|(n, entry)| {
            let (id, label, icon, danger, accent) = match entry {
                Entry::Act { id, label, icon, danger } => (*id, label.clone(), *icon, *danger, false),
                Entry::Row(row) => (row.id, row.label.clone(), row.icon, false, row.beside.accent),
                Entry::Rule => return div().h(edge).mx(look.length(token::SPACE_2)).my(pad).bg(look.color(token::BORDER_SUBTLE)).into_any_element(),
                _ => return div().into_any_element(),
            };
            let ink = if danger {
                look.color(token::STATUS_COMPLETED_FAILED)
            } else if accent {
                look.color(token::ACCENT)
            } else {
                look.color(token::FG_DEFAULT)
            };
            let led = Some(n) == first;
            let on = self.sub().inside && self.sub().cursor == Some(n);
            labelled(
                div()
                    .id(id)
                    .h(row)
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .px(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .when(led, |item| item.rounded_tl(px(0.)).rounded_bl(px(0.)).bg(look.color(token::ACCENT_MUTED)))
                    .border(edge)
                    .border_color(if on { look.color(token::ACCENT) } else { gpui::transparent_black() })
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(ink)
                    .whitespace_nowrap()
                    .when(on && !led, |item| item.bg(look.color(token::BG_HOVER)))
                    .when(!led, |item| item.hover(|style| style.bg(look.color(token::BG_HOVER))))
                    .cursor_pointer()
                    .child(div().flex_none().size(glyph).children(icon.map(|icon| svg().path(icon_path(icon)).size(glyph).text_color(ink))))
                    .child(div().flex_1().child(label.clone()))
                    .on_click(cx.listener(move |this, _, _, cx| this.pick_sub(n, cx))),
                Role::MenuItem,
                label,
            )
            .into_any_element()
        });
        let name = match &self.entries[at] {
            Entry::Row(row) => SharedString::from(format!("{} actions", row.label)),
            _ => SharedString::from("Actions"),
        };
        let sub: AnyElement = labelled(
            div()
                .id("submenu")
                .flex_none()
                .mt(pad + above)
                .min_w(look.length(token::W_MENU))
                .flex()
                .flex_col()
                .px(pad)
                .pb(pad)
                .rounded(look.length(token::RADIUS_LG))
                .rounded_tl(px(0.))
                .border(edge)
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_OVERLAY))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                .font_family(look.font_stack(token::FONT_SANS).into_iter().next().unwrap_or_default())
                .occlude()
                .children(items),
            Role::Menu,
            name,
        )
        .into_any_element();
        // The tint over the menu's right padding and edge and the submenu's left edge and padding, at the row's height.
        let bridge = div().absolute().left(width - edge - pad).top(edge + pad + above).w((edge + pad) * 2.).h(row).bg(look.color(token::ACCENT_MUTED));
        wrap.child(sub).child(bridge)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Anchor, Entity, KeyDownEvent, Keystroke, Point, TestAppContext};

    use super::*;

    fn pressed(key: &'static str) -> KeyDownEvent {
        KeyDownEvent { keystroke: Keystroke::parse(key).expect("a key"), is_held: false, prefer_character_input: false }
    }

    fn menu(cx: &mut TestAppContext) -> Entity<Dropdown> {
        let acts = vec![Entry::Row(Row::new("merge", "Merge with Pricing on mac-air").accent()), Entry::Rule, Entry::Act { id: "delete", label: "Delete Pricing".into(), icon: None, danger: true }];
        let entries = vec![Entry::Row(Row::new("every", "Every project")), Entry::Row(Row::new("one", "Pricing").chip("This Mac", None).submenu(acts))];
        cx.new(|cx| Dropdown::new("Editing", entries, Point::default(), Anchor::TopLeft, false, cx))
    }

    #[gpui::test]
    fn right_enters_the_submenu_from_its_row_and_left_or_escape_come_out(cx: &mut TestAppContext) {
        let menu = menu(cx);
        menu.update(cx, |menu, cx| {
            menu.cursor = Some(0);
            assert!(!menu.sub_key(&pressed("right"), cx), "→ on a row with no submenu is not taken");
            menu.cursor = Some(1);
            assert!(menu.sub_key(&pressed("right"), cx));
            assert!(menu.sub.inside);
            assert_eq!(menu.sub.cursor, Some(0), "the first act");
            assert!(menu.sub_key(&pressed("down"), cx));
            assert_eq!(menu.sub.cursor, Some(2), "the rule is stepped over");
            assert!(menu.sub_key(&pressed("down"), cx));
            assert_eq!(menu.sub.cursor, Some(2), "the last act stays");
            assert!(menu.sub_key(&pressed("up"), cx));
            assert_eq!(menu.sub.cursor, Some(0));
            assert!(menu.sub_key(&pressed("left"), cx));
            assert!(!menu.sub.inside);
            assert!(menu.sub_key(&pressed("right"), cx));
            assert!(menu.sub_key(&pressed("escape"), cx), "Esc inside goes back and does not shut the menu");
            assert!(!menu.sub.inside);
            assert!(!menu.sub_key(&pressed("escape"), cx), "Esc outside is the menu's, which shuts it");
        });
    }

    #[gpui::test]
    fn enter_inside_picks_the_act_the_keys_are_on(cx: &mut TestAppContext) {
        let menu = menu(cx);
        let picked = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let seen = picked.clone();
        let _held = cx.update(|cx| cx.subscribe(&menu, move |_, event: &Picked, _| {
            let Picked::Entry { id, .. } = event;
            seen.borrow_mut().push(*id);
        }));
        menu.update(cx, |menu, cx| {
            menu.cursor = Some(1);
            menu.sub_key(&pressed("right"), cx);
            menu.sub_key(&pressed("down"), cx);
            menu.sub_key(&pressed("enter"), cx);
        });
        assert_eq!(*picked.borrow(), ["delete"]);
    }
}
