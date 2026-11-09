//! The All notes panel (`Layer.tsx:1303-1587`): every note the layer has, by screen, in four tabs
//! over a search, each row with its own acts and the panel's head with the same acts over a
//! selection. It is docked to the trailing edge above the bar, and scrolls inside itself.
//!
//! **No count is drawn**: the tabs, the screens and the selection carry none, where Electron's carry
//! the number of notes they hold (the owner's no-aggregate-counts rule).

use gpui::{div, point, prelude::*, Anchor, ClickEvent, Context, FontWeight, Role, SharedString, Stateful, Window};
use gpui_component::input::Input;

use super::{split, Layer, Note, Status};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::titlebar::parts::{card, glyph, tip, Btn, Weight};

/// The panel's tabs, each a view over the notes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Tab {
    Open,
    NoJob,
    HasJob,
    Done,
}

impl Tab {
    const ALL: [Tab; 4] = [Tab::Open, Tab::NoJob, Tab::HasJob, Tab::Done];

    fn label(self) -> &'static str {
        match self {
            Tab::Open => "Open",
            Tab::NoJob => "No job",
            Tab::HasJob => "Has job",
            Tab::Done => "Done",
        }
    }

    fn holds(self, note: &Note) -> bool {
        match self {
            Tab::Open => note.status == Status::Open,
            Tab::NoJob => note.status == Status::Open && note.sent.is_none(),
            Tab::HasJob => note.status == Status::Open && note.sent.is_some(),
            Tab::Done => note.status == Status::Done,
        }
    }
}

/// Whether the search's words are in the note's text, its screen, or what it is on.
fn matches(note: &Note, words: &str) -> bool {
    words.is_empty()
        || [note.said.as_str(), note.screen.as_deref().unwrap_or(""), note.on.as_str()].iter().any(|field| field.to_lowercase().contains(words))
}

/// The two states of one checkbox: a box with a tick when it is on.
fn checkbox(look: &Look, id: SharedString, on: bool, label: &'static str) -> Stateful<gpui::Div> {
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .cursor_pointer()
        .text_size(look.length(token::TEXT_XS))
        .child(
            div()
                .size(look.length(token::SPACE_4))
                .flex()
                .items_center()
                .justify_center()
                .rounded(look.length(token::RADIUS_SM))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if on { look.color(token::ACCENT) } else { look.color(token::BORDER_DEFAULT) })
                .bg(if on { look.color(token::ACCENT) } else { look.color(token::BG_SUNKEN) })
                .children(on.then(|| glyph("check", look.length(token::SPACE_3), look.color(token::TEXT_ON_ACCENT)))),
        )
        .child(label)
}

