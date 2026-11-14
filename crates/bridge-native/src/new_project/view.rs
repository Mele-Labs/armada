//! The New project sheet drawn as the approved mock draws it (`np-sketch.py`'s `sheet`, and `LEARN` for Use
//! defaults): every size and colour a token.

use gpui::{div, point, prelude::*, px, AnyElement, Context, Div, ElementId, Focusable, FontWeight, HighlightStyle, KeyDownEvent, Role, SharedString, StyledText, Toggled, Window};
use gpui_component::input::{Input, InputState};
use gpui_component::text::TextView;

use super::values::{self, Key};
use super::{tilde, Line, NewProjectSheet, Place, PLACES};
use crate::keymap::{Act, TEXT_BOX};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::titlebar::parts::{glyph, kbd, mono, sans, tip, Btn, Weight};

impl Render for NewProjectSheet {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.is_open() {
            return div().into_any_element();
        }
        let look = cx.look();
        let view = crate::room::of(window, cx).size;
        let width = look.length(token::PALETTE_WIDTH).min(view.width - look.length(token::SPACE_6) * 2.);
        let top = (view.height * 0.06).max(look.length(token::H_TITLE_BAR));
        let frame = div()
            .id("new-project-frame")
            .key_context("new_project")
            .track_focus(&self.at.frame)
            .on_action(cx.listener(|this, key: &Act, window, cx| this.on_act(key, window, cx)))
            // A press on the sheet is not a press on the scrim.
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .w(width)
            .max_h(view.height - top - look.length(token::SPACE_6))
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .text_color(look.color(token::FG_DEFAULT))
            .font_family(sans(look))
            .child(self.head(look))
            .child(
                div()
                    .id("new-project-body")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p(look.length(token::SPACE_4))
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_4))
                    .child(self.name_section(window, cx))
                    .child(self.repositories(window, cx))
                    .child(self.place_section(window, cx))
                    .child(self.settings(window, cx)),
            )
            .child(self.foot(window, cx));
        let scrim = div()
            .id("new-project-scrim")
            .absolute()
            .inset_0()
            .occlude()
            .bg(look.color(token::SCRIM))
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, window, cx| this.close(window, cx)));
        let column = div().absolute().top(top).left((view.width - width) / 2.).child(frame);
        labelled(div().id("new-project").absolute().inset_0().child(scrim).child(column).children(self.popup()), Role::Dialog, "New project").into_any_element()
    }
}

/// A section's caption: small capitals, muted, with its glyph.
fn caption(icon: Option<&'static str>, words: &str, look: &Look) -> Div {
    div()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_1) * 1.5)
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .text_color(look.color(token::FG_MUTED))
        .children(icon.map(|name| glyph(name, look.length(token::SPACE_3), look.color(token::FG_MUTED))))
        .child(words.to_uppercase())
}

fn section(look: &Look) -> Div {
    div().flex().flex_col().gap(look.length(token::SPACE_2))
}

/// A text field's frame: sunken, one hairline, the accent and its ring while it has the keys.
fn field(look: &Look, focused: bool) -> Div {
    let accent = look.color(token::ACCENT);
    div()
        .h(look.length(token::H_CONTROL_LG))
        .px(look.length(token::SPACE_3))
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(if focused { accent } else { look.color(token::BORDER_DEFAULT) })
        .when(focused, |field| {
            field.shadow(vec![gpui::BoxShadow { color: accent, offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: look.length(token::BORDER_WIDTH), inset: false }])
        })
        .bg(look.color(token::BG_SUNKEN))
        .text_size(look.length(token::TEXT_SM))
}

/// A text box inside a field: the field draws its frame.
fn text_box(state: &gpui::Entity<InputState>) -> Div {
    div().flex_1().min_w_0().key_context(TEXT_BOX).child(Input::new(state).appearance(false))
}

/// A placeholder shape while a read is out.
fn bone(look: &Look, wide: gpui::Pixels) -> Div {
    div().flex_none().w(wide).h(look.length(token::SPACE_3)).rounded(look.length(token::RADIUS_SM)).bg(look.color(token::BG_HOVER))
}

