//! Settings → Theme: one field holding the theme in force, which opens Dark, Light and the themes mods ship, each
//! with its colours, and narrows them as it is typed in. Classic's `ThemeSettings` over `ThemePicker`
//! (`packages/components/src/compositions/ThemePicker/`). Under it, the theme mods and the tweaks (`mods.rs`).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::OnceLock;

use gpui::{
    anchored, canvas, deferred, div, point, prelude::*, px, svg, AnyElement, App, Bounds, Context, FocusHandle, FontWeight, Hsla, Keystroke, MouseButton,
    Pixels, Role, ScrollHandle, SharedString, Window,
};

use super::overlay::SettingsOverlay;
use super::view::{bounds_of, card, ringed, small};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{catalogue, token, Choice, Look, Token, Tone};

/// What a strip previews a theme by, in the order it is drawn (`packages/tokens/themes/swatch.mjs`).
const SWATCH: [Token; 8] = [
    token::BG_BASE,
    token::BG_RAISED,
    token::FG_DEFAULT,
    token::ACCENT,
    token::STATUS_COMPLETED_SUCCESS,
    token::STATUS_COMPLETED_FAILED,
    token::STATUS_AWAITING_REVIEW,
    token::STATUS_RUNNING,
];

/// The field's name, and the list's.
pub const NAME: &str = "Theme";

/// How many rows the list shows before it scrolls.
const ROWS_SHOWN: f32 = 8.;

/// A mod Armada ships is named `built-in:` and its folder's name; its title is the folder's name.
const BUILT_IN: &str = "built-in:";

fn strip_of(look: &Look) -> Vec<Hsla> {
    SWATCH.iter().map(|one| look.color(*one)).collect()
}

/// Dark, Light and every catalogue theme, each with its tone and strip, read once.
fn shipped() -> &'static [(String, String, Tone, Vec<Hsla>)] {
    static SHIPPED: OnceLock<Vec<(String, String, Tone, Vec<Hsla>)>> = OnceLock::new();
    SHIPPED.get_or_init(|| {
        let mut all = vec![("dark".to_string(), "Dark".to_string(), Tone::Dark, strip_of(&Look::dark())), ("light".to_string(), "Light".to_string(), Tone::Light, strip_of(&Look::light()))];
        for one in catalogue() {
            if let Some(look) = Look::catalogue(&one.id) {
                all.push((one.id.clone(), one.title.clone(), one.tone, strip_of(&look)));
            }
        }
        all
    })
}

/// One theme the field offers.
#[derive(Clone, Debug, PartialEq)]
pub struct Offered {
    pub id: String,
    pub title: String,
    /// `None` until its colours are known: a mod whose sheet Fleet has not checked yet.
    pub strip: Option<Vec<Hsla>>,
}

/// Dark, Light and From mods, each sorted by title after its own; a group with no theme is left out.
pub fn groups(mods: &[(String, Option<Vec<Hsla>>)]) -> Vec<(&'static str, Vec<Offered>)> {
    let of = |tone: Tone, first: &str| {
        let mut rest: Vec<Offered> = shipped().iter().filter(|(id, _, at, _)| *at == tone && id != first && id.starts_with("catalogue:")).map(|(id, title, _, strip)| Offered { id: id.clone(), title: title.clone(), strip: Some(strip.clone()) }).collect();
        rest.sort_by(|a, b| a.title.cmp(&b.title));
        let own = shipped().iter().find(|(id, ..)| id == first).map(|(id, title, _, strip)| Offered { id: id.clone(), title: title.clone(), strip: Some(strip.clone()) });
        own.into_iter().chain(rest).collect::<Vec<_>>()
    };
    let from_mods = mods.iter().map(|(name, strip)| Offered { id: name.clone(), title: name.strip_prefix(BUILT_IN).unwrap_or(name).to_string(), strip: strip.clone() }).collect();
    vec![("Dark", of(Tone::Dark, "dark")), ("Light", of(Tone::Light, "light")), ("From mods", from_mods)].into_iter().filter(|(_, choices)| !choices.is_empty()).collect()
}

