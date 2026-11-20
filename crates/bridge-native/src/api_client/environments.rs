//! The Environment part: the repository's environments, each a group of variables the person edits and saves whole, with a
//! secret typed once and sent once. Fleet keeps a secret's value in the Keychain and answers only that it is set, so a secret
//! that is set reads `Set` with a Replace that opens an empty field; a value typed is cleared the moment the save call is
//! made and is never kept, drawn or written to a log. The Job's servers are listed under the editable ones, read only.

use gpui::{div, prelude::*, Focusable as _, svg, AnyElement, Context, Entity, FocusHandle, FontWeight, Role, SharedString, Subscription, Task, Toggled, Window};
use gpui_component::input::{Input, InputEvent, InputState};
use ipc::{Placed, RequestEnvironmentRead, RequestEnvironmentWrite, RequestPlace, VariableRead, VariableWrite};

use super::view::field;
use super::{model, wire, ApiClient, Part};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::settings::view::{ghost, refusal, ring_colour, switch_track, tip};
use crate::titlebar::parts::mono;

/// One variable of an environment form: a name, a value and whether it is a secret.
pub(crate) struct VarRow {
    pub name: Entity<InputState>,
    pub value: Entity<InputState>,
    pub secret: bool,
    /// A secret the Keychain already holds a value for: drawn as `Set` until Replace opens the field.
    pub set: bool,
    pub replacing: bool,
    pub switch_focus: FocusHandle,
    pub replace_focus: FocusHandle,
    pub remove_focus: FocusHandle,
    _subs: Vec<Subscription>,
}

/// One environment as a form: the person's own or the repository's, or one being added.
pub(crate) struct EnvForm {
    pub id: u64,
    /// The environment as Fleet read it, for a form that is not new; a read that differs replaces the form.
    pub read: Option<Placed<RequestEnvironmentRead>>,
    pub name: Entity<InputState>,
    pub rows: Vec<VarRow>,
    pub refused: Option<String>,
    pub deleting: bool,
    pub task: Option<Task<()>>,
    pub save_focus: FocusHandle,
    pub delete_focus: FocusHandle,
    pub confirm_focus: FocusHandle,
    pub cancel_focus: FocusHandle,
    _subs: Vec<Subscription>,
}

impl EnvForm {
    fn label(&self, cx: &gpui::App) -> String {
        match &self.read {
            Some(read) => read.item.name.clone(),
            None => self.name.read(cx).value().trim().to_string(),
        }
    }
}

fn name_draft(job: &str) -> String {
    format!("request:{job}:environment-name")
}

fn row_blank(row: &VarRow, cx: &gpui::App) -> bool {
    row.name.read(cx).value().is_empty() && row.value.read(cx).value().is_empty() && !row.set
}

impl ApiClient {
    // ---- the forms -------------------------------------------------------------------------------

    fn form_mut(&mut self, id: u64) -> Option<&mut EnvForm> {
        if let Some(adding) = self.adding.as_mut().filter(|form| form.id == id) {
            return Some(adding);
        }
        self.forms.iter_mut().find(|form| form.id == id)
    }

    #[allow(clippy::too_many_arguments)]
    fn var_row(&self, form: u64, name: &str, value: &str, secret: bool, set: bool, replacing: bool, window: &mut Window, cx: &mut Context<Self>) -> VarRow {
        let name_state = cx.new(|cx| InputState::new(window, cx).placeholder("base"));
        name_state.update(cx, |input, cx| input.set_value(name.to_string(), window, cx));
        let value_state = cx.new(|cx| InputState::new(window, cx));
        value_state.update(cx, |input, cx| {
            input.set_value(value.to_string(), window, cx);
            input.set_masked(secret, window, cx);
        });
        let typed = move |this: &mut ApiClient, event: &InputEvent, window: &mut Window, cx: &mut Context<ApiClient>| match event {
            InputEvent::Change => this.env_edited(form, window, cx),
            InputEvent::PressEnter { .. } => this.save_environment(form, window, cx),
            InputEvent::Focus => {
                this.env_active = Some(form);
                cx.notify();
            }
            InputEvent::Blur => cx.notify(),
        };
        let subs = vec![
            cx.subscribe_in(&name_state, window, move |this, _, event: &InputEvent, window, cx| typed(this, event, window, cx)),
            cx.subscribe_in(&value_state, window, move |this, _, event: &InputEvent, window, cx| typed(this, event, window, cx)),
        ];
        VarRow {
            name: name_state,
            value: value_state,
            secret,
            set,
            replacing,
            switch_focus: cx.focus_handle().tab_stop(true),
            replace_focus: cx.focus_handle().tab_stop(true),
            remove_focus: cx.focus_handle().tab_stop(true),
            _subs: subs,
        }
    }