/// A tick box: empty, ticked, or half, for a repository some of whose folders are picked.
fn tick(look: &Look, ticked: bool, half: bool) -> Div {
    let accent = look.color(token::ACCENT);
    div()
        .flex_none()
        .size(look.length(token::SPACE_4))
        .flex()
        .items_center()
        .justify_center()
        .rounded(look.length(token::RADIUS_SM))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(if ticked || half { accent } else { look.color(token::BORDER_STRONG) })
        .when(ticked, |tick| tick.bg(accent).child(glyph("check", look.length(token::SPACE_3), look.color(token::FG_INVERSE))))
        .when(half && !ticked, |tick| tick.bg(accent.opacity(0.45)))
}

impl NewProjectSheet {
    fn head(&self, look: &Look) -> Div {
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_RAISED))
            .child(glyph("folder", look.length(token::SPACE_4), look.color(token::FG_DEFAULT)))
            .child(
                div()
                    .flex_1()
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .child("New project"),
            )
            .child(kbd("Esc", look))
    }

    fn name_section(&self, window: &Window, cx: &Context<Self>) -> Div {
        let look = cx.look();
        let focused = self.boxes.name.focus_handle(cx).is_focused(window);
        section(look).child(caption(None, "Name", look)).child(labelled(field(look, focused).id("np-name").child(text_box(&self.boxes.name)), Role::TextInput, "Name"))
    }

    // -- the repositories ----------------------------------------------------------------------------------

    fn repositories(&self, window: &Window, cx: &Context<Self>) -> Div {
        let look = cx.look();
        let find_focused = self.boxes.find.focus_handle(cx).is_focused(window);
        let find = labelled(
            field(look, find_focused).id("np-find").child(glyph("search", look.length(token::SPACE_3) + look.length(token::SPACE_1) / 2., look.color(token::FG_SUBTLE))).child(text_box(&self.boxes.find)),
            Role::SearchInput,
            "Find a repository",
        );
        let list_focused = self.at.list.is_focused(window);
        let mut rows: Vec<AnyElement> = Vec::new();
        match &self.served {
            None => {
                for wide in [7., 5.5, 6.5] {
                    rows.push(self.row_frame(look).child(bone(look, look.length(token::SPACE_12) * wide / 3.)).into_any_element());
                }
            }
            Some(_) => {
                for (at, line) in self.lines(cx).into_iter().enumerate() {
                    let active = list_focused && at == self.active;
                    rows.push(self.line_view(at, line.clone(), active, cx));
                    // The opened repository's search stands under its Whole repository, and its folders under it.
                    if let Line::Whole(of) = line {
                        rows.push(self.folder_search(of, window, cx));
                        if self.folders_unread() {
                            rows.push(self.fold_frame(look).child(bone(look, look.length(token::SPACE_12) * 2.)).into_any_element());
                        }
                        if let Some(why) = self.folder_refusal(cx) {
                            rows.push(self.quiet_line("np-folder-refusal", why, cx));
                        }
                    }
                    if line == Line::Clone && self.clone_open {
                        rows.push(self.clone_fields(window, cx));
                        if let Some(why) = self.forge_refusal(cx) {
                            rows.push(self.quiet_line("np-forge-refusal", why, cx));
                        }
                    }
                    // Where more matched than the rows show, the list ends on a line that says so.
                    if matches!(line, Line::Forge(of) if of + 1 == self.forge_rows(cx).len()) && self.forge_cut(cx) {
                        rows.push(self.quiet_line("np-forge-cut", "More match. Type more to narrow it.".into(), cx));
                    }
                    if line == Line::New && self.new_open {
                        rows.push(self.new_fields(window, cx));
                    }
                }
            }
        }
        let list = labelled(
            div()
                .id("np-list")
                .track_focus(&self.at.list)
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| this.list_key(event, window, cx)))
                .flex()
                .flex_col()
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(if list_focused { token::ACCENT } else { token::BORDER_DEFAULT }))
                .bg(look.color(token::BG_SUNKEN))
                .children(rows),
            Role::Tree,
            "Repositories",
        );
        section(look).child(caption(Some("git-branch"), "Repositories", look)).child(find).child(list)
    }

    fn row_frame(&self, look: &Look) -> Div {
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::H_CONTROL_LG) + look.length(token::SPACE_1))
            .px(look.length(token::SPACE_3))
            .text_size(look.length(token::TEXT_SM))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
    }

    /// A line under an opened repository: lower, smaller, and set in past its chevron and box.
    fn fold_frame(&self, look: &Look) -> Div {
        self.row_frame(look).h(look.length(token::H_CONTROL_LG)).pl(look.length(token::SPACE_12) - look.length(token::SPACE_1) / 2.).text_size(look.length(token::TEXT_XS))
    }

    /// A line under an opened repository or a search that says a thing quietly: Fleet's sentence for a refusal, or
    /// that the answer was cut.
    fn quiet_line(&self, id: &'static str, said: SharedString, cx: &Context<Self>) -> AnyElement {
        let look = cx.look();
        // A span in backticks is code, drawn as the thread draws it; the name VoiceOver reads leaves the marks out.
        let words = if said.contains('`') {
            TextView::markdown(ElementId::Name(id.into()), said.to_string()).w_full().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).into_any_element()
        } else {
            div().child(said.clone()).into_any_element()
        };
        let row = self.fold_frame(look).id(id).text_color(look.color(token::FG_SUBTLE)).child(div().min_w_0().truncate().child(words));
        labelled(row, Role::Status, said.replace('`', "")).into_any_element()
    }

    fn meta(look: &Look, said: impl Into<SharedString>) -> Div {
        div().flex_none().ml_auto().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(said.into())
    }

    fn line_view(&self, at: usize, line: Line, active: bool, cx: &Context<Self>) -> AnyElement {
        let look = cx.look();
        let hover = look.color(token::BG_HOVER);
        let keys_here = move |row: gpui::Stateful<Div>| row.when(active, |row| row.bg(hover)).cursor_pointer().hover(move |style| style.bg(hover));
        let to_line = move |this: &mut NewProjectSheet, window: &mut Window, cx: &mut Context<NewProjectSheet>| {
            this.active = at;
            window.focus(&this.at.list, cx);
        };
        match line {
            Line::Repository(of) => {
                let Some(one) = self.served.as_ref().and_then(|all| all.get(of)).cloned() else { return div().into_any_element() };
                let pick = self.pick_of(of);
                let opened = self.opened == Some(of);
                let part = !pick.whole && !pick.folders.is_empty();
                let meta = match pick.folders.len() {
                    _ if !part => tilde(&one.root),
                    1 => "1 folder picked".to_string(),
                    many => format!("{many} folders picked"),
                };
                let accent = look.color(token::ACCENT);
                // A press anywhere on the row but its box opens it, the chevron with it.
                let chevron = div()
                    .flex_none()
                    .child(glyph(if opened { "chevron-down" } else { "chevron-right" }, look.length(token::SPACE_3) + look.length(token::SPACE_1) / 2., look.color(token::FG_MUTED)));
                let tick_box = labelled(
                    tick(look, pick.whole, part).id(SharedString::from(format!("np-tick-{of}"))).aria_toggled(if pick.whole { Toggled::True } else if part { Toggled::Mixed } else { Toggled::False }).on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        to_line(this, window, cx);
                        this.tick_whole(of, cx);
                    })),
                    Role::CheckBox,
                    one.name.clone(),
                );
                let row = self
                    .row_frame(look)
                    .relative()
                    .id(SharedString::from(format!("np-repository-{of}")))
                    .when(pick.whole, |row| row.bg(look.color(token::ACCENT_FAINT)).child(div().absolute().left_0().top_0().bottom_0().w(look.length(token::BORDER_WIDTH) * 2.).bg(accent)))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        to_line(this, window, cx);
                        this.toggle_open(of, window, cx);
                    }))
                    .child(chevron)
                    .child(tick_box)
                    .child(div().min_w_0().truncate().child(one.name.clone()))
                    .child(Self::meta(look, meta));
                labelled(keys_here(row), Role::TreeItem, one.name).into_any_element()
            }
            Line::Whole(of) => {
                let pick = self.pick_of(of);
                let root = self.served.as_ref().and_then(|all| all.get(of)).map(|one| tilde(&one.root)).unwrap_or_default();
                let row = self
                    .fold_frame(look)
                    .id(SharedString::from(format!("np-whole-{of}")))
                    .aria_toggled(if pick.whole { Toggled::True } else { Toggled::False })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        to_line(this, window, cx);
                        this.tick_whole(of, cx);
                    }))
                    .child(tick(look, pick.whole, false))
                    .child("Whole repository")
                    .child(Self::meta(look, root));
                labelled(keys_here(row), Role::CheckBox, "Whole repository").into_any_element()
            }
            Line::Forge(of) => {
                let Some(found) = self.forge_rows(cx).get(of).cloned() else { return div().into_any_element() };
                let here = self.forge_local(&found).is_some();
                let said = format!("{}/{}", found.owner, found.name);
                let when = found.pushed_at.as_ref().and_then(|at| crate::settings::time::stamp_at(at.as_str()));
                let lock = found.private.then(|| {
                    labelled(
                        div().id(SharedString::from(format!("np-forge-private-{of}"))).flex_none().tooltip(tip("Private repository")).child(glyph("lock", look.length(token::SPACE_3), look.color(token::FG_MUTED))),
                        Role::Image,
                        format!("{} is private", said),
                    )
                });
                let row = self
                    .fold_frame(look)
                    .id(SharedString::from(format!("np-forge-{of}")))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        to_line(this, window, cx);
                        this.use_forge(of, window, cx);
                    }))
                    .child(glyph("git-branch", look.length(token::SPACE_3) + look.length(token::SPACE_1) / 2., look.color(token::FG_MUTED)))
                    .child(div().min_w_0().truncate().child(said.clone()))
                    .children(lock)
                    .child(Self::meta(look, if here { "already here".to_string() } else { when.unwrap_or_default() }));
                labelled(keys_here(row), Role::Button, said).into_any_element()
            }
            Line::Folder { at: of, path, lit, kept } => {
                let ticked = self.pick_of(of).folders.contains(&path);
                let marked = HighlightStyle { color: Some(look.color(token::ACCENT)), font_weight: Some(FontWeight(look.number(token::WEIGHT_HEADING))), ..HighlightStyle::default() };
                let spans: Vec<(std::ops::Range<usize>, HighlightStyle)> = if path.to_lowercase().len() == path.len() {
                    lit.iter().filter_map(|&at| path.get(at..).and_then(|rest| rest.chars().next()).map(|one| (at..at + one.len_utf8(), marked))).collect()
                } else {
                    Vec::new()
                };
                let name = path.clone();
                let row = self
                    .fold_frame(look)
                    .id(SharedString::from(format!("np-folder-{path}")))
                    .aria_toggled(if ticked { Toggled::True } else { Toggled::False })
                    .when(kept, |row| row.opacity(0.8))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        to_line(this, window, cx);
                        this.tick_folder(of, name.clone(), cx);
                    }))
                    .child(tick(look, ticked, false))
                    .child(glyph("folder", look.length(token::SPACE_3) + look.length(token::SPACE_1) / 2., look.color(token::FG_MUTED)))
                    .child(div().min_w_0().truncate().child(StyledText::new(path.clone()).with_highlights(spans)))
                    .when(kept, |row| row.child(Self::meta(look, "picked")));
                labelled(keys_here(row), Role::CheckBox, path).into_any_element()
            }
            Line::Clone | Line::New => {
                let clone = line == Line::Clone;
                let (icon, said, meta) = if clone { ("link", "Clone a repository", "not on this Mac yet") } else { ("plus", "New empty repository", "a git repository on disk") };
                let row = self
                    .row_frame(look)
                    .id(if clone { "np-clone" } else { "np-new" })
                    .text_color(look.color(token::STATUS_HANDED_IN))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.active = at;
                        if clone {
                            this.toggle_clone(window, cx);
                        } else {
                            this.toggle_new(window, cx);
                        }
                    }))
                    .child(glyph(icon, look.length(token::SPACE_3) + look.length(token::SPACE_1) / 2., look.color(token::STATUS_HANDED_IN)))
                    .child(said)
                    .child(Self::meta(look, meta));
                labelled(keys_here(row), Role::Button, said).into_any_element()
            }
        }
    }

    /// Find under an opened repository: every folder of it, the letters found marked in each.
    fn folder_search(&self, of: usize, window: &Window, cx: &Context<Self>) -> AnyElement {
        let look = cx.look();
        let name = self.served.as_ref().and_then(|all| all.get(of)).map(|one| one.name.clone()).unwrap_or_default();
        let focused = self.boxes.folders.focus_handle(cx).is_focused(window);
        labelled(
            div()
                .id("np-folder-search")
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1) * 1.5)
                .h(look.length(token::H_CONTROL_LG))
                .ml(look.length(token::SPACE_12) - look.length(token::SPACE_1) / 2.)
                .mr(look.length(token::SPACE_3))
                .border_b(look.length(token::BORDER_WIDTH))
                .border_color(look.color(if focused { token::ACCENT } else { token::BORDER_SUBTLE }))
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(token::FG_MUTED))
                .child(glyph("search", look.length(token::SPACE_3), look.color(token::FG_MUTED)))
                .child(text_box(&self.boxes.folders).font_family(mono(look)).text_color(look.color(token::FG_DEFAULT)))
                .child(div().flex_none().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(format!("Every folder in {name}"))),
            Role::SearchInput,
            format!("Find a folder in {name}"),
        )
        .into_any_element()
    }

    /// The two fields under a row that makes a repository: what to clone or name it, and the folder it goes in.
    fn sub(look: &Look) -> Div {
        div()
            .flex_none()
            .flex()
            .gap(look.length(token::SPACE_2))
            .p(look.length(token::SPACE_3))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_dashed()
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_RAISED))
    }

    fn parent_field(&self, clone: bool, window: &Window, cx: &Context<Self>) -> AnyElement {
        let look = cx.look();
        let handle = if clone { &self.at.clone_parent } else { &self.at.new_parent };
        let parent = if clone { &self.clone_parent } else { &self.new_parent };
        let name = if clone { "Folder to clone into" } else { "Folder to make it in" };
        labelled(
            field(look, handle.is_focused(window))
                .id(if clone { "np-clone-parent" } else { "np-new-parent" })
                .flex_1()
                .track_focus(handle)
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, window, cx| this.open_parent(clone, window, cx)))
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "down" | "enter" | "space") {
                        cx.stop_propagation();
                        this.open_parent(clone, window, cx);
                    }
                }))
                .child(glyph("folder", look.length(token::SPACE_3) + look.length(token::SPACE_1) / 2., look.color(token::FG_MUTED)))
                .child(div().min_w_0().truncate().child(parent.as_deref().map(tilde).unwrap_or_default()))
                .child(div().ml_auto().child(glyph("chevron-down", look.length(token::SPACE_3) + look.length(token::SPACE_1) / 2., look.color(token::FG_SUBTLE)))),
            Role::Button,
            name,
        )
        .into_any_element()
    }

    fn clone_fields(&self, window: &Window, cx: &Context<Self>) -> AnyElement {
        let look = cx.look();
        let focused = self.boxes.clone.focus_handle(cx).is_focused(window);
        Self::sub(look)
            .child(labelled(
                field(look, focused).id("np-clone-url").flex_1().child(glyph("globe", look.length(token::SPACE_3) + look.length(token::SPACE_1) / 2., look.color(token::FG_MUTED))).child(text_box(&self.boxes.clone)),
                Role::TextInput,
                "Repository to clone",
            ))
            .child(self.parent_field(true, window, cx))
            .into_any_element()
    }

    fn new_fields(&self, window: &Window, cx: &Context<Self>) -> AnyElement {
        let look = cx.look();
        let focused = self.boxes.new.focus_handle(cx).is_focused(window);
        Self::sub(look)
            .child(labelled(field(look, focused).id("np-new-name").flex_1().child(text_box(&self.boxes.new)), Role::TextInput, "New repository name"))
            .child(self.parent_field(false, window, cx))
            .into_any_element()
    }

    // -- where its file is kept ----------------------------------------------------------------------------------

    fn place_section(&self, window: &Window, cx: &Context<Self>) -> Div {
        let look = cx.look();
        let focused = self.at.place.is_focused(window);
        let parts = PLACES.iter().enumerate().map(|(at, (place, said, icon))| {
            let on = self.place == *place;
            let place = *place;
            labelled(
                div()
                    .id(SharedString::from(format!("np-place-{at}")))
                    .flex_1()
                    .h(look.length(token::H_CONTROL_LG))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(look.length(token::SPACE_1) * 1.5)
                    .text_size(look.length(token::TEXT_XS))
                    .cursor_pointer()
                    .when(at + 1 < PLACES.len(), |part| part.border_r(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
                    .text_color(look.color(if on { token::FG_DEFAULT } else { token::FG_MUTED }))
                    .when(on, |part| part.bg(look.color(token::ACCENT_MUTED)).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))))
                    .aria_toggled(if on { Toggled::True } else { Toggled::False })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.place = place;
                        window.focus(&this.at.place, cx);
                        cx.notify();
                    }))
                    .child(glyph(icon, look.length(token::SPACE_3) + look.length(token::SPACE_1) / 2., look.color(if on { token::FG_DEFAULT } else { token::FG_MUTED })))
                    .child(*said),
                Role::RadioButton,
                *said,
            )
        });
        let seg = labelled(
            div()
                .id("np-place")
                .track_focus(&self.at.place)
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| this.place_key(event, cx)))
                .flex()
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(if focused { token::ACCENT } else { token::BORDER_DEFAULT }))
                .children(parts),
            Role::RadioGroup,
            "Where its file is kept",
        );
        let synced = (self.place == Place::Synced).then(|| {
            let focused = self.boxes.synced.focus_handle(cx).is_focused(window);
            labelled(
                field(look, focused).id("np-synced").child(glyph("folder", look.length(token::SPACE_3) + look.length(token::SPACE_1) / 2., look.color(token::FG_MUTED))).child(text_box(&self.boxes.synced)),
                Role::TextInput,
                "Synced folder",
            )
        });
        section(look).child(caption(Some("hard-drive"), "Where its file is kept", look)).child(seg).children(synced)
    }

    // -- settings -----------------------------------------------------------------------------------------------

    fn settings(&self, window: &Window, cx: &Context<Self>) -> Div {
        let look = cx.look();
        let tiles = div()
            .flex()
            .gap(look.length(token::SPACE_2))
            .child(self.tile(false, "zap", "Use defaults", "Your first answer to each question becomes this project's setting", window, cx))
            .child(self.tile(true, "sliders-horizontal", "Customize", "Set each one now", window, cx));
        let small = |said: &'static str| div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)).child(said);
        let mut part = section(look).child(caption(Some("sliders-horizontal"), "Settings", look)).child(labelled(tiles.id("np-tiles"), Role::RadioGroup, "Settings"));
        if self.custom {
            part = part.child(small("Grey is what every project uses"));
            for group in &values::GROUPS {
                let rows: Vec<AnyElement> = group.keys.iter().map(|key| self.setting(*key, window, cx)).collect();
                part = part.child(self.group(group.icon, group.title, rows, look));
            }
            part = part.child(self.group("globe", "Sharing", vec![self.share_row(window, cx)], look));
        } else {
            part = part.child(self.learnt(look));
        }
        part
    }

    fn tile(&self, custom: bool, icon: &'static str, said: &'static str, line: &'static str, window: &Window, cx: &Context<Self>) -> AnyElement {
        let look = cx.look();
        let on = self.custom == custom;
        let handle = if custom { &self.at.custom } else { &self.at.defaults };
        let focused = handle.is_focused(window);
        let accent = look.color(token::ACCENT);
        let mut shadows = Vec::new();
        if on || focused {
            shadows.push(gpui::BoxShadow { color: accent, offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: look.length(token::BORDER_WIDTH), inset: false });
        }
        if on {
            shadows.extend(look.shadows(token::SHADOW_CARD));
        }
        let mark = div()
            .size(look.length(token::H_CONTROL_LG))
            .flex()
            .items_center()
            .justify_center()
            .rounded(look.length(token::RADIUS_MD))
            .bg(look.color(if on { token::ACCENT } else { token::BG_SUNKEN }))
            .child(glyph(icon, look.length(token::SPACE_4) + look.length(token::SPACE_1) / 2., look.color(if on { token::FG_INVERSE } else { token::FG_MUTED })));
        let body = div()
            .relative()
            .size_full()
            .p(look.length(token::SPACE_3))
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_LG))
            .when(on, |body| body.bg(look.color(token::ACCENT_FAINT)))
            .child(mark)
            .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(said))
            .child(div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)).child(line))
            .when(on, |body| {
                let inset = look.length(token::SPACE_2) + look.length(token::SPACE_1) / 2.;
                body.child(div().absolute().top(inset).right(inset).size(look.length(token::SPACE_2)).rounded_full().bg(accent))
            });
        labelled(
            div()
                .id(if custom { "np-tile-custom" } else { "np-tile-defaults" })
                .flex_1()
                .flex_basis(px(0.))
                .min_w_0()
                .track_focus(handle)
                .cursor_pointer()
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if on || focused { accent } else { look.color(token::BORDER_DEFAULT) })
                .bg(look.color(token::BG_RAISED))
                .shadow(shadows)
                .aria_toggled(if on { Toggled::True } else { Toggled::False })
                .on_click(cx.listener(move |this, _, window, cx| {
                    let handle = if custom { this.at.custom.clone() } else { this.at.defaults.clone() };
                    window.focus(&handle, cx);
                    this.set_custom(custom, cx);
                }))
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| this.tile_key(custom, event, window, cx)))
                .child(body),
            Role::RadioButton,
            said,
        )
        .into_any_element()
    }

    /// Use defaults: what the project learns, and what it takes as every project does.
    fn learnt(&self, look: &Look) -> Div {
        let line = |icon: &'static str, said: &'static str, how: &'static str| {
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1) * 1.5)
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .child(glyph(icon, look.length(token::SPACE_3) + look.length(token::SPACE_1) / 4., look.color(token::FG_MUTED)))
                .child(said)
                .child(div().ml_auto().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(how))
        };
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1) * 1.5)
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_dashed()
            .border_color(look.color(token::BORDER_DEFAULT))
            .child(line("git-merge", "How work lands", "asked on the first Job, then kept"))
            .child(line("sliders-horizontal", "Everything else", "what every project uses"))
    }

    fn group(&self, icon: &'static str, title: &'static str, rows: Vec<AnyElement>, look: &Look) -> Div {
        let last = rows.len().saturating_sub(1);
        div()
            .px(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_RAISED))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1) * 1.5)
                    .h(look.length(token::H_ROW))
                    .text_size(look.length(token::TEXT_2XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_MUTED))
                    .border_b(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .child(glyph(icon, look.length(token::SPACE_3), look.color(token::FG_MUTED)))
                    .child(title),
            )
            .children(rows.into_iter().enumerate().map(|(at, row)| {
                div()
                    .when(at < last, |row| row.border_b(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
                    .child(row)
            }))
    }

    fn set_row(look: &Look, label: &'static str, note: Option<&'static str>) -> Div {
        div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .min_h(look.length(token::SPACE_8) + look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_SM))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(label)
                    .children(note.map(|note| div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_SUBTLE)).child(note))),
            )
    }

    /// One setting: its label, the modified dot where this project changes it, and its pick, grey and dashed
    /// while it is what every project uses.
    fn setting(&self, key: Key, window: &Window, cx: &Context<Self>) -> AnyElement {
        let look = cx.look();
        let Some(handle) = self.at.settings.get(&key) else { return div().into_any_element() };
        let focused = handle.is_focused(window) || self.menu_open_on(key);
        let shown = self.shown(key);
        let changed = shown.as_ref().is_some_and(|(_, changed)| *changed);
        let inherited = shown.is_some() && !changed && !key.own();
        let ring = look.color(token::ACCENT);
        let pick = div()
            .id(SharedString::from(format!("np-set-{key:?}")))
            .track_focus(handle)
            .flex_none()
            .h(look.length(token::H_CONTROL))
            .px(look.length(token::SPACE_2))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1) * 1.5)
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(if focused { ring } else { look.color(token::BORDER_DEFAULT) })
            .text_size(look.length(token::TEXT_XS))
            .cursor_pointer()
            .when(inherited, |pick| pick.border_dashed().text_color(look.color(token::FG_SUBTLE)))
            .when(!inherited, |pick| pick.bg(look.color(token::BG_SUNKEN)))
            .on_click(cx.listener(move |this, _, window, cx| this.open_setting(key, window, cx)))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "down" | "enter" | "space") {
                    cx.stop_propagation();
                    this.open_setting(key, window, cx);
                }
            }))
            .when(inherited, |pick| pick.tooltip(tip("What every project uses")))
            .map(|pick| match &shown {
                Some((word, _)) => pick.child(key.said(word, &self.models)),
                None => pick.child(bone(look, look.length(token::SPACE_12))),
            })
            .child(glyph("chevron-down", look.length(token::SPACE_3), look.color(token::FG_SUBTLE)));
        let dot = changed.then(|| {
            labelled(
                div().id(SharedString::from(format!("np-mod-{key:?}"))).flex_none().size(look.length(token::SPACE_1) * 1.5).rounded_full().bg(look.color(token::SETTING_MODIFIED)).tooltip(tip("Set for this project")),
                Role::Image,
                format!("{} set for this project", key.label()),
            )
        });
        let row = Self::set_row(look, key.label(), key.note()).children(dot).child(labelled(pick, Role::Button, key.label()));
        // Custom, picked among the trackers, opens its name field right under the row.
        if key == Key::Tracker && self.tracker.read(cx).is_shown() {
            return div().flex().flex_col().gap(look.length(token::SPACE_2)).pb(look.length(token::SPACE_3)).child(row).child(self.tracker.clone()).into_any_element();
        }
        row.into_any_element()
    }

    fn share_row(&self, window: &Window, cx: &Context<Self>) -> AnyElement {
        let look = cx.look();
        let on = self.share;
        let focused = self.at.share.is_focused(window);
        let nudge = look.length(token::SPACE_1) / 2.;
        let knob = look.length(token::SPACE_3) + nudge;
        let wide = look.length(token::SPACE_8) - nudge;
        let switch = div()
            .id("np-share")
            .track_focus(&self.at.share)
            .relative()
            .flex_none()
            .w(wide)
            .h(look.length(token::SPACE_4) + nudge)
            .rounded_full()
            .cursor_pointer()
            .bg(look.color(if on { token::ACCENT } else { token::BORDER_STRONG }))
            .when(focused, |switch| {
                switch.shadow(vec![gpui::BoxShadow { color: look.color(token::ACCENT), offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: look.length(token::BORDER_WIDTH), inset: false }])
            })
            .aria_toggled(if on { Toggled::True } else { Toggled::False })
            .on_click(cx.listener(|this, _, _, cx| {
                this.share = !this.share;
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if matches!(event.keystroke.key.as_str(), "space" | "enter") {
                    cx.stop_propagation();
                    this.share = !this.share;
                    cx.notify();
                }
            }))
            .child(div().absolute().top(nudge).left(if on { wide - knob - nudge } else { nudge }).size(knob).rounded_full().bg(look.color(if on { token::FG_INVERSE } else { token::FG_MUTED })));
        Self::set_row(look, "Share with your other Macs", None).child(labelled(switch, Role::Switch, "Share with your other Macs")).into_any_element()
    }

    // -- the foot ------------------------------------------------------------------------------------------------

    fn foot(&self, window: &Window, cx: &Context<Self>) -> Div {
        let look = cx.look();
        let ring = look.color(token::ACCENT);
        let refusal = self.refused.clone().map(|why| labelled(div().id("np-refused").text_color(look.color(token::ERROR)).child(why.clone()), Role::Alert, why));
        let cancel = Btn::new("np-cancel", "Cancel", Weight::Secondary)
            .draw(look, cx)
            .track_focus(&self.at.cancel)
            .when(self.at.cancel.is_focused(window), |button| button.border_color(ring))
            .on_click(cx.listener(|this, _, window, cx| this.close(window, cx)));
        let ready = self.named(cx);
        let create = Btn::new("np-create", "Create project", Weight::Primary)
            .pending(self.sending)
            .disabled(!ready)
            .draw(look, cx)
            .shadow(look.shadows(token::SHADOW_PRIMARY))
            .track_focus(&self.at.create)
            .when(self.at.create.is_focused(window), |button| button.border_color(look.color(token::FG_DEFAULT)))
            .when(ready && !self.sending, |button| button.on_click(cx.listener(|this, _, window, cx| this.create(window, cx))))
            .child(kbd("⌘↩", look));
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .bg(look.color(token::BG_RAISED))
            .child(div().flex_1().min_w_0().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).children(refusal))
            .child(labelled(cancel, Role::Button, "Cancel"))
            .child(labelled(create, Role::Button, "Create project"))
    }
}