impl Layer {
    /// The notes the tab and the search hold, by screen: open first within a screen, a screen named
    /// by its notes in order and "no screen" last.
    fn listed(&self) -> Vec<(Option<SharedString>, Vec<&Note>)> {
        let words = self.query.trim().to_lowercase();
        let held: Vec<&Note> = self.notes.iter().filter(|note| self.tab.holds(note) && matches(note, &words)).collect();
        let mut screens: Vec<Option<SharedString>> = Vec::new();
        for note in &held {
            if !screens.contains(&note.screen) {
                screens.push(note.screen.clone());
            }
        }
        screens.sort_by(|a, b| match (a, b) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, _) => std::cmp::Ordering::Greater,
            (_, None) => std::cmp::Ordering::Less,
            (Some(a), Some(b)) => a.cmp(b),
        });
        screens
            .into_iter()
            .map(|screen| {
                let mut list: Vec<&Note> = held.iter().copied().filter(|note| note.screen == screen).collect();
                list.sort_by_key(|note| note.status == Status::Done);
                (screen, list)
            })
            .collect()
    }

    /// A box ticked: a press on its own, or the run from the last one ticked with Shift.
    fn tick(&mut self, id: u32, shift: bool, cx: &mut Context<Self>) {
        let order: Vec<u32> = self.listed().into_iter().flat_map(|(_, list)| list.into_iter().map(|note| note.id)).collect();
        let on = !self.selected.contains(&id);
        let from = self.anchor.and_then(|anchor| order.iter().position(|one| *one == anchor));
        let to = order.iter().position(|one| *one == id);
        let run: Vec<u32> = match (shift, from, to) {
            (true, Some(from), Some(to)) => order[from.min(to)..=from.max(to)].to_vec(),
            _ => vec![id],
        };
        for one in run {
            if on {
                self.selected.insert(one);
            } else {
                self.selected.remove(&one);
            }
        }
        self.anchor = Some(id);
        self.deleting_selection = false;
        cx.notify();
    }

    fn chosen_ids(&self) -> Vec<u32> {
        let listed: Vec<u32> = self.listed().into_iter().flat_map(|(_, list)| list.into_iter().map(|note| note.id)).collect();
        self.notes.iter().filter(|note| self.selected.contains(&note.id) && listed.contains(&note.id)).map(|note| note.id).collect()
    }

    pub(super) fn panel(&self, window: &Window, cx: &Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let view = window.viewport_size();
        let gap = look.length(token::SPACE_2);
        let bottom = self.bar_at.get().map_or(look.length(token::SPACE_12), |bar| view.height - bar.origin.y + gap);
        let groups = self.listed();
        let listed_ids: Vec<u32> = groups.iter().flat_map(|(_, list)| list.iter().map(|note| note.id)).collect();
        let chosen = self.chosen_ids();
        let chosen_notes: Vec<&Note> = self.notes.iter().filter(|note| chosen.contains(&note.id)).collect();
        let chosen_sendable: Vec<u32> = chosen_notes.iter().filter(|note| note.status == Status::Open && note.sent.is_none()).map(|note| note.id).collect();
        let all_chosen = !listed_ids.is_empty() && chosen.len() == listed_ids.len();
        let busy = self.is_sending();

        let field = self.search.as_ref().map(|(search, _)| {
            labelled(
                div().id("notes-search").child(
                    Input::new(search)
                        .h(look.length(token::H_CONTROL_LG))
                        .rounded(look.length(token::RADIUS_LG))
                        .prefix(gpui::svg().path(icon_path("search")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_SUBTLE))),
                ),
                Role::SearchInput,
                "Search notes",
            )
        });
        let tabs = Tab::ALL.into_iter().map(|tab| {
            let on = self.tab == tab;
            labelled(
                div()
                    .id(SharedString::from(format!("notes-tab-{}", tab.label())))
                    .h(look.length(token::H_CONTROL))
                    .px(look.length(token::SPACE_3))
                    .flex()
                    .items_center()
                    .rounded(look.length(token::RADIUS_LG))
                    .text_size(look.length(token::TEXT_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .cursor_pointer()
                    .when(on, |tab| tab.bg(look.color(token::BG_RAISED)).text_color(look.color(token::FG_DEFAULT)))
                    .when(!on, |tab| tab.text_color(look.color(token::FG_MUTED)).hover(|style| style.bg(look.color(token::BG_HOVER))))
                    .child(tab.label())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.tab = tab;
                        this.deleting_row = None;
                        this.deleting_selection = false;
                        cx.notify();
                    })),
                Role::Tab,
                tab.label(),
            )
        });
        let tabs = labelled(div().id("notes-tabs").flex().gap(look.length(token::SPACE_1)).children(tabs), Role::TabList, "Notes");

        let selection = (!listed_ids.is_empty()).then(|| {
            let any_open = chosen_notes.iter().any(|note| note.status == Status::Open);
            let any_done = chosen_notes.iter().any(|note| note.status == Status::Done);
            let armed = self.deleting_selection;
            let to_all = listed_ids.clone();
            let ids = chosen_sendable.clone();
            let again = chosen_sendable.clone();
            let acts = (!chosen.is_empty()).then(|| {
                let done = chosen.clone();
                let reopen = chosen.clone();
                let doomed = chosen.clone();
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .children(any_open.then(|| {
                        labelled(
                            Btn::new("selection-done", "Mark done", Weight::Ghost).disabled(busy).draw(look, cx).on_click(cx.listener(move |this, _, _, cx| {
                                for id in &done {
                                    if this.notes.iter().any(|note| note.id == *id && note.status == Status::Open) {
                                        this.set_status(*id, Status::Done, cx);
                                    }
                                }
                            })),
                            Role::Button,
                            "Mark done",
                        )
                    }))
                    .children(any_done.then(|| {
                        labelled(
                            Btn::new("selection-reopen", "Reopen", Weight::Ghost).disabled(busy).draw(look, cx).on_click(cx.listener(move |this, _, _, cx| {
                                for id in &reopen {
                                    if this.notes.iter().any(|note| note.id == *id && note.status == Status::Done) {
                                        this.set_status(*id, Status::Open, cx);
                                    }
                                }
                            })),
                            Role::Button,
                            "Reopen",
                        )
                    }))
                    .child(labelled(
                        Btn::new("selection-delete", if armed { "Press again to delete" } else { "Delete" }, Weight::Ghost)
                            .disabled(busy)
                            .draw(look, cx)
                            .text_color(look.color(token::ERROR))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if this.deleting_selection {
                                    for id in &doomed {
                                        this.remove(*id, cx);
                                    }
                                    this.deleting_selection = false;
                                } else {
                                    this.deleting_selection = true;
                                }
                                cx.notify();
                            })),
                        Role::Button,
                        if armed { "Press again to delete" } else { "Delete" },
                    ))
            });
            labelled(
                div()
                    .id("notes-selection")
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(labelled(
                        checkbox(look, "select-all".into(), all_chosen, "Select all").on_click(cx.listener(move |this, _, _, cx| {
                            if all_chosen {
                                this.selected.clear();
                            } else {
                                this.selected.extend(to_all.iter().copied());
                            }
                            this.deleting_selection = false;
                            cx.notify();
                        })),
                        Role::CheckBox,
                        "Select all",
                    ))
                    .child(split(
                        look,
                        cx,
                        "selection-start".into(),
                        "Start job".into(),
                        busy || chosen_sendable.is_empty(),
                        false,
                        None,
                        cx.listener(move |this, _, _, cx| this.send(ids.clone(), None, cx)),
                        cx.listener(move |this, event: &ClickEvent, window, cx| {
                            let at = event.position();
                            this.open_menu(again.clone(), point(at.x, at.y), Anchor::TopRight, window, cx)
                        }),
                    ))
                    .children(acts),
                Role::Group,
                "Selected notes",
            )
        });

        let screens = groups.iter().map(|(screen, list)| {
            let shut = self.folded.contains(screen);
            let name: SharedString = screen.clone().unwrap_or_else(|| "No screen".into());
            let key = screen.clone();
            let rows = (!shut).then(|| div().flex().flex_col().gap(look.length(token::SPACE_2)).children(list.iter().map(|note| self.row(note, cx))));
            labelled(
                div()
                    .id(SharedString::from(format!("notes-screen-{name}")))
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .child(labelled(
                        div()
                            .id(SharedString::from(format!("notes-fold-{name}")))
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_1))
                            .px(look.length(token::SPACE_1))
                            .text_size(look.length(token::TEXT_XS))
                            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                            .text_color(look.color(token::FG_MUTED))
                            .cursor_pointer()
                            .child(glyph(if shut { "chevron-right" } else { "chevron-down" }, look.length(token::SPACE_3), look.color(token::FG_MUTED)))
                            .child(name.clone())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if !this.folded.remove(&key) {
                                    this.folded.insert(key.clone());
                                }
                                cx.notify();
                            })),
                        Role::Button,
                        name.clone(),
                    ))
                    .children(rows),
                Role::Group,
                name,
            )
        });
        let empty = listed_ids.is_empty().then(|| div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child("No notes"));

        labelled(
            card(look)
                .id("notes-panel")
                .occlude()
                .absolute()
                .right(look.length(token::SPACE_3))
                .top(look.length(token::SPACE_3))
                .bottom(bottom)
                .w(look.length(token::W_TOOLTIP) * 1.5)
                .max_w(view.width - look.length(token::SPACE_6))
                .p(look.length(token::SPACE_3))
                .gap(look.length(token::SPACE_3))
                .text_size(look.length(token::TEXT_SM))
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_3))
                        .children(field)
                        .child(tabs)
                        .children(selection),
                )
                .child(
                    div()
                        .id("notes-scroll")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_3))
                        .children(empty)
                        .children(screens),
                ),
            Role::Region,
            "All notes",
        )
    }

    /// One note's row: what it says and what it is on, the Job it went to, and its acts in fixed
    /// columns so they line up down the list.
    fn row(&self, note: &Note, cx: &Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let id = note.id;
        let here = note.ring.is_some();
        let busy = self.is_sending();
        let ticked = self.selected.contains(&id);
        let open = note.status == Status::Open;
        let dispatchable = open && note.sent.is_none() && !self.targets.is_empty();
        let shotless = !here;
        let armed = self.deleting_row == Some(id);
        let ids = vec![id];
        let again = ids.clone();
        let column = |width: gpui::Pixels| div().flex_none().w(width).flex().items_center();
        let (select, start, done) = (look.length(token::SPACE_12) * 1.6, look.length(token::SPACE_12) * 2.6, look.length(token::SPACE_12) * 2.);
        let start_cell = if dispatchable {
            let cell = split(
                look,
                cx,
                format!("row-{id}-start"),
                "Start job".into(),
                busy,
                false,
                None,
                cx.listener(move |this, _, _, cx| this.send(ids.clone(), None, cx)),
                cx.listener(move |this, event: &ClickEvent, window, cx| {
                    let at = event.position();
                    this.open_menu(again.clone(), point(at.x, at.y), Anchor::TopRight, window, cx)
                }),
            );
            // Notes written on another screen have no picture to send with them.
            div().id(SharedString::from(format!("row-{id}-shot"))).when(shotless, |cell| cell.tooltip(tip("No screenshot: on another screen"))).child(cell)
        } else {
            // A row without Start job keeps its column, so the acts after it line up down the list.
            div().id(SharedString::from(format!("row-{id}-none")))
        };
        labelled(
            div()
                .id(SharedString::from(format!("note-row-{id}")))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .p(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if ticked { look.color(token::ACCENT) } else { look.color(token::BORDER_SUBTLE) })
                .bg(look.color(token::BG_BASE))
                .child(labelled(
                    div()
                        .id(SharedString::from(format!("note-main-{id}")))
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_1))
                        .when(here, |main| main.cursor_pointer())
                        .when(!here, |main| main.opacity(0.7))
                        .tooltip(tip(if here { "Show on this screen" } else { "On another screen" }))
                        .child(div().text_size(look.length(token::TEXT_SM)).child(note.said.clone()))
                        .child(div().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_MUTED)).child(note.on.clone()))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if this.notes.iter().any(|note| note.id == id && note.ring.is_some()) {
                                this.open_note(id, window, cx);
                            }
                        })),
                    Role::Button,
                    note.said.clone(),
                ))
                .children(note.sent.clone().map(|title| {
                    div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(format!("Sent to Job {title}"))
                }))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .child(column(select).child(labelled(
                            checkbox(look, format!("note-{id}-select").into(), ticked, "Select").on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                                this.tick(id, event.modifiers().shift, cx);
                            })),
                            Role::CheckBox,
                            format!("Select {}", note.said),
                        )))
                        .child(column(start).child(start_cell))
                        .child(column(done).child(labelled(
                            Btn::new(SharedString::from(format!("note-{id}-status")), if open { "Mark done" } else { "Reopen" }, Weight::Ghost)
                                .draw(look, cx)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    let next = if this.notes.iter().any(|note| note.id == id && note.status == Status::Open) { Status::Done } else { Status::Open };
                                    this.set_status(id, next, cx);
                                })),
                            Role::Button,
                            if open { "Mark done" } else { "Reopen" },
                        )))
                        .child(div().flex_1().min_w_0().flex().items_center().child(labelled(
                            Btn::new(SharedString::from(format!("note-{id}-delete")), if armed { "Press again to delete" } else { "Delete" }, Weight::Ghost)
                                .draw(look, cx)
                                .text_color(look.color(token::ERROR))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if this.deleting_row == Some(id) {
                                        this.deleting_row = None;
                                        this.remove(id, cx);
                                    } else {
                                        this.deleting_row = Some(id);
                                        cx.notify();
                                    }
                                })),
                            Role::Button,
                            if armed { "Press again to delete" } else { "Delete" },
                        ))),
                ),
            Role::ListItem,
            note.said.clone(),
        )
    }
}