/// The groups narrowed to the titles holding `needle`, and a group left with none dropped.
pub fn narrowed(all: Vec<(&'static str, Vec<Offered>)>, needle: &str) -> Vec<(&'static str, Vec<Offered>)> {
    let needle = needle.trim().to_lowercase();
    all.into_iter().map(|(label, choices)| (label, choices.into_iter().filter(|one| one.title.to_lowercase().contains(&needle)).collect::<Vec<_>>())).filter(|(_, choices)| !choices.is_empty()).collect()
}

/// The theme field's own state.
pub struct ThemeField {
    pub focus: FocusHandle,
    pub open: bool,
    /// What was typed, or `None` while the field shows the chosen theme.
    pub query: Option<String>,
    /// What the field holds is selected, as it is when the keys arrive or after ⌘A: the next key typed replaces it.
    pub selected: bool,
    pub active: usize,
    /// The word picked here, until Fleet answers: the field reads it at once, as Classic's does.
    pub picked: Option<String>,
    pub refused: Option<String>,
    /// The strip of each mod, by name and the sheet it was read from.
    strips: RefCell<BTreeMap<String, (String, Vec<Hsla>)>>,
    scroll: ScrollHandle,
    /// Where each row was drawn last frame, by its place, and the list itself.
    rows: Rc<RefCell<BTreeMap<usize, Bounds<Pixels>>>>,
    list: Rc<RefCell<Option<Bounds<Pixels>>>>,
}

impl ThemeField {
    pub fn new(cx: &mut App) -> ThemeField {
        ThemeField {
            focus: cx.focus_handle().tab_stop(true),
            open: false,
            query: None,
            selected: false,
            active: 0,
            picked: None,
            refused: None,
            strips: RefCell::default(),
            scroll: ScrollHandle::new(),
            rows: Rc::default(),
            list: Rc::default(),
        }
    }
}

impl SettingsOverlay {
    /// The word in force: what was picked here while Fleet answers, else Fleet's.
    fn theme_word(&self, cx: &App) -> String {
        self.theme.picked.clone().unwrap_or_else(|| self.wire.theme(cx))
    }

    /// The theme mods that are on and pass their checks, each with its strip once Fleet has checked its sheet.
    fn theme_mods(&self, cx: &App) -> Vec<(String, Option<Vec<Hsla>>)> {
        let sheets = self.wire.theme_sheets(cx);
        let mut strips = self.theme.strips.borrow_mut();
        self.wire
            .mods(cx)
            .into_iter()
            .filter(|row| row.enabled && row.valid && row.kind.is_none_or(|kind| kind == ipc::ModKind::Theme))
            .map(|row| {
                let strip = sheets.get(&row.name).map(|sheet| match strips.get(&row.name) {
                    Some((read, strip)) if read == sheet => strip.clone(),
                    _ => {
                        let strip = strip_of(&Look::of_mod(&row.name, sheet));
                        strips.insert(row.name.clone(), (sheet.clone(), strip.clone()));
                        strip
                    }
                });
                (row.name, strip)
            })
            .collect()
    }

    fn theme_shown(&self, cx: &App) -> Vec<(&'static str, Vec<Offered>)> {
        let all = groups(&self.theme_mods(cx));
        match &self.theme.query {
            Some(needle) => narrowed(all, needle),
            None => all,
        }
    }

    fn theme_flat(&self, cx: &App) -> Vec<Offered> {
        self.theme_shown(cx).into_iter().flat_map(|(_, choices)| choices).collect()
    }

    /// The keys reach the field: what it holds is selected, and the list opens on the theme in force.
    pub fn theme_focused(&mut self, cx: &mut Context<Self>) {
        self.theme.selected = true;
        self.theme_open(cx);
    }

    /// Open the list on the theme in force, as a press on Classic's field does.
    fn theme_open(&mut self, cx: &mut Context<Self>) {
        if self.theme.open {
            return;
        }
        let word = self.theme_word(cx);
        self.theme.active = self.theme_flat(cx).iter().position(|one| one.id == word).unwrap_or(0);
        self.theme.open = true;
        self.reveal_theme_row();
        cx.notify();
    }

    pub fn theme_shut(&mut self, cx: &mut Context<Self>) {
        self.theme.open = false;
        // What was typed gives way to the theme's name, and the selection goes with it.
        if self.theme.query.take().is_some() {
            self.theme.selected = false;
        }
        cx.notify();
    }

    /// Choose a theme: it is drawn at once, and kept by Fleet as the `theme` preference.
    fn theme_choose(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(picked) = self.theme_flat(cx).into_iter().nth(at) else { return };
        self.theme_shut(cx);
        self.theme.selected = false;
        if picked.id == self.theme_word(cx) || !self.live {
            return;
        }
        self.theme.picked = Some(picked.id.clone());
        self.theme.refused = None;
        self.wire.show_theme(&picked.id, window, cx);
        let sent = self.wire.save_preference("theme", picked.id, cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = sent.await;
            let _ = this.update_in(cx, |this, window, cx| {
                match answer {
                    Ok(preferences) => this.wire.preferences_saved(preferences, window, cx),
                    Err(why) => {
                        this.theme.refused = Some(why.said);
                        let was = this.wire.theme(cx);
                        this.wire.show_theme(&was, window, cx);
                    }
                }
                this.theme.picked = None;
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Bring the highlighted row into the list's view, from where the rows were drawn last frame.
    fn reveal_theme_row(&mut self) {
        let (Some(list), Some(row)) = (*self.theme.list.borrow(), self.theme.rows.borrow().get(&self.theme.active).copied()) else {
            if self.theme.active == 0 {
                self.theme.scroll.set_offset(point(px(0.), px(0.)));
            }
            return;
        };
        let offset = self.theme.scroll.offset();
        if row.top() < list.top() {
            self.theme.scroll.set_offset(point(offset.x, offset.y + (list.top() - row.top())));
        } else if row.bottom() > list.bottom() {
            self.theme.scroll.set_offset(point(offset.x, offset.y - (row.bottom() - list.bottom())));
        }
    }

    /// The field's keys, ahead of the overlay's: `true` where one was taken. Esc with the list shut, and Tab, go
    /// on to the overlay, so the same key still closes Settings and moves on.
    pub fn theme_key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.theme.focus.is_focused(window) {
            return false;
        }
        let mods = &key.modifiers;
        if key.key == "a" && mods.platform && !mods.control && !mods.alt {
            self.theme.selected = true;
            cx.notify();
            return true;
        }
        if mods.platform || mods.control || mods.alt {
            return false;
        }
        let count = self.theme_flat(cx).len();
        match key.key.as_str() {
            "escape" if self.theme.open => self.theme_shut(cx),
            "escape" => return false,
            "tab" => {
                self.theme_shut(cx);
                return false;
            }
            "down" if !self.theme.open => self.theme_open(cx),
            "down" if count > 0 => {
                self.theme.active = (self.theme.active + 1) % count;
                self.reveal_theme_row();
            }
            "up" if self.theme.open && count > 0 => {
                self.theme.active = (self.theme.active + count - 1) % count;
                self.reveal_theme_row();
            }
            "up" | "down" => {}
            "enter" if self.theme.open && count > 0 => self.theme_choose(self.theme.active, window, cx),
            "enter" => {}
            // A key typed over a selection replaces it; otherwise it goes after what the field holds.
            "backspace" => {
                let mut typed = self.theme_held_text(cx);
                typed.pop();
                self.theme_typed(typed);
            }
            _ => {
                let Some(text) = typed(key) else { return false };
                let held = self.theme_held_text(cx);
                self.theme_typed(held + &text);
            }
        }
        cx.notify();
        true
    }

    /// The text a key typed goes after: nothing over a selection, else what was typed, else the theme's name.
    fn theme_held_text(&self, cx: &App) -> String {
        if self.theme.selected {
            return String::new();
        }
        self.theme.query.clone().unwrap_or_else(|| {
            let word = self.theme_word(cx);
            groups(&self.theme_mods(cx)).into_iter().flat_map(|(_, choices)| choices).find(|one| one.id == word).map(|one| one.title).unwrap_or_default()
        })
    }

    fn theme_typed(&mut self, text: String) {
        self.theme.query = Some(text);
        self.theme.selected = false;
        self.theme.open = true;
        self.theme.active = 0;
        self.theme.scroll.set_offset(point(px(0.), px(0.)));
    }

    pub fn render_theme(&self, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let word = self.theme_word(cx);
        let all = groups(&self.theme_mods(cx));
        let chosen = all.iter().flat_map(|(_, choices)| choices).find(|one| one.id == word).cloned();
        let focused = self.theme.focus.is_focused(window);
        let off = !self.live;
        let typing = self.theme.query.clone();
        let shown = match &typing {
            Some(needle) => narrowed(all, needle),
            None => all,
        };
        let text: SharedString = typing.clone().or_else(|| chosen.as_ref().map(|one| one.title.clone())).unwrap_or_default().into();
        // On focus Classic selects what the field holds, so the first key replaces it.
        let selected = focused && self.theme.selected && !text.is_empty();
        let words = div().min_w_0().flex().items_center().truncate().child(if selected {
            div().rounded(look.length(token::RADIUS_SM)).bg(look.color(token::ACCENT_MUTED)).child(text.clone()).into_any_element()
        } else {
            div().child(text.clone()).into_any_element()
        });
        // The caret after what is typed; while the words are selected, the selection says where the keys go.
        let caret = (focused && !selected).then(|| div().flex_none().w(look.length(token::BORDER_WIDTH)).h(look.length(token::LEADING_SM)).bg(look.color(token::FG_DEFAULT)));
        let trailing = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .children(chosen.as_ref().filter(|_| typing.is_none()).map(|one| strip(look, one.strip.as_deref())))
            .child(svg().path(icon_path("chevron-down")).flex_none().size(look.length(token::SPACE_4)).text_color(look.color(token::FG_MUTED)));
        let field = ringed(
            look,
            div()
                .id("theme-field")
                .track_focus(&self.theme.focus)
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_SUNKEN))
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .text_color(if off { look.color(token::FG_SUBTLE) } else { look.color(token::FG_DEFAULT) })
                .cursor_text()
                .child(div().flex_1().min_w_0().flex().items_center().child(words).children(caret))
                .child(trailing)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, window, cx| {
                        if this.theme.focus.is_focused(window) {
                            if this.theme.open {
                                this.theme_shut(cx);
                            } else {
                                this.theme_open(cx);
                            }
                        } else {
                            window.focus(&this.theme.focus, cx);
                        }
                        cx.stop_propagation();
                    }),
                ),
            &self.theme.focus,
            window,
        );
        let list = (self.theme.open && focused && !shown.is_empty() && !off).then(|| self.theme_list(&shown, &word, look, window, cx));
        let label = div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_MUTED)).child(NAME);
        let body = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .child(label)
            .child(labelled(field, Role::ComboBox, NAME))
            .children(list)
            .children(self.theme.refused.clone().map(|said| labelled(small(look, said.clone(), look.color(token::STATUS_COMPLETED_FAILED)).id("theme-refused"), Role::Alert, said)));
        labelled(card(look, "card-theme", NAME, None, vec![body.into_any_element()]), Role::Region, NAME).into_any_element()
    }

    fn theme_list(&self, shown: &[(&'static str, Vec<Offered>)], word: &str, look: &Look, window: &Window, cx: &Context<Self>) -> AnyElement {
        let wide = bounds_of(window, cx, Role::ComboBox, NAME).map_or(look.length(token::W_PROPOSAL_SETTINGS), |bounds| bounds.size.width);
        let rows = self.theme.rows.clone();
        rows.borrow_mut().clear();
        let mut at = 0;
        let mut children: Vec<AnyElement> = Vec::new();
        for (label, choices) in shown {
            let heading = div()
                .pt(look.length(token::SPACE_2))
                .pb(look.length(token::SPACE_1))
                .px(look.length(token::SPACE_3))
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_2XS))
                .text_color(look.color(token::FG_SUBTLE))
                .child(*label);
            let mut group = div().flex().flex_col().child(heading);
            for one in choices {
                let index = at;
                at += 1;
                let measured = rows.clone();
                let row = div()
                    .id(SharedString::from(format!("theme-{}", one.id)))
                    .relative()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .h(look.length(token::H_MENU_ITEM))
                    .px(look.length(token::SPACE_3))
                    .text_size(look.length(token::TEXT_SM))
                    .text_color(look.color(token::FG_DEFAULT))
                    .cursor_pointer()
                    .when(index == self.theme.active, |row| row.bg(look.color(token::BG_HOVER)))
                    .when(one.id == word, |row| row.font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))))
                    .child(div().flex_1().min_w_0().truncate().child(SharedString::from(one.title.clone())))
                    .child(strip(look, one.strip.as_deref()))
                    .child(canvas(move |bounds, _, _| { measured.borrow_mut().insert(index, bounds); }, |_, _, _, _| {}).absolute().top_0().left_0().size_full())
                    .on_mouse_move(cx.listener(move |this, _, _, cx| {
                        if this.theme.active != index {
                            this.theme.active = index;
                            cx.notify();
                        }
                    }))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, window, cx| {
                            this.theme_choose(index, window, cx);
                            cx.stop_propagation();
                        }),
                    );
                group = group.child(labelled(row, Role::ListBoxOption, one.title.clone()));
            }
            children.push(labelled(group.id(SharedString::from(format!("theme-group-{label}"))), Role::Group, *label).into_any_element());
        }
        let measured = self.theme.list.clone();
        let list = div()
            .id("theme-list")
            .relative()
            .w(wide)
            .max_h(look.length(token::H_MENU_ITEM) * ROWS_SHOWN)
            .overflow_y_scroll()
            .track_scroll(&self.theme.scroll)
            .mt(look.length(token::SPACE_1))
            .py(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .occlude()
            .child(canvas(move |bounds, _, _| *measured.borrow_mut() = Some(bounds), |_, _, _, _| {}).absolute().top_0().left_0().size_full())
            .children(children);
        // Over what follows the field, as Classic's list floats under it; the field keeps the keys.
        div().h_0().child(deferred(anchored().child(labelled(list, Role::ListBox, NAME))).with_priority(3)).into_any_element()
    }
}

