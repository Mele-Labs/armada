//! Saving: the request into its collection (⌘S), a new or reopened request asked for a name and a collection on an inline row,
//! and a collection's shared headers opened on the right and written whole. Fleet writes only the person's own file, and
//! refuses while that file will not read, so no Save is offered then.

use gpui::{div, prelude::*, Focusable as _, svg, AnyElement, Context, Entity, FocusHandle, FontWeight, Role, SharedString, Subscription, Task, Window};
use gpui_component::input::{Input, InputEvent, InputState};
use ipc::{RequestCollection, RequestPlace, RequestSaved, RequestToSave, SavedRequest};

use super::view::field;
use super::{headers_of, model, wire, ApiClient, HeaderRow, Open, Part, Pick, Rows};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::settings::view::{refusal, ring_colour, tip};
use crate::titlebar::parts::mono;

fn draft_key(job: &str) -> String {
    format!("request:{job}:save-name")
}

/// The inline row under the request line that asks for a name and a collection.
pub(crate) struct SaveRow {
    pub name: Entity<InputState>,
    /// The collection picked, by its id.
    pub collection: Option<String>,
    pub collection_focus: FocusHandle,
    pub save_focus: FocusHandle,
    _subs: Vec<Subscription>,
}

/// A collection's shared headers open on the right.
pub(crate) struct CollectionEdit {
    pub id: String,
    pub name: Entity<InputState>,
    pub rows: Vec<HeaderRow>,
    pub refused: Option<String>,
    pub task: Option<Task<()>>,
    pub add_focus: FocusHandle,
    pub save_focus: FocusHandle,
    _subs: Vec<Subscription>,
}

/// Why a save of the person's own file is off, in the words of the tooltip.
const LOCKED: &str = "Your own requests file would not read";

impl ApiClient {
    // ---- what a save needs -----------------------------------------------------------------------

    /// Why saving is off for now: the file would not read, a header name is wrong, or there is no address.
    pub(crate) fn save_off(&self, cx: &gpui::App) -> Option<String> {
        if model::yours_unread(self.requests().map(|read| read.as_ref())).is_some() {
            return Some(LOCKED.to_string());
        }
        if self.save_task.is_some() {
            return Some("Saving".to_string());
        }
        if self.bad_header(cx).is_some() {
            return Some("A header name has no spaces".to_string());
        }
        if self.address_text(cx).trim().is_empty() {
            return Some("The request has no address".to_string());
        }
        None
    }

    /// Why the inline row's Save is off: as the control's, and the name and the collection it still lacks.
    fn save_new_off(&self, cx: &gpui::App) -> Option<String> {
        if let Some(why) = self.save_off(cx) {
            return Some(why);
        }
        let row = self.saving.as_ref()?;
        if row.name.read(cx).value().trim().is_empty() {
            return Some("Name the request".to_string());
        }
        if row.collection.is_none() {
            return Some("Pick a collection".to_string());
        }
        None
    }

    /// Whether the request line is followed by a row: a saved request, a send or a run opened, or an address typed.
    fn has_crumb(&self, cx: &gpui::App) -> bool {
        self.collection_edit.is_none() && (self.saving.is_some() || !matches!(self.open, Open::Blank) || !self.address_text(cx).trim().is_empty())
    }

    // ---- ⌘S and the Save control -----------------------------------------------------------------

