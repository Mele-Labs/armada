//! The Fleet build switcher in the Fleet popover, the Keep and Revert bar above the screen, and the
//! dialog that confirms a revert (`FleetBuild.tsx`, `trying-bar.tsx`).

use gpui::{
    anchored, canvas, deferred, div, prelude::*, px, Anchor, AnyElement, Context, FontWeight, Pixels, Role, SharedString,
};
use ipc::{BuildSource, SnapshotState};

use super::face::{ahead_said, behind_said, built_said, kept_said, stage_word, Build, Position};
use super::parts::{card, glyph, mono, sans, spinner, tip, Btn, Weight};
use super::{Dialog, MenuKind, TitleBar};
use crate::dropdown::Entry;
use crate::labels::labelled;
use crate::look::{token, Look};

/// What a restart costs a Drone that is working, on hover.

impl TitleBar {
    /// The switcher, stacked, as the first block of the Fleet popover.
    pub(super) fn build_section(&self, held: &Build, look: &Look, cx: &Context<Self>) -> AnyElement {
        let restart = held.restart();
        let report = &held.report;
        let working = restart.pending;
        let spin = look.color(token::FG_DEFAULT);
        let select_at = std::rc::Rc::clone(&self.at.select);
        let on_name = match report.on {
            BuildSource::Main => "Main",
            BuildSource::Preview => "Preview",
        };
        let select = labelled(
            div()
                .id("build-select")
                .relative()
                .w_full()
                .h(look.length(token::H_CONTROL))
                .flex()
                .items_center()
                .justify_between()
                .px(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .bg(look.color(token::BG_SUNKEN))
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .when(!working, |select| {
                    let ring = Self::focus_ring(look);
                    select
                        .cursor_pointer()
                        .focusable()
                        .tab_stop(true)
                        .focus_visible(move |style| style.border_color(ring))
                        .hover(|style| style.bg(look.color(token::BG_HOVER)))
                        .on_click(cx.listener(|this, _, window, cx| this.build_select_pressed(window, cx)))
                })
                .when(working, |select| select.opacity(0.5))
                .child(canvas(move |bounds, _, _| select_at.set(Some(bounds)), |_, _, _, _| {}).absolute().size_full())
                .child(on_name)
                .child(glyph("chevron-down", look.length(token::SPACE_4), look.color(token::FG_MUTED)))
                .tooltip(tip("Build Fleet runs on")),
            Role::Button,
            "Build",
        );
        let restart_button = {
            let button = Btn::new("restart-build", restart.label, Weight::Secondary)
                .icon("rotate-cw")
                .pending(restart.pending)
                .disabled(restart.disabled)
                .full()
                .draw(look, cx)
                .tooltip(tip(restart.tip));
            let button = if restart.disabled { button } else { button.on_click(cx.listener(|this, _, window, cx| this.restart_pressed(window, cx))) };
            labelled(button, Role::Button, restart.label)
        };
        let revert_button = held.revert_tip().map(|said| {
            let label = if report.reverting { "Reverting" } else { "Revert" };
            let off = working || report.reverting || report.swapping;
            let button = Btn::new("revert-build", label, Weight::Secondary).icon("undo-2").pending(report.reverting).disabled(off).full().draw(look, cx).tooltip(tip(said));
            let button = if off { button } else { button.on_click(cx.listener(|this, _, window, cx| this.revert_pressed(window, cx))) };
            labelled(button, Role::Button, label)
        });
        let picked = (!report.picked.is_empty()).then(|| {
            let chips: Vec<AnyElement> = report
                .picked
                .iter()
                .enumerate()
                .map(|(at, one)| {
                    let name: SharedString = one.title.clone().unwrap_or_else(|| one.branch.clone()).into();
                    let branch = one.branch.clone();
                    let leave: SharedString = format!("Leave {name} out of the build").into();
                    div()
                        .id(("picked", at))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(look.length(token::SPACE_2))
                        .pl(look.length(token::SPACE_3))
                        .pr(look.length(token::SPACE_1))
                        .h(look.length(token::H_CONTROL))
                        .rounded(look.length(token::RADIUS_LG))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_SUBTLE))
                        .bg(look.color(token::BG_SUNKEN))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .child(div().min_w_0().truncate().child(name))
                        .child(labelled(
                            super::parts::icon_button(("unpick", at), "x", look.color(token::FG_MUTED), look)
                                .size(look.length(token::H_CONTROL_SM))
                                .tooltip(tip("Leave out of the build"))
                                .on_click(cx.listener(move |this, _, _, cx| this.unpick_pressed(branch.clone(), cx))),
                            Role::Button,
                            leave,
                        ))
                        .into_any_element()
                })
                .collect();
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .child(div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::TEXT_LABEL)).child("Next build holds"))
                .children(chips)
        });
        let snapshot = report.snapshot.as_ref().map(|snapshot| match snapshot.state {
            SnapshotState::Building => labelled(
                div()
                    .id("snapshot")
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(spinner("snapshot-spin", px(12.), spin, cx))
                    .child("Building")
                    .children(snapshot.stage.map(|stage| div().child(stage_word(stage)))),
                Role::Status,
                "Building",
            )
            .into_any_element(),
            SnapshotState::Failed => {
                let said: SharedString = snapshot.failed.clone().unwrap_or_default().into();
                self.failed_line(look, "snapshot-failed", said)
            }
            SnapshotState::Ready => {
                let off = report.swapping || report.reverting || working;
                let label = if report.swapping { "Swapping" } else { "Swap" };
                let button = Btn::new("swap-build", label, Weight::Secondary).pending(report.swapping).disabled(off).draw(look, cx);
                let button = if off { button } else { button.on_click(cx.listener(|this, _, _, cx| this.swap_pressed(cx))) };
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .child(
                        self.mark(look, "snapshot-ready", "check", token::STATUS_COMPLETED_SUCCESS, "Ready").text_color(look.color(token::STATUS_COMPLETED_SUCCESS)),
                    )
                    .children((!snapshot.jobs.is_empty()).then(|| {
                        labelled(
                            div()
                                .id("held-by-snapshot")
                                .flex()
                                .flex_col()
                                .gap(look.length(token::SPACE_1))
                                .text_size(look.length(token::TEXT_XS))
                                .line_height(look.length(token::LEADING_XS))
                                .children(snapshot.jobs.iter().map(|one| div().truncate().child(one.clone()))),
                            Role::List,
                            "Held by the snapshot",
                        )
                    }))
                    .children(snapshot.bridge_reopens.then(|| {
                        div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child("Bridge reopens")
                    }))
                    .child(labelled(button, Role::Button, label))
                    .into_any_element()
            }
        });
        let failed = (!working).then(|| report.failed.clone()).flatten().map(|said| self.failed_line(look, "build-failed", said.into()));
        let left_out = (report.on == BuildSource::Preview && !working && !report.left_out.is_empty()).then(|| {
            labelled(
                div().id("left-out").flex().flex_col().gap(look.length(token::SPACE_3)).children(report.left_out.iter().enumerate().map(|(at, one)| {
                    div()
                        .id(("left-out-row", at))
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_1))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(look.length(token::SPACE_2))
                                .child(labelled(
                                    div()
                                        .id(("left-out-mark", at))
                                        .flex_none()
                                        .tooltip(tip("Left out of the preview"))
                                        .child(glyph("circle-slash", px(12.), look.color(token::STATUS_AWAITING_REVIEW))),
                                    Role::Image,
                                    "Left out of the preview",
                                ))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .font_family(mono(look))
                                        .text_size(look.length(token::TEXT_XS))
                                        .line_height(look.length(token::LEADING_XS))
                                        .child(one.branch.clone()),
                                )
                                // The moved column holds its width on every row, so the rows line up.
                                .child(div().flex_none().w(px(12.)).children(one.moved.then(|| {
                                    labelled(
                                        div()
                                            .id(("left-out-moved", at))
                                            .tooltip(tip("Moved since it was left out"))
                                            .child(glyph("git-commit-horizontal", px(12.), look.color(token::FG_MUTED))),
                                        Role::Image,
                                        "Moved since it was left out",
                                    )
                                }))),
                        )
                        .child(
                            div()
                                .id(("left-out-why", at))
                                .pl(px(12.) + look.length(token::SPACE_2))
                                .truncate()
                                .text_size(look.length(token::TEXT_XS))
                                .line_height(look.length(token::LEADING_XS))
                                .text_color(look.color(token::FG_MUTED))
                                .tooltip(tip(one.why.clone()))
                                .child(one.why.clone()),
                        )
                })),
                Role::List,
                "Left out of the preview",
            )
        });
        // The Refresh row has a Refresh preview of its own, so this one gives way to it.
        let offers = self.offers_section(held, look, cx);
        let restart_stays = super::offers::restart_stays(held, offers.is_some());
        let position = div()
            .flex()
            .items_center()
            .justify_between()
            .gap(look.length(token::SPACE_2))
            .children((!working).then(|| div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::TEXT_LABEL)).child("main")))
            .child(self.position_marks(held, look, cx));
        div()
            .id("fleet-build")
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .pb(look.length(token::SPACE_3))
            .mb(look.length(token::SPACE_1))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(select)
            .children(restart_stays.then_some(restart_button))
            .children(offers)
            .children(revert_button)
            .children(picked)
            .children(snapshot)
            .children(failed)
            .children(left_out)
            .children(self.origin_section(held, look, cx))
            // A restart with no stage yet has only its button's spinner to say; the row comes back with the stage's words.
            .children((!matches!(held.position(), Position::Restarting { stage: None, .. })).then_some(position))
            .into_any_element()
    }

    /// A mark: a 12px glyph with words, in mono, in the colour of its tone.
    fn mark(&self, look: &Look, id: &'static str, name: &'static str, tone: crate::look::Token, said: &'static str) -> gpui::Stateful<gpui::Div> {
        div()
            .id(id)
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .child(glyph(name, px(12.), look.color(tone)))
            .child(said)
    }

    fn failed_line(&self, look: &Look, id: &'static str, said: SharedString) -> AnyElement {
        labelled(
            div()
                .id(id)
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::STATUS_ESCALATED))
                .child(said.clone()),
            Role::Alert,
            said,
        )
        .into_any_element()
    }

    /// Where the build stands against main, or the stage a restart has got to.
    fn position_marks(&self, held: &Build, look: &Look, _cx: &Context<Self>) -> AnyElement {
        let small_mark = |id: &'static str, name: &'static str, tone: crate::look::Token, count: Option<u32>, said: String| -> AnyElement {
            let said: SharedString = said.into();
            labelled(
                div()
                    .id(id)
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(tone))
                    .tooltip(tip(said.clone()))
                    .child(glyph(name, px(12.), look.color(tone)))
                    .children(count.map(|count| count.to_string())),
                Role::Image,
                said,
            )
            .into_any_element()
        };
        match held.position() {
            Position::Unknown => div().into_any_element(),
            Position::Aligned => small_mark("aligned", "check", token::STATUS_COMPLETED_SUCCESS, None, "Aligned with main".into()),
            Position::Marks { ahead, behind } => div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .children((ahead > 0).then(|| small_mark("ahead", "arrow-up", token::FG_MUTED, Some(ahead), ahead_said(ahead))))
                .children((behind > 0).then(|| small_mark("behind", "arrow-down", token::STATUS_AWAITING_REVIEW, Some(behind), behind_said(behind))))
                .into_any_element(),
            Position::Restarting { target, stage } => {
                let on = if target == BuildSource::Preview { "the preview" } else { "main" };
                let tip_said = format!("Restarting Fleet on {on}");
                let (role, name): (Role, SharedString) = match stage {
                    Some(stage) => (Role::Status, stage_word(stage).into()),
                    None => (Role::Image, "Restarting Fleet".into()),
                };
                labelled(
                    div()
                        .id("restarting")
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .font_family(sans(look))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .text_color(look.color(token::FG_MUTED))
                        .tooltip(tip(tip_said))
                        .child(spinner("restarting-spin", px(12.), look.color(token::FG_MUTED), _cx))
                        .children(stage.map(stage_word)),
                    role,
                    name,
                )
                .into_any_element()
            }
        }
    }

    fn build_select_pressed(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) {
        if self.menu.as_ref().is_some_and(|menu| menu.kind == MenuKind::Build) {
            self.menu = None;
            cx.notify();
            return;
        }
        let Some(from) = self.at.select.get() else { return };
        let Some(on) = self.face.build.as_ref().map(|held| held.report.on) else { return };
        let entries = vec![
            Entry::Choice { id: "main", label: "Main".into(), on: on == BuildSource::Main },
            Entry::Choice { id: "preview", label: "Preview".into(), on: on == BuildSource::Preview },
        ];
        self.open_menu(MenuKind::Build, entries, "Build", from, false, window, cx);
    }

    /// The Keep and Revert bar: one line across the top of the screen, above the Fleet notice.
    pub(super) fn trying_bar(&self, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let trying = &self.face.build.as_ref()?.report.trying;
        if trying.is_empty() {
            return None;
        }
        let said: SharedString = format!("Trying {}", trying.join(", ")).into();
        let pending = self.face.build.as_ref().is_some_and(|held| held.report.reverting);
        let keep = Btn::new("trying-keep", "Keep", Weight::Secondary).disabled(pending).draw(look, cx);
        let keep = if pending { keep } else { keep.on_click(cx.listener(|this, _, _, cx| this.keep_pressed(cx))) };
        let revert = Btn::new("trying-revert", if pending { "Reverting" } else { "Revert" }, Weight::Secondary).pending(pending).disabled(pending).draw(look, cx);
        let revert = if pending { revert } else { revert.on_click(cx.listener(|this, _, window, cx| this.revert_confirmed(window, cx))) };
        Some(
            labelled(
                div()
                    .id("trying-bar")
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(look.length(token::SPACE_4))
                    .px(look.length(token::SPACE_4))
                    .py(look.length(token::SPACE_2))
                    // The state's hue on the frame: this build is not yet kept.
                    .border_t(look.length(token::BORDER_WIDTH))
                    .border_b(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_SUNKEN))
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(glyph("flask-conical", look.length(token::SPACE_4), look.color(token::STATUS_AWAITING_REVIEW)))
                    .child(div().flex_1().min_w_0().truncate().child(said))
                    .child(div().flex_none().flex().gap(look.length(token::SPACE_2)).child(labelled(keep, Role::Button, "Keep")).child(labelled(
                        revert,
                        Role::Button,
                        if pending { "Reverting" } else { "Revert" },
                    ))),
                Role::Region,
                "Trying",
            )
            .into_any_element(),
        )
    }

    /// The Revert confirm or the Drones confirm, over the window, when one is asked.
    pub(super) fn dialog_layer(&self, look: &Look, room: gpui::Bounds<Pixels>, cx: &Context<Self>) -> Option<AnyElement> {
        let dialog = self.dialog?;
        let held = self.face.build.as_ref()?;
        let (title, confirm, body): (SharedString, &'static str, AnyElement) = match dialog {
            Dialog::Revert => {
                let previous = held.report.previous.as_ref()?;
                let title = format!("Revert to the build of {}", built_said(&previous.built_at));
                (title.into(), "Revert", self.revert_rows(held, look))
            }
        };
        let act = dialog;
        let confirmed = cx.listener(move |this, _, window, cx| this.dialog_confirmed(act, window, cx));
        let cancelled = cx.listener(|this, _, window, cx| this.escape(window, cx));
        let scrim = div()
            .id("dialog-scrim")
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .p(look.length(token::SPACE_6))
            .bg(look.color(token::SCRIM))
            .occlude()
            .on_click(cx.listener(|this, _, window, cx| this.escape(window, cx)))
            .child(labelled(
                card(look)
                    .id("dialog")
                    .track_focus(&self.dialog_focus)
                    .capture_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| match event.keystroke.key.as_str() {
                        "escape" => {
                            this.escape(window, cx);
                            cx.stop_propagation();
                        }
                        "enter" => {
                            this.dialog_confirmed(act, window, cx);
                            cx.stop_propagation();
                        }
                        _ => {}
                    }))
                    // A press inside the dialog is not a press on the scrim.
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .w_full()
                    .max_w(look.length(token::W_DIALOG))
                    .gap(look.length(token::SPACE_4))
                    .p(look.length(token::PAD_CARD))
                    .child(
                        div()
                            .text_size(look.length(token::TEXT_BASE))
                            .line_height(look.length(token::LEADING_BASE))
                            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                            .child(title.clone()),
                    )
                    .child(body)
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(look.length(token::SPACE_2))
                            .child(labelled(Btn::new("dialog-cancel", "Cancel", Weight::Secondary).draw(look, cx).on_click(cancelled), Role::Button, "Cancel"))
                            .child(labelled(Btn::new("dialog-confirm", confirm, Weight::Primary).draw(look, cx).on_click(confirmed), Role::Button, confirm)),
                    ),
                Role::Dialog,
                title,
            ));
        Some(
            deferred(anchored().position(room.origin).anchor(Anchor::TopLeft).child(div().w(room.size.width).h(room.size.height).child(scrim)))
                .with_priority(2)
                .into_any_element(),
        )
    }

    fn dialog_confirmed(&mut self, dialog: Dialog, window: &mut gpui::Window, cx: &mut Context<Self>) {
        match dialog {
            Dialog::Revert => self.revert_confirmed(window, cx),
        }
    }

    fn revert_rows(&self, held: &Build, look: &Look) -> AnyElement {
        let report = &held.report;
        let Some(previous) = report.previous.as_ref() else { return div().into_any_element() };
        let running = report.running.as_ref().map(|one| built_said(&one.built_at));
        let running_said = running.clone().unwrap_or_else(|| "running".into());
        let before = built_said(&previous.built_at);
        let row = |label: &'static str, value: AnyElement| -> AnyElement {
            div()
                .flex()
                .gap(look.length(token::SPACE_4))
                .child(div().flex_none().w(look.length(token::SPACE_12) * 2.).text_color(look.color(token::FG_MUTED)).child(label))
                .child(div().flex_1().min_w_0().child(value))
                .into_any_element()
        };
        let text = |said: String| div().child(said).into_any_element();
        let jobs = |label: &'static str, names: &[String], why: String| -> AnyElement {
            row(
                label,
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .child(labelled(
                        div().id(label).flex().flex_col().gap(look.length(token::SPACE_1)).children(names.iter().map(|one| div().child(one.clone()))),
                        Role::List,
                        label,
                    ))
                    .child(div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(why))
                    .into_any_element(),
            )
        };
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .children(report.running.as_ref().map(|one| row("Running now", text(kept_said(&one.built_at, &one.commit)))))
            .child(row("Goes back to", text(kept_said(&previous.built_at, &previous.commit))))
            .children((!previous.takes_out.is_empty()).then(|| {
                jobs(
                    "Jobs taken out",
                    &previous.takes_out,
                    format!("Their changes are in the {running_said} build and not in the {before} one, so your app goes without them. The Jobs themselves carry on."),
                )
            }))
            .children((!previous.brings_back.is_empty()).then(|| {
                jobs(
                    "Jobs brought back",
                    &previous.brings_back,
                    format!("Their changes are in the {before} build and not in the {running_said} one, so your app gets them back."),
                )
            }))
            .child(row("Restarts", text(if previous.bridge_reopens { "Fleet, and this window" } else { "Fleet. This window stays open" }.into())))
            .children((!report.carried.is_empty()).then(|| {
                jobs("Jobs still working", &report.carried, "These Jobs are at work now. Their agents keep running while Fleet restarts.".into())
            }))
            .into_any_element()
    }

}