/// The theme's colours as one compact strip. Not known yet draws the same strip, said in nothing.
fn strip(look: &Look, colours: Option<&[Hsla]>) -> AnyElement {
    let cell = |colour: Option<Hsla>| div().w(look.length(token::SPACE_2)).h(look.length(token::SPACE_3)).bg(colour.unwrap_or(look.color(token::BG_HOVER)));
    let cells: Vec<_> = match colours {
        Some(colours) => colours.iter().map(|one| cell(Some(*one))).collect(),
        None => (0..SWATCH.len()).map(|_| cell(None)).collect(),
    };
    div().flex_none().flex().overflow_hidden().border(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_DEFAULT)).rounded(look.length(token::RADIUS_SM)).children(cells).into_any_element()
}

/// The text a key types, if it is one that types.
fn typed(key: &Keystroke) -> Option<String> {
    if key.key == "space" {
        return Some(" ".into());
    }
    if key.key.chars().count() != 1 {
        return None;
    }
    key.key_char.clone().filter(|text| !text.chars().any(char::is_control)).or_else(|| Some(key.key.clone()))
}

/// The look a theme word draws, worked out as `LookController` does, for a walk on the mock Fleet, where no
/// controller runs.
pub fn look_of(word: &str, sheets: &BTreeMap<String, String>, window: &Window) -> Look {
    let tone = match window.appearance() {
        gpui::WindowAppearance::Light | gpui::WindowAppearance::VibrantLight => Tone::Light,
        _ => Tone::Dark,
    };
    Look::pick(&Choice::parse(word), tone, sheets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_and_light_lead_their_groups_and_the_catalogue_follows_by_title() {
        let all = groups(&[]);
        let labels: Vec<&str> = all.iter().map(|(label, _)| *label).collect();
        assert_eq!(labels, ["Dark", "Light"]);
        assert_eq!(all[0].1[0].title, "Dark");
        assert_eq!(all[1].1[0].title, "Light");
        let rest: Vec<&str> = all[0].1[1..].iter().map(|one| one.title.as_str()).collect();
        let mut sorted = rest.clone();
        sorted.sort();
        assert_eq!(rest, sorted);
        assert!(all.iter().flat_map(|(_, choices)| choices).all(|one| one.strip.as_ref().is_some_and(|strip| strip.len() == 8)));
    }

    #[test]
    fn typing_narrows_to_the_titles_holding_it_and_drops_an_empty_group() {
        let mods = vec![("dusk".to_string(), None), ("built-in:paper".to_string(), None)];
        let found = narrowed(groups(&mods), "DUS");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, "From mods");
        assert_eq!(found[0].1[0].title, "dusk");
        let shipped = narrowed(groups(&mods), "paper");
        assert_eq!(shipped[0].1[0].title, "paper");
        assert_eq!(shipped[0].1[0].id, "built-in:paper");
        assert!(narrowed(groups(&mods), "nothing at all").is_empty());
    }
}