    /// ⌘S and Save: what is open says what it saves. A collection's headers, the environment being edited, a saved request back
    /// into its collection, and any other request asks for a name and a collection first.
    pub(crate) fn save_pressed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.collection_edit.is_some() {
            return self.save_collection(window, cx);
        }
        if self.part == Part::Environment {
            return self.save_active_environment(window, cx);
        }
        // A press that cannot save says why in the control's tooltip.
        if self.save_off(cx).is_some() {
            return;
        }
        match &self.open {
            Open::Saved(..) if self.saved().is_some() => self.save_saved(window, cx),
            _ if self.saving.is_some() => match self.save_new_off(cx) {
                None => self.commit_new(window, cx),
                Some(_) => self.focus_save_row(window, cx),
            },
            _ => self.open_save_row(window, cx),
        }
    }

    fn focus_save_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.saving.as_ref() else { return };
        let (name, empty) = (row.name.clone(), row.name.read(cx).value().trim().is_empty());
        if empty || row.collection.is_some() {
            name.update(cx, |input, cx| input.focus(window, cx));
        } else {
            window.focus(&row.collection_focus, cx);
        }
    }

    /// The saved request back into its collection: the whole collection, with this request replaced by what is in the editor.
    fn save_saved(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(collection), Some(saved)) = (self.collection().map(|one| one.item.clone()), self.saved().cloned()) else { return };
        let body = Some(self.body_text(cx)).filter(|body| !body.is_empty());
        let request = SavedRequest {
            id: saved.id.clone(),
            name: saved.name,
            method: self.method.clone(),
            url: self.address_text(cx).trim().to_string(),
            headers: self.own_headers(cx),
            body,
        };
        let mut kept = collection;
        kept.requests = kept.requests.into_iter().map(|one| if one.id == request.id { request.clone() } else { one }).collect();
        self.save_said = None;
        let job = self.job.to_string();
        self.save_task = self.ask(
            kept,
            move |port, cx, kept| wire::save_collection(port, &job, kept, cx),
            |store, kept| store.save_collection(kept),
            |this, answer, window, cx| {
                this.save_task = None;
                match answer {
                    Ok(read) => this.landed(Ok(read), window, cx),
                    Err(said) => this.save_said = Some(said),
                }
                cx.notify();
            },
            window,
            cx,
        );
        cx.notify();
    }

    /// The inline row: a name, and the collection the person's own file has first, the repository's where it has none of its own.
    fn open_save_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let kept = crate::drafts::kept(&draft_key(&self.job), cx).unwrap_or_default();
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Name"));
        name.update(cx, |input, cx| input.set_value(kept, window, cx));
        let subs = vec![cx.subscribe_in(&name, window, |this, input, event: &InputEvent, window, cx| match event {
            InputEvent::Change => {
                crate::drafts::keep(&draft_key(&this.job), input.read(cx).value().as_ref(), cx);
                cx.notify();
            }
            InputEvent::PressEnter { .. } => this.save_pressed(window, cx),
            InputEvent::Focus | InputEvent::Blur => cx.notify(),
        })];
        let collection = self.requests().and_then(|read| {
            read.collections.iter().find(|one| one.place == RequestPlace::Yours).or_else(|| read.collections.first()).map(|one| one.item.id.clone())
        });
        self.saving = Some(SaveRow { name: name.clone(), collection, collection_focus: cx.focus_handle().tab_stop(true), save_focus: cx.focus_handle().tab_stop(true), _subs: subs });
        self.save_said = None;
        name.update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    /// A new or reopened request into the collection picked, under the name given. The read comes again and selects it.
    fn commit_new(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.saving.as_ref() else { return };
        let (name, Some(collection)) = (row.name.read(cx).value().trim().to_string(), row.collection.clone()) else { return };
        let request = RequestToSave {
            name,
            method: self.method.clone(),
            url: self.address_text(cx).trim().to_string(),
            headers: self.own_headers(cx),
            body: Some(self.body_text(cx)).filter(|body| !body.is_empty()),
        };
        self.save_said = None;
        let job = self.job.to_string();
        self.save_task = self.ask(
            (collection, request),
            move |port, cx, (collection, request)| wire::save_request(port, &job, collection, request, cx),
            |store, (collection, request)| store.save_request(&collection, request),
            |this, answer, window, cx| {
                this.save_task = None;
                match answer {
                    Ok(saved) => this.request_saved(saved, window, cx),
                    Err(said) => this.save_said = Some(said),
                }
                cx.notify();
            },
            window,
            cx,
        );
        cx.notify();
    }

    fn request_saved(&mut self, saved: RequestSaved, window: &mut Window, cx: &mut Context<Self>) {
        self.saving = None;
        crate::drafts::keep(&draft_key(&self.job), "", cx);
        self.select_after = Some((saved.collection_id, saved.request_id));
        self.read_requests(window, cx);
    }

    /// The saved request in the list, open and under the keys, once the read that holds it has landed.
    pub(crate) fn select_saved(&mut self, collection: &str, request: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.open = Open::Saved(collection.to_string(), request.to_string());
        if matches!(self.filter, super::model::Filter::Agent | super::model::Filter::Mine) {
            self.filter = super::model::Filter::ThisJob;
        }
        self.expand_runs(window, cx);
        self.relist(false);
        let at = self.lines.iter().position(|line| self.is_open(line));
        if let Some(at) = at {
            self.cursor = Some(at);
            self.list.scroll_to_reveal_item(at);
        }
    }

    /// Esc: an inline form or a confirm goes away first. False where none was open.
    pub(crate) fn back_out(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.back_out_environment(window, cx) {
            return true;
        }
        if self.saving.is_some() {
            self.saving = None;
            self.focus_address(window, cx);
            cx.notify();
            return true;
        }
        if self.collection_edit.is_some() {
            self.collection_edit = None;
            self.relist(false);
            self.focus_address(window, cx);
            cx.notify();
            return true;
        }
        false
    }

    // ---- a collection's shared headers -----------------------------------------------------------

    /// A collection heading pressed: its name and shared headers open on the right.
    pub(crate) fn open_collection(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(placed) = self.requests().and_then(|read| read.collections.get(at)).cloned() else { return };
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Name"));
        name.update(cx, |input, cx| input.set_value(placed.item.name.clone(), window, cx));
        let subs = vec![cx.subscribe_in(&name, window, |this, _, event: &InputEvent, window, cx| match event {
            InputEvent::PressEnter { .. } => this.save_collection(window, cx),
            _ => cx.notify(),
        })];
        let rows = self.rows_of(Rows::Collection, &placed.item.headers, window, cx);
        self.collection_edit = Some(CollectionEdit {
            id: placed.item.id.clone(),
            name: name.clone(),
            rows,
            refused: None,
            task: None,
            add_focus: cx.focus_handle().tab_stop(true),
            save_focus: cx.focus_handle().tab_stop(true),
            _subs: subs,
        });
        self.saving = None;
        self.cursor = self.lines.iter().position(|line| matches!(line, model::Line::Collection(one) if *one == at));
        name.update(cx, |input, cx| input.focus(window, cx));
        self.relist(false);
        cx.notify();
    }

    /// A read that no longer lists the open collection puts its headers away.
    pub(crate) fn settle_collection_edit(&mut self) {
        let gone = match (&self.collection_edit, self.requests()) {
            (Some(edit), Some(read)) => !read.collections.iter().any(|one| one.item.id == edit.id),
            _ => false,
        };
        if gone {
            self.collection_edit = None;
        }
    }

    /// Why the collection's Save is off: the file would not read, a header name is wrong, or the name is blank.
    pub(crate) fn collection_off(&self, cx: &gpui::App) -> Option<String> {
        let edit = self.collection_edit.as_ref()?;
        if model::yours_unread(self.requests().map(|read| read.as_ref())).is_some() {
            return Some(LOCKED.to_string());
        }
        if edit.task.is_some() {
            return Some("Saving".to_string());
        }
        let names: Vec<String> = edit.rows.iter().map(|row| row.name.read(cx).value().to_string()).collect();
        if model::bad_header(&names).is_some() {
            return Some("A header name has no spaces".to_string());
        }
        if edit.name.read(cx).value().trim().is_empty() {
            return Some("Name the collection".to_string());
        }
        None
    }

    /// The whole collection, with its name and shared headers as typed and its requests as Fleet lists them.
    pub(crate) fn save_collection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.collection_off(cx).is_some() {
            return;
        }
        let Some(edit) = self.collection_edit.as_ref() else { return };
        let Some(held) = self.requests().and_then(|read| read.collections.iter().find(|one| one.item.id == edit.id)).map(|one| one.item.clone()) else { return };
        let kept = RequestCollection { id: held.id, name: edit.name.read(cx).value().trim().to_string(), headers: headers_of(&edit.rows, cx), requests: held.requests };
        if let Some(edit) = self.collection_edit.as_mut() {
            edit.refused = None;
        }
        let job = self.job.to_string();
        let task = self.ask(
            kept,
            move |port, cx, kept| wire::save_collection(port, &job, kept, cx),
            |store, kept| store.save_collection(kept),
            |this, answer, window, cx| {
                if let Some(edit) = this.collection_edit.as_mut() {
                    edit.task = None;
                    if let Err(said) = &answer {
                        edit.refused = Some(said.clone());
                    }
                }
                if let Ok(read) = answer {
                    this.landed(Ok(read), window, cx);
                }
                cx.notify();
            },
            window,
            cx,
        );
        if let Some(edit) = self.collection_edit.as_mut() {
            if task.is_some() {
                edit.task = task;
            }
        }
        cx.notify();
    }

    // ---- drawing ---------------------------------------------------------------------------------

    /// The row under the request line: the crumb of a saved request on the left, Save at the right, Fleet's words for a refused
    /// save under it, and the inline row for a request not saved yet.
    pub(crate) fn crumb(&self, look: &Look, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.has_crumb(cx) {
            return None;
        }
        let place = self.collection().zip(self.saved()).map(|(collection, saved)| (collection.item.name.clone(), saved.name.clone()));
        let crumb = place.map(|(collection, request)| {
            let said = format!("{collection}, {request}");
            labelled(
                div()
                    .id("api-crumb-place")
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .text_color(look.color(token::FG_MUTED))
                    .child(SharedString::from(collection))
                    .child(svg().flex_none().path(icon_path("chevron-right")).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_SUBTLE)))
                    .child(div().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(request))),
                Role::Label,
                said,
            )
        });
        let row = div()
            .id("api-crumb")
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_4))
            .px(look.length(token::SPACE_4))
            .pb(look.length(token::SPACE_3))
            .children(crumb)
            .child(div().flex_1())
            .child(self.save_control(look, cx));
        let said = self.save_said.clone().map(|said| {
            div().id("api-save-said").flex_none().px(look.length(token::SPACE_4)).pb(look.length(token::SPACE_3)).child(refusal(look, said))
        });
        Some(div().flex_none().flex().flex_col().child(row).children(said).children(self.save_row_view(look, window, cx)).into_any_element())
    }

    /// The small `Save ⌘S` at the right of the crumb row. Off, it says why.
    fn save_control(&self, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let why = self.save_off(cx);
        let off = why.is_some();
        labelled(
            div()
                .id("api-save")
                .track_focus(&self.save_focus)
                .tab_stop(!off)
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL_SM))
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .text_size(look.length(token::TEXT_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .whitespace_nowrap()
                .map(|button| {
                    if off {
                        button.opacity(0.5).text_color(look.color(token::FG_MUTED))
                    } else {
                        button
                            .text_color(look.color(token::FG_DEFAULT))
                            .cursor_pointer()
                            .hover(|style| style.bg(look.color(token::BG_HOVER)))
                            .focus_visible(|style| style.border_color(ring_colour(look)))
                            .on_click(cx.listener(|this, _, window, cx| this.save_pressed(window, cx)))
                    }
                })
                .when_some(why.filter(|why| why != "Saving"), |button, why| button.tooltip(tip(why)))
                .child("Save")
                .child(div().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(crate::keymap::shown("api_client_save"))),
            Role::Button,
            "Save",
        )
        .into_any_element()
    }

    /// Name, collection and Save, in a row under the crumb, for a request not saved yet.
    fn save_row_view(&self, look: &Look, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let row = self.saving.as_ref()?;
        let focused = row.name.read(cx).focus_handle(cx).is_focused(window);
        let collection = row.collection.as_ref().and_then(|id| self.requests()?.collections.iter().find(|one| one.item.id == *id)).map(|one| one.item.name.clone());
        let why = self.save_new_off(cx);
        let off = why.is_some();
        let name = labelled(field(look, "api-save-name", focused).child(Input::new(&row.name).appearance(false)), Role::TextInput, "Request name");
        let picked = collection.is_some();
        let picker = labelled(
            div()
                .id("api-save-collection")
                .track_focus(&row.collection_focus)
                .tab_stop(true)
                .flex_none()
                .flex()
                .items_center()
                .h(look.length(token::H_CONTROL))
                .w(look.length(token::SPACE_12) * 3.)
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_BASE))
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(if picked { token::FG_DEFAULT } else { token::FG_MUTED }))
                .truncate()
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .focus_visible(|style| style.border_color(ring_colour(look)))
                .tooltip(tip("Collection"))
                .on_click(cx.listener(|this, event: &gpui::ClickEvent, window, cx| this.pick(Pick::Collection, event.is_keyboard(), window, cx)))
                .child(SharedString::from(collection.unwrap_or_else(|| "Collection".to_string()))),
            Role::Button,
            "Collection",
        );
        let button = labelled(
            div()
                .id("api-save-commit")
                .track_focus(&row.save_focus)
                .tab_stop(!off)
                .flex_none()
                .flex()
                .items_center()
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_4))
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(token::ACCENT))
                .text_color(look.color(token::TEXT_ON_ACCENT))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_size(look.length(token::TEXT_XS))
                .map(|button| {
                    if off {
                        button.opacity(0.5)
                    } else {
                        button
                            .cursor_pointer()
                            .hover(|style| style.bg(look.color(token::ACCENT_HOVER)))
                            .focus_visible(|style| style.border_2().border_color(ring_colour(look)))
                            .on_click(cx.listener(|this, _, window, cx| this.commit_new(window, cx)))
                    }
                })
                .when_some(why.filter(|why| why != "Saving"), |button, why| button.tooltip(tip(why)))
                .child("Save"),
            Role::Button,
            "Save request",
        );
        Some(
            labelled(
                div()
                    .id("api-save-row")
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_4))
                    .pb(look.length(token::SPACE_3))
                    .child(name)
                    .child(picker)
                    .child(button),
                Role::Group,
                "Save to a collection",
            )
            .into_any_element(),
        )
    }

    /// A collection's name and its shared headers, in place of the request, with Save writing the whole collection.
    pub(crate) fn collection_view(&self, look: &Look, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let edit = self.collection_edit.as_ref()?;
        let placed = self.requests()?.collections.iter().find(|one| one.item.id == edit.id)?;
        let focused = edit.name.read(cx).focus_handle(cx).is_focused(window);
        let kept = match (placed.place, placed.hides) {
            (RequestPlace::Repository, _) => "In the repository",
            (_, true) => "Yours, and hides the repository's",
            _ => "Yours",
        };
        let why = self.collection_off(cx);
        let off = why.is_some();
        let kept_line = labelled(
            div()
                .id("api-collection-kept")
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_color(look.color(token::FG_MUTED))
                .child(div().flex_none().text_color(look.color(token::FG_SUBTLE)).child("Kept"))
                .children((placed.place == RequestPlace::Repository).then(|| {
                    div().id("api-collection-repository").flex_none().child(svg().flex_none().path(icon_path("folder-git-2")).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_MUTED))).tooltip(tip("The repository's"))
                }))
                .child(kept),
            Role::Label,
            format!("Kept {kept}"),
        );
        let header_rows = self.header_rows(look, Rows::Collection, window, cx);
        let add = self.add_header_button(look, Rows::Collection, cx);
        let save = labelled(
            div()
                .id("api-collection-save")
                .track_focus(&edit.save_focus)
                .tab_stop(!off)
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_4))
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(token::ACCENT))
                .text_color(look.color(token::TEXT_ON_ACCENT))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_size(look.length(token::TEXT_XS))
                .map(|button| {
                    if off {
                        button.opacity(0.5)
                    } else {
                        button
                            .cursor_pointer()
                            .hover(|style| style.bg(look.color(token::ACCENT_HOVER)))
                            .focus_visible(|style| style.border_2().border_color(ring_colour(look)))
                            .on_click(cx.listener(|this, _, window, cx| this.save_collection(window, cx)))
                    }
                })
                .when_some(why.filter(|why| why != "Saving"), |button, why| button.tooltip(tip(why)))
                .child("Save")
                .child(div().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).opacity(0.7).child(crate::keymap::shown("api_client_save"))),
            Role::Button,
            "Save collection",
        );
        Some(
            labelled(
                div()
                    .id("api-collection")
                    .flex_none()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_4))
                    .child(labelled(field(look, "api-collection-name", focused).child(Input::new(&edit.name).appearance(false)), Role::TextInput, "Collection name"))
                    .child(kept_line)
                    .child(
                        labelled(
                            div()
                                .id("api-collection-headers")
                                .flex()
                                .flex_col()
                                .gap(look.length(token::SPACE_2))
                                .child(labelled(
                                    div().id("api-collection-headers-title").text_size(look.length(token::TEXT_2XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_SUBTLE)).child("SHARED HEADERS"),
                                    Role::Heading,
                                    "Shared headers",
                                ))
                                .children(header_rows)
                                .child(div().flex().child(add)),
                            Role::Group,
                            "Shared headers",
                        ),
                    )
                    .child(div().flex().items_center().gap(look.length(token::SPACE_3)).child(save))
                    .children(edit.refused.clone().map(|said| div().id("api-collection-refused").child(refusal(look, said)))),
                Role::Group,
                "Collection",
            )
            .into_any_element(),
        )
    }
}
