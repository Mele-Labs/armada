//! Add: paste a link to a plugin or a skill, Fleet looks and says what installing it runs, and then it installs.
//! Fleet runs nothing at the look, and the install looks again rather than taking commands from here.

use gpui::{div, prelude::*, AnyElement, Context, Entity, Focusable as _, FontWeight, Role, SharedString, Subscription, Window};
use gpui_component::input::{Input, InputEvent, InputState};
use ipc::{InstallKitPiece, KitPieceInstalled, KitPieceKind, KitSourceFound};

use super::{model, KitScreen, Load, Source};
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::titlebar::parts::{mono, Btn, Weight};

/// Where an Add stands.
pub(super) enum Step {
    /// Waiting for a link.
    Ask,
    Looking,
    Found { found: KitSourceFound, choice: Option<String> },
    Installing,
    /// Fleet ran a command and it failed, or refused: what it said.
    Said(Vec<String>),
}

pub(super) struct Add {
    pub field: Entity<InputState>,
    pub step: Step,
    _typed: Subscription,
}

/// What the link field keeps, so a pasted link survives a restart.
const KEEP_ADD: &str = "kit:add";

impl KitScreen {
    /// The Add button, or A: the sheet opens with the keys in its field.
    pub(super) fn start_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.adding.is_some() {
            return;
        }
        let field = cx.new(|cx| InputState::new(window, cx).placeholder("Link to a plugin or a skill"));
        if let Some(words) = crate::drafts::kept(KEEP_ADD, cx) {
            field.update(cx, |state, cx| state.set_value(words, window, cx));
        }
        let typed = cx.subscribe_in(&field, window, |this, state, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                let text = state.read(cx).value().to_string();
                if text.is_empty() {
                    crate::drafts::clear(KEEP_ADD, cx);
                } else {
                    crate::drafts::keep(KEEP_ADD, &text, cx);
                }
                // A new link is a new look.
                if let Some(add) = this.adding.as_mut() {
                    if !matches!(add.step, Step::Looking | Step::Installing) {
                        add.step = Step::Ask;
                    }
                }
                cx.notify();
            }
        });
        let handle = field.read(cx).focus_handle(cx);
        window.focus(&handle, cx);
        self.adding = Some(Add { field, step: Step::Ask, _typed: typed });
        cx.notify();
    }

    pub(super) fn close_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.adding = None;
        self.focus(super::Pane::List, window, cx);
    }

    /// Enter in the sheet: look at the link, or install what the look found.
    pub(super) fn add_enter(&mut self, cx: &mut Context<Self>) {
        let Some(add) = self.adding.as_ref() else { return };
        match &add.step {
            Step::Ask | Step::Said(_) => self.look(cx),
            Step::Found { .. } => self.install(cx),
            Step::Looking | Step::Installing => {}
        }
    }

    fn look(&mut self, cx: &mut Context<Self>) {
        let mock = self.is_mock();
        let Some(add) = self.adding.as_mut() else { return };
        let source = add.field.read(cx).value().trim().to_string();
        if source.is_empty() {
            return;
        }
        if mock {
            add.step = Step::Found {
                found: KitSourceFound::Plugin {
                    name: "design-kit".into(),
                    marketplace: "design-kit".into(),
                    repository: "example/design-kit".into(),
                    skills: vec!["frontend-design".into()],
                    commands: vec!["claude plugin marketplace add example/design-kit".into(), "claude plugin install design-kit@design-kit".into()],
                },
                choice: None,
            };
            cx.notify();
            return;
        }
        let Some(port) = self.port(cx) else {
            if let Some(add) = self.adding.as_mut() {
                add.step = Step::Said(vec!["Fleet is not connected, so nothing can be installed.".into()]);
            }
            cx.notify();
            return;
        };
        if let Some(add) = self.adding.as_mut() {
            add.step = Step::Looking;
        }
        let ask = super::wire::look(port, &source, cx);
        self.work.insert(
            "add",
            cx.spawn(async move |this, cx| {
                let answer = ask.await;
                let _ = this.update(cx, |this, cx| {
                    let Some(add) = this.adding.as_mut() else { return };
                    add.step = match answer {
                        Ok(found) => Step::Found { found, choice: None },
                        Err(why) => Step::Said(vec![why.said]),
                    };
                    cx.notify();
                });
            }),
        );
        cx.notify();
    }

    /// What Install is asked for: the look's repository and kind, and the one picked where it found several.
    fn installing(found: &KitSourceFound, choice: &Option<String>) -> Option<InstallKitPiece> {
        match found {
            KitSourceFound::Plugin { repository, .. } => Some(InstallKitPiece { source: repository.clone(), kind: KitPieceKind::Plugin, choice: None }),
            KitSourceFound::Skill { repository, .. } => Some(InstallKitPiece { source: repository.clone(), kind: KitPieceKind::Skill, choice: None }),
            KitSourceFound::Several { repository, kind, .. } => choice.clone().map(|choice| InstallKitPiece { source: repository.clone(), kind: *kind, choice: Some(choice) }),
            KitSourceFound::Unknown { .. } => None,
        }
    }

    fn install(&mut self, cx: &mut Context<Self>) {
        let Some(add) = self.adding.as_mut() else { return };
        let Step::Found { found, choice } = &add.step else { return };
        let Some(ask) = Self::installing(found, choice) else { return };
        if matches!(self.source, Source::Mock(_)) {
            self.adding = None;
            cx.notify();
            return;
        }
        let Some(port) = self.port(cx) else { return };
        if let Some(add) = self.adding.as_mut() {
            add.step = Step::Installing;
        }
        let ask = super::wire::install(port, ask, cx);
        self.work.insert(
            "add",
            cx.spawn(async move |this, cx| {
                let answer = ask.await;
                let _ = this.update(cx, |this, cx| match answer {
                    Ok(KitPieceInstalled::Installed { setup }) => {
                        this.inventory = Load::Read(setup);
                        this.adding = None;
                        crate::drafts::clear(KEEP_ADD, cx);
                        this.relist(cx);
                    }
                    Ok(KitPieceInstalled::Failed { command, error, taken_back, .. }) => {
                        if let Some(add) = this.adding.as_mut() {
                            let mut said = vec![format!("{command} did not finish."), error];
                            said.extend(taken_back.into_iter().map(|one| format!("Taken back: {one}.")));
                            add.step = Step::Said(said);
                        }
                        cx.notify();
                    }
                    Err(why) => {
                        if let Some(add) = this.adding.as_mut() {
                            add.step = Step::Said(vec![why.said]);
                        }
                        cx.notify();
                    }
                });
            }),
        );
        cx.notify();
    }

    fn choose(&mut self, name: String, cx: &mut Context<Self>) {
        if let Some(Step::Found { choice, .. }) = self.adding.as_mut().map(|add| &mut add.step) {
            *choice = Some(name);
            cx.notify();
        }
    }

    /// The sheet over Kit while a link is being added.
    pub(super) fn render_add(&self, look: &Look, cx: &mut Context<Self>) -> Option<AnyElement> {
        let add = self.adding.as_ref()?;
        let line = |text: String| div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(text));
        let commands = |commands: &[String]| {
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .p(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(token::BG_SUNKEN))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .font_family(mono(look))
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_DEFAULT))
                .children(commands.iter().map(|one| div().child(SharedString::from(one.clone()))))
        };
        let mut body = div().flex().flex_col().gap(look.length(token::SPACE_3)).child(labelled(
            div().id("kit-add-link").key_context(crate::keymap::TEXT_BOX).text_size(look.length(token::TEXT_SM)).child(Input::new(&add.field)),
            Role::TextInput,
            "Link to a plugin or a skill",
        ));
        let mut ready = false;
        match &add.step {
            Step::Ask => {}
            Step::Looking => body = body.child(line("Reading it. Nothing runs yet.".into())),
            Step::Installing => body = body.child(line("Installing.".into())),
            Step::Said(said) => {
                for one in said {
                    body = body.child(labelled(crate::settings::view::refusal(look, one.clone()).id(SharedString::from(format!("kit-add-said-{one}"))), Role::Alert, one.clone()));
                }
            }
            Step::Found { found, choice } => match found {
                KitSourceFound::Plugin { name, marketplace, skills, commands: runs, .. } => {
                    ready = true;
                    body = body.child(line(format!("Plugin {name} from {marketplace}.")));
                    if !skills.is_empty() {
                        body = body.child(line(format!("It brings {}.", skills.join(", "))));
                    }
                    body = body.child(line("Installing runs".into())).child(commands(runs));
                }
                KitSourceFound::Skill { name, commands: runs, .. } => {
                    ready = true;
                    body = body.child(line(format!("Skill {name}."))).child(line("Installing runs".into())).child(commands(runs));
                }
                KitSourceFound::Several { choices, .. } => {
                    ready = choice.is_some();
                    for (n, one) in choices.iter().enumerate() {
                        let picked = choice.as_deref() == Some(one.name.as_str());
                        let name = one.name.clone();
                        let row = div()
                            .id(SharedString::from(format!("kit-add-choice-{n}")))
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .min_h(look.length(token::H_CONTROL_SM))
                            .px(look.length(token::SPACE_2))
                            .rounded(look.length(token::RADIUS_MD))
                            .border(look.length(token::BORDER_WIDTH))
                            .border_color(if picked { look.color(token::ACCENT) } else { gpui::transparent_black() })
                            .when(picked, |row| row.bg(look.color(token::SURFACE_ROW_SELECTED)))
                            .cursor_pointer()
                            .hover(|style| style.bg(look.color(token::BG_HOVER)))
                            .text_size(look.length(token::TEXT_SM))
                            .text_color(look.color(token::FG_DEFAULT))
                            .on_click(cx.listener(move |this, _, _, cx| this.choose(name.clone(), cx)))
                            .child(SharedString::from(one.name.clone()))
                            .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(one.says.clone().unwrap_or_default())));
                        body = body.child(labelled(row, Role::RadioButton, one.name.clone()));
                    }
                    if let Some(chosen) = choice.as_ref().and_then(|name| choices.iter().find(|one| &one.name == name)) {
                        body = body.child(line("Installing runs".into())).child(commands(&chosen.commands));
                    }
                }
                KitSourceFound::Unknown { why } => body = body.child(labelled(crate::settings::view::refusal(look, why.clone()).id("kit-add-unknown"), Role::Alert, why.clone())),
            },
        }
        let working = matches!(add.step, Step::Looking | Step::Installing);
        let action = if matches!(add.step, Step::Found { .. }) {
            let button = Btn::new("kit-add-install", "Install", Weight::Primary).disabled(!ready).draw(look, cx);
            labelled(button.when(ready, |button| button.on_click(cx.listener(|this, _, _, cx| this.install(cx)))), Role::Button, "Install")
        } else {
            let button = Btn::new("kit-add-look", "Look", Weight::Primary).disabled(working).pending(working).draw(look, cx);
            labelled(button.when(!working, |button| button.on_click(cx.listener(|this, _, _, cx| this.look(cx)))), Role::Button, "Look")
        };
        let cancel = labelled(Btn::new("kit-add-cancel", "Cancel", Weight::Secondary).draw(look, cx).on_click(cx.listener(|this, _, window, cx| this.close_add(window, cx))), Role::Button, "Cancel");
        let dialog = labelled(
            crate::titlebar::parts::card(look)
                .id("kit-add")
                .w(look.length(token::SPACE_12) * 9.)
                .max_h_full()
                .p(look.length(token::SPACE_4))
                .gap(look.length(token::SPACE_3))
                .child(div().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child("Add to Kit"))
                .child(body)
                .child(div().flex().justify_end().gap(look.length(token::SPACE_2)).child(cancel).child(action)),
            Role::Dialog,
            "Add to Kit",
        );
        Some(sheet(look, dialog))
    }

    /// The question before a plugin goes: what runs, what goes with it, and that there is no way back.
    pub(super) fn render_confirm(&self, look: &Look, cx: &mut Context<Self>) -> Option<AnyElement> {
        let confirm = self.confirming.as_ref()?;
        let title = format!("Remove {}", confirm.row.name);
        let mut body = div().flex().flex_col().gap(look.length(token::SPACE_3));
        if let model::Removal::Piece(piece) = confirm.row.removal() {
            if let Load::Read(inventory) = &self.inventory {
                if let Some(run) = uninstall_of(inventory, &piece.name) {
                    body = body.child(
                        div()
                            .p(look.length(token::SPACE_3))
                            .rounded(look.length(token::RADIUS_MD))
                            .bg(look.color(token::BG_SUNKEN))
                            .border(look.length(token::BORDER_WIDTH))
                            .border_color(look.color(token::BORDER_SUBTLE))
                            .font_family(mono(look))
                            .text_size(look.length(token::TEXT_2XS))
                            .text_color(look.color(token::FG_DEFAULT))
                            .child(SharedString::from(run)),
                    );
                }
            }
        }
        let say = |text: String| div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(text));
        if !confirm.brings.is_empty() {
            body = body.child(say(format!("Removes what it brings: {}.", confirm.brings.join(", "))));
        }
        body = body.child(say("There is no Undo.".into()));
        let remove = labelled(Btn::new("kit-remove-confirm", "Remove", Weight::Primary).draw(look, cx).on_click(cx.listener(|this, _, _, cx| this.confirm_remove(cx))), Role::Button, "Remove");
        let cancel = labelled(
            Btn::new("kit-remove-cancel", "Cancel", Weight::Secondary).draw(look, cx).on_click(cx.listener(|this, _, _, cx| {
                this.confirming = None;
                cx.notify();
            })),
            Role::Button,
            "Cancel",
        );
        let dialog = labelled(
            crate::titlebar::parts::card(look)
                .id("kit-confirm")
                .w(look.length(token::SPACE_12) * 8.)
                .p(look.length(token::SPACE_4))
                .gap(look.length(token::SPACE_3))
                .child(div().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(title.clone())))
                .child(body)
                .child(div().flex().justify_end().gap(look.length(token::SPACE_2)).child(cancel).child(remove)),
            Role::Dialog,
            title,
        );
        Some(sheet(look, dialog))
    }
}

/// The command an installed plugin's Remove runs, as its inventory row spells it.
fn uninstall_of(inventory: &ipc::KitInventory, name: &str) -> Option<String> {
    let kind = inventory.kinds.iter().find(|row| row.kind == "plugins")?;
    match &kind.read {
        ipc::WhatWasRead::Read { items, .. } => items.iter().find(|item| item.name == name).and_then(|item| item.uninstall.clone()),
        ipc::WhatWasRead::NotRead { .. } => None,
    }
}

/// A dialog over a scrim, filling Kit.
fn sheet(look: &Look, dialog: impl IntoElement) -> AnyElement {
    div().absolute().top_0().left_0().size_full().flex().items_center().justify_center().bg(look.color(token::SCRIM)).occlude().child(dialog).into_any_element()
}