    fn blank_row(&self, form: u64, window: &mut Window, cx: &mut Context<Self>) -> VarRow {
        self.var_row(form, "", "", false, false, false, window, cx)
    }

    /// A form for an environment Fleet read, or an empty one to add; either ends with a row to type into.
    fn env_form(&mut self, read: Option<&Placed<RequestEnvironmentRead>>, window: &mut Window, cx: &mut Context<Self>) -> EnvForm {
        let id = self.env_next;
        self.env_next += 1;
        let mut rows: Vec<VarRow> = read
            .map(|read| {
                read.item
                    .variables
                    .iter()
                    .map(|variable| match variable {
                        VariableRead::Plain { name, value } => self.var_row(id, name, value, false, false, false, window, cx),
                        VariableRead::Secret { name, set } => self.var_row(id, name, "", true, *set, false, window, cx),
                    })
                    .collect()
            })
            .unwrap_or_default();
        rows.push(self.blank_row(id, window, cx));
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("test"));
        // The name of an environment being added is kept as the body is, so closing the tab does not lose it.
        let is_new = read.is_none();
        if is_new {
            if let Some(kept) = crate::drafts::kept(&name_draft(&self.job), cx) {
                name.update(cx, |input, cx| input.set_value(kept, window, cx));
            }
        }
        let subs = vec![cx.subscribe_in(&name, window, move |this, input, event: &InputEvent, window, cx| match event {
            InputEvent::Change if is_new => {
                crate::drafts::keep(&name_draft(&this.job), input.read(cx).value().as_ref(), cx);
                cx.notify();
            }
            InputEvent::PressEnter { .. } => this.save_environment(id, window, cx),
            InputEvent::Focus => {
                this.env_active = Some(id);
                cx.notify();
            }
            _ => cx.notify(),
        })];
        EnvForm {
            id,
            read: read.cloned(),
            name,
            rows,
            refused: None,
            deleting: false,
            task: None,
            save_focus: cx.focus_handle().tab_stop(true),
            delete_focus: cx.focus_handle().tab_stop(true),
            confirm_focus: cx.focus_handle().tab_stop(true),
            cancel_focus: cx.focus_handle().tab_stop(true),
            _subs: subs,
        }
    }

    /// A read landed: a form whose environment Fleet reads as before stays with what was typed in it; one that changed, such as
    /// after its own save, is made again from the read, and one that is gone goes.
    pub(crate) fn sync_forms(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let editable: Vec<Placed<RequestEnvironmentRead>> = self.environments().iter().filter(|one| one.place != RequestPlace::Server).cloned().collect();
        let mut old = std::mem::take(&mut self.forms);
        let mut next = Vec::with_capacity(editable.len());
        for env in editable {
            match old.iter().position(|form| form.read.as_ref() == Some(&env)) {
                Some(at) => next.push(old.remove(at)),
                None => next.push(self.env_form(Some(&env), window, cx)),
            }
        }
        self.forms = next;
    }

    /// Something was typed in a form: it ends with a row to type into.
    fn env_edited(&mut self, form: u64, window: &mut Window, cx: &mut Context<Self>) {
        let needs = self.form_mut(form).map(|one| one.rows.last().map_or(true, |last| !row_blank(last, cx))).unwrap_or(false);
        if needs {
            let row = self.blank_row(form, window, cx);
            if let Some(one) = self.form_mut(form) {
                one.rows.push(row);
            }
        }
        cx.notify();
    }

    fn set_secret(&mut self, form: u64, at: usize, secret: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.form_mut(form).and_then(|one| one.rows.get_mut(at)) else { return };
        if row.set || row.secret == secret {
            return;
        }
        row.secret = secret;
        row.value.update(cx, |input, cx| input.set_masked(secret, window, cx));
        cx.notify();
    }

    fn replace_secret(&mut self, form: u64, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.form_mut(form).and_then(|one| one.rows.get_mut(at)) else { return };
        row.replacing = true;
        let value = row.value.clone();
        value.update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    fn remove_variable(&mut self, form: u64, at: usize, cx: &mut Context<Self>) {
        if let Some(one) = self.form_mut(form) {
            if at < one.rows.len() {
                one.rows.remove(at);
            }
        }
        cx.notify();
    }

    // ---- adding, saving, deleting ----------------------------------------------------------------

    fn yours_locked(&self) -> bool {
        model::yours_unread(self.requests().map(|read| read.as_ref())).is_some()
    }

    pub(crate) fn add_environment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.yours_locked() {
            return;
        }
        if self.adding.is_none() {
            let form = self.env_form(None, window, cx);
            self.adding = Some(form);
        }
        if let Some(name) = self.adding.as_ref().map(|form| form.name.clone()) {
            name.update(cx, |input, cx| input.focus(window, cx));
        }
        cx.notify();
    }

    /// Why a form's Save is off.
    fn env_off(&self, form: &EnvForm, cx: &gpui::App) -> Option<String> {
        if self.yours_locked() {
            return Some("Your own requests file would not read".to_string());
        }
        if form.task.is_some() {
            return Some("Saving".to_string());
        }
        if form.label(cx).is_empty() {
            return Some("Name the environment".to_string());
        }
        None
    }

    /// ⌘S in the Environment part: the form the keys were last in.
    pub(crate) fn save_active_environment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.env_active {
            self.save_environment(id, window, cx);
        }
    }

    /// The form whole, as `POST /requests/environments`. A secret's value goes in the body and out of the field at once.
    pub(crate) fn save_environment(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.forms.iter().chain(self.adding.as_ref()).find(|form| form.id == id) else { return };
        if self.env_off(form, cx).is_some() {
            return;
        }
        let name = form.label(cx);
        let variables: Vec<VariableWrite> = form
            .rows
            .iter()
            .filter(|row| !row.name.read(cx).value().trim().is_empty())
            .map(|row| {
                let name = row.name.read(cx).value().trim().to_string();
                let value = row.value.read(cx).value().to_string();
                if row.secret {
                    VariableWrite::Secret { name, value: Some(value).filter(|value| !value.is_empty()) }
                } else {
                    VariableWrite::Plain { name, value }
                }
            })
            .collect();
        let write = RequestEnvironmentWrite { name, variables };
        self.wipe_secrets(id, window, cx);
        if let Some(form) = self.form_mut(id) {
            form.refused = None;
        }
        let job = self.job.to_string();
        let task = self.ask(
            write,
            move |port, cx, write| wire::save_environment(port, &job, write, cx),
            |store, write| store.save_environment(write),
            move |this, answer, window, cx| {
                if let Some(form) = this.form_mut(id) {
                    form.task = None;
                    if let Err(said) = &answer {
                        form.refused = Some(said.clone());
                    }
                }
                if let Ok(read) = answer {
                    if this.adding.as_ref().is_some_and(|form| form.id == id) {
                        this.adding = None;
                        crate::drafts::keep(&name_draft(&this.job), "", cx);
                    }
                    this.landed(Ok(read), window, cx);
                }
                cx.notify();
            },
            window,
            cx,
        );
        if let Some(form) = self.form_mut(id) {
            if task.is_some() {
                form.task = task;
            }
        }
        cx.notify();
    }

    /// Each secret's field made again, empty: what was typed goes, and nothing of it stays in the field's history.
    fn wipe_secrets(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.forms.iter().chain(self.adding.as_ref()).find(|form| form.id == id) else { return };
        let fresh: Vec<(usize, VarRow)> = form
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.secret)
            .map(|(at, row)| {
                let name = row.name.read(cx).value().to_string();
                (at, self.var_row(id, &name, "", true, row.set, row.replacing, window, cx))
            })
            .collect();
        if let Some(form) = self.form_mut(id) {
            for (at, row) in fresh {
                if let Some(slot) = form.rows.get_mut(at) {
                    *slot = row;
                }
            }
        }
    }

    fn ask_delete(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.form_mut(id) else { return };
        form.deleting = true;
        let confirm = form.confirm_focus.clone();
        window.focus(&confirm, cx);
        cx.notify();
    }

    fn keep_environment(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.form_mut(id) else { return };
        form.deleting = false;
        let back = form.delete_focus.clone();
        window.focus(&back, cx);
        cx.notify();
    }

    fn delete_environment(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.forms.iter().find(|form| form.id == id) else { return };
        let Some(name) = form.read.as_ref().map(|read| read.item.name.clone()) else { return };
        if self.yours_locked() || form.task.is_some() {
            return;
        }
        let job = self.job.to_string();
        let task = self.ask(
            name,
            move |port, cx, name| wire::delete_environment(port, &job, name, cx),
            |store, name| store.delete_environment(&name),
            move |this, answer, window, cx| {
                if let Some(form) = this.form_mut(id) {
                    form.task = None;
                    form.deleting = false;
                    if let Err(said) = &answer {
                        form.refused = Some(said.clone());
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
        if let Some(form) = self.form_mut(id) {
            if task.is_some() {
                form.task = task;
            }
        }
        cx.notify();
    }

    /// Esc in the Environment part: a delete that was asked goes unasked, then a form being added goes.
    pub(crate) fn back_out_environment(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.part != Part::Environment {
            return false;
        }
        if let Some(id) = self.forms.iter().find(|form| form.deleting).map(|form| form.id) {
            self.keep_environment(id, window, cx);
            return true;
        }
        if self.adding.is_some() {
            self.adding = None;
            let focus = self.env_add_focus.clone();
            window.focus(&focus, cx);
            cx.notify();
            return true;
        }
        false
    }

    // ---- drawing ---------------------------------------------------------------------------------

    /// The part: each environment of the repository as a form, Add environment, then the Job's servers read only.
    pub(crate) fn environment_panel(&self, look: &Look, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if matches!(self.read, super::Load::Reading) {
            return self.environment_bones(look);
        }
        let locked = self.yours_locked();
        let mut groups: Vec<AnyElement> = Vec::new();
        for form in &self.forms {
            groups.push(self.env_form_view(look, form, locked, window, cx));
        }
        if let Some(form) = self.adding.as_ref() {
            groups.push(self.env_form_view(look, form, locked, window, cx));
        }
        let none = groups.is_empty() && !locked && self.requests().is_some_and(|read| read.not_read.is_empty()) && !self.environments().iter().any(|one| one.place == RequestPlace::Server);
        let add = (self.adding.is_none() && !locked).then(|| {
            labelled(
                ghost(look, "api-env-add", "Add environment", false)
                    .track_focus(&self.env_add_focus)
                    .tab_stop(true)
                    .focus_visible(|style| style.border_color(ring_colour(look)))
                    .on_click(cx.listener(|this, _, window, cx| this.add_environment(window, cx))),
                Role::Button,
                "Add environment",
            )
        });
        let servers: Vec<&Placed<RequestEnvironmentRead>> = self.environments().iter().filter(|one| one.place == RequestPlace::Server).collect();
        let servers = (!servers.is_empty()).then(|| {
            labelled(
                div()
                    .id("api-env-servers")
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_3))
                    .child(labelled(
                        div().id("api-env-servers-title").text_size(look.length(token::TEXT_2XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_SUBTLE)).child("THIS JOB'S SERVERS"),
                        Role::Heading,
                        "This Job's servers",
                    ))
                    .children(servers.into_iter().enumerate().map(|(at, one)| self.server_view(look, at, one))),
                Role::Group,
                "This Job's servers",
            )
        });
        labelled(
            div()
                .id("api-environments")
                .flex_none()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_4))
                .children(none.then(|| div().id("api-env-none").text_color(look.color(token::FG_SUBTLE)).child("No environments in this repository")))
                .children(groups)
                .children(add)
                .children(servers),
            Role::Group,
            "Environment",
        )
        .into_any_element()
    }

    /// Skeleton shapes where the environments will be, while Fleet has not answered.
    fn environment_bones(&self, look: &Look) -> AnyElement {
        use crate::settings::view::bone;
        labelled(
            div()
                .id("api-environments-loading")
                .flex_none()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .p(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .child(bone(look, look.length(token::SPACE_12) * 1.5, look.length(token::LEADING_SM)))
                .children((0..3).map(|at| {
                    div()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .child(bone(look, look.length(token::SPACE_12) * 3., look.length(token::H_CONTROL)))
                        .child(bone(look, look.length(token::SPACE_12) * (3. + at as f32), look.length(token::H_CONTROL)))
                })),
            Role::Image,
            "Loading",
        )
        .into_any_element()
    }

    /// An environment of a server the Job runs: its variables as they read, none of them editable.
    fn server_view(&self, look: &Look, at: usize, one: &Placed<RequestEnvironmentRead>) -> AnyElement {
        let name_column = look.length(token::SPACE_12) * 3.;
        let rows = one.item.variables.iter().enumerate().map(|(row, variable)| {
            let (name, value) = match variable {
                VariableRead::Plain { name, value } => (name.clone(), value.clone()),
                VariableRead::Secret { name, .. } => (name.clone(), "Set".to_string()),
            };
            labelled(
                div()
                    .id(SharedString::from(format!("api-server-{at}-{row}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .h(look.length(token::H_ROW))
                    .font_family(mono(look))
                    .child(div().flex_none().w(name_column).truncate().text_color(look.color(token::FG_MUTED)).child(SharedString::from(name.clone())))
                    .child(div().flex_1().min_w_0().truncate().child(SharedString::from(value.clone()))),
                Role::Row,
                format!("{name}: {value}"),
            )
        });
        labelled(
            div()
                .id(SharedString::from(format!("api-server-{at}")))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .p(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_3))
                        .child(div().font_family(mono(look)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(one.item.name.clone())))
                        .child(div().text_color(look.color(token::FG_SUBTLE)).child("Read only")),
                )
                .children(rows),
            Role::Group,
            one.item.name.clone(),
        )
        .into_any_element()
    }

    /// One environment's form.
    fn env_form_view(&self, look: &Look, form: &EnvForm, locked: bool, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let id = form.id;
        let label = form.read.as_ref().map(|read| read.item.name.clone());
        let title = label.clone().unwrap_or_else(|| "New environment".to_string());
        let name_column = look.length(token::SPACE_12) * 3.;
        let switch_column = look.length(token::SPACE_12) + look.length(token::SPACE_4);
        let last = form.rows.len().saturating_sub(1);
        let mark = form.read.as_ref().map(|read| {
            if read.hides {
                div().id(SharedString::from(format!("api-env-{id}-hides"))).flex_none().text_color(look.color(token::FG_SUBTLE)).child("Hides the repository's").into_any_element()
            } else if read.place == RequestPlace::Repository {
                div()
                    .id(SharedString::from(format!("api-env-{id}-repository")))
                    .flex_none()
                    .child(svg().flex_none().path(icon_path("folder-git-2")).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_MUTED)))
                    .tooltip(tip("The repository's"))
                    .into_any_element()
            } else {
                div().into_any_element()
            }
        });
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(div().font_family(mono(look)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(title.clone())))
            .children(mark);
        let name_field = form.read.is_none().then(|| {
            let focused = form.name.read(cx).focus_handle(cx).is_focused(window);
            labelled(field(look, SharedString::from(format!("api-env-{id}-name")), focused).child(Input::new(&form.name).appearance(false)), Role::TextInput, "Environment name")
        });
        let column = |word: &'static str, wide: Option<gpui::Pixels>| {
            div().flex_none().when_some(wide, |cell, wide| cell.w(wide)).when(wide.is_none(), |cell| cell.flex_1()).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(word)
        };
        let heads = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(column("Name", Some(name_column)))
            .child(column("Value", None))
            .child(column("Secret", Some(switch_column)))
            .child(div().flex_none().w(look.length(token::H_CONTROL_SM)));
        let rows = form.rows.iter().enumerate().map(|(at, row)| {
            let n = at + 1;
            let name_focused = row.name.read(cx).focus_handle(cx).is_focused(window);
            let value_focused = row.value.read(cx).focus_handle(cx).is_focused(window);
            let typed = row.name.read(cx).value().to_string();
            let named = if typed.is_empty() { format!("Variable {n}") } else { typed };
            let showing_set = row.secret && row.set && !row.replacing;
            let value = if showing_set {
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .h(look.length(token::H_CONTROL))
                    .child(
                        div()
                            .id(SharedString::from(format!("api-env-{id}-{at}-set")))
                            .flex_none()
                            .child(svg().flex_none().path(icon_path("square-asterisk")).size(look.length(token::SPACE_3)).text_color(look.color(token::FG_MUTED)))
                            .tooltip(tip("Set, in the Keychain")),
                    )
                    .child(div().flex_none().child("Set"))
                    .child(labelled(
                        ghost(look, SharedString::from(format!("api-env-{id}-{at}-replace")), "Replace", false)
                            .track_focus(&row.replace_focus)
                            .tab_stop(true)
                            .focus_visible(|style| style.border_color(ring_colour(look)))
                            .on_click(cx.listener(move |this, _, window, cx| this.replace_secret(id, at, window, cx))),
                        Role::Button,
                        format!("Replace {named}"),
                    ))
                    .into_any_element()
            } else {
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .child(labelled(
                        field(look, SharedString::from(format!("api-env-{id}-{at}-value")), value_focused).child(Input::new(&row.value).appearance(false)),
                        Role::TextInput,
                        if row.secret { format!("Secret value of {named}") } else { format!("Value of {named}") },
                    ))
                    .into_any_element()
            };
            let on = row.secret;
            let off = row.set;
            let toggle = labelled(
                div()
                    .id(SharedString::from(format!("api-env-{id}-{at}-secret")))
                    .track_focus(&row.switch_focus)
                    .tab_stop(!off)
                    .flex_none()
                    .w(switch_column)
                    .flex()
                    .items_center()
                    .when(!off, |cell| cell.cursor_pointer().on_click(cx.listener(move |this, _, window, cx| this.set_secret(id, at, !on, window, cx))))
                    .aria_toggled(if on { Toggled::True } else { Toggled::False })
                    .child(switch_track(look, on, off)),
                Role::Switch,
                format!("Secret {named}"),
            );
            let remove = (at != last).then(|| {
                labelled(
                    div()
                        .id(SharedString::from(format!("api-env-{id}-{at}-remove")))
                        .track_focus(&row.remove_focus)
                        .tab_stop(true)
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(look.length(token::H_CONTROL_SM))
                        .rounded(look.length(token::RADIUS_MD))
                        .cursor_pointer()
                        .hover(|style| style.bg(look.color(token::BG_HOVER)))
                        .focus_visible(|style| style.bg(look.color(token::BG_HOVER)))
                        .tooltip(tip("Remove"))
                        .on_click(cx.listener(move |this, _, _, cx| this.remove_variable(id, at, cx)))
                        .child(svg().flex_none().path(icon_path("x")).size(look.length(token::SPACE_4)).text_color(look.color(token::FG_SUBTLE))),
                    Role::Button,
                    format!("Remove variable {n}"),
                )
            });
            div()
                .id(SharedString::from(format!("api-env-{id}-row-{at}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(div().flex_none().w(name_column).flex().child(labelled(
                    field(look, SharedString::from(format!("api-env-{id}-{at}-name")), name_focused).child(Input::new(&row.name).appearance(false)),
                    Role::TextInput,
                    format!("Variable {n} name"),
                )))
                .child(value)
                .child(toggle)
                .child(div().flex_none().w(look.length(token::H_CONTROL_SM)).children(remove))
        });
        let why = self.env_off(form, cx);
        let off = why.is_some();
        let save = labelled(
            div()
                .id(SharedString::from(format!("api-env-{id}-save")))
                .track_focus(&form.save_focus)
                .tab_stop(!off)
                .flex_none()
                .flex()
                .items_center()
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_4))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_RAISED))
                .text_size(look.length(token::TEXT_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .map(|button| {
                    if off {
                        button.opacity(0.5).text_color(look.color(token::FG_MUTED))
                    } else {
                        button
                            .text_color(look.color(token::FG_DEFAULT))
                            .cursor_pointer()
                            .hover(|style| style.bg(look.color(token::BG_HOVER)))
                            .focus_visible(|style| style.border_color(ring_colour(look)))
                            .on_click(cx.listener(move |this, _, window, cx| this.save_environment(id, window, cx)))
                    }
                })
                .when_some(why.filter(|why| why != "Saving"), |button, why| button.tooltip(tip(why)))
                .child("Save"),
            Role::Button,
            format!("Save {title}"),
        );
        // Only the person's own can be taken out; the repository's is theirs to hide by saving a copy.
        let deletable = form.read.as_ref().is_some_and(|read| read.place == RequestPlace::Yours);
        let acts = if form.deleting {
            let confirm = labelled(
                ghost(look, SharedString::from(format!("api-env-{id}-confirm")), "Delete", false)
                    .track_focus(&form.confirm_focus)
                    .tab_stop(true)
                    .text_color(look.color(token::STATUS_COMPLETED_FAILED))
                    .border_color(look.color(token::STATUS_COMPLETED_FAILED))
                    .bg(look.color(token::STATUS_COMPLETED_FAILED_BG))
                    .focus_visible(|style| style.border_color(ring_colour(look)))
                    .on_click(cx.listener(move |this, _, window, cx| this.delete_environment(id, window, cx))),
                Role::Button,
                format!("Delete {title}"),
            );
            let cancel = labelled(
                ghost(look, SharedString::from(format!("api-env-{id}-cancel")), "Cancel", false)
                    .track_focus(&form.cancel_focus)
                    .tab_stop(true)
                    .focus_visible(|style| style.border_color(ring_colour(look)))
                    .on_click(cx.listener(move |this, _, window, cx| this.keep_environment(id, window, cx))),
                Role::Button,
                "Cancel delete",
            );
            labelled(
                div().id(SharedString::from(format!("api-env-{id}-confirm-row"))).flex().items_center().gap(look.length(token::SPACE_2)).child(div().text_color(look.color(token::FG_MUTED)).child(SharedString::from(format!("Delete {title}")))).child(confirm).child(cancel),
                Role::Group,
                format!("Delete {title}"),
            )
            .into_any_element()
        } else {
            let delete = deletable.then(|| {
                labelled(
                    ghost(look, SharedString::from(format!("api-env-{id}-delete")), "Delete", locked)
                        .track_focus(&form.delete_focus)
                        .tab_stop(!locked)
                        .focus_visible(|style| style.border_color(ring_colour(look)))
                        .when(!locked, |button| button.on_click(cx.listener(move |this, _, window, cx| this.ask_delete(id, window, cx))))
                        .when(locked, |button| button.tooltip(tip("Your own requests file would not read"))),
                    Role::Button,
                    format!("Delete {title}"),
                )
            });
            div().id(SharedString::from(format!("api-env-{id}-acts"))).flex().items_center().gap(look.length(token::SPACE_2)).child(save).children(delete).into_any_element()
        };
        labelled(
            div()
                .id(SharedString::from(format!("api-env-{id}")))
                .flex_none()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .p(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .child(head)
                .children(name_field)
                .child(heads)
                .children(rows)
                .child(acts)
                .children(form.refused.clone().map(|said| div().id(SharedString::from(format!("api-env-{id}-refused"))).child(refusal(look, said)))),
            Role::Group,
            title,
        )
        .into_any_element()
    }
}
