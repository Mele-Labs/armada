//! The Fleet build switcher in the Fleet popover, the Keep and Revert bar above the screen, and the
//! dialog that confirms a revert (`FleetBuild.tsx`, `trying-bar.tsx`).

use gpui::{
    anchored, canvas, deferred, div, prelude::*, px, Anchor, AnyElement, Context, FontWeight, Hsla, Pixels, Role, SharedString,
};
use ipc::BuildSource;

use super::face::{ahead_said, behind_said, built_said, kept_said, stage_word, Build, Position};
use super::parts::{card, glyph, mono, sans, spinner, tip, Btn, Weight};
use super::{story, Dialog, MenuKind, TitleBar};
use crate::dropdown::Entry;
use crate::labels::labelled;
use crate::look::{token, Look, Token};

/// One row of a panel section: its state glyph, the work's name with its branches on hover, a line under it,
/// a word in the state's hue, and a control at its end.
pub(super) struct Row {
    pub mark: AnyElement,
    pub name: SharedString,
    pub hover: Option<SharedString>,
    pub about: Option<SharedString>,
    pub word: Option<(&'static str, Hsla)>,
    pub end: Option<AnyElement>,
}

/// A row, a hairline over it. `kind` and `at` give it and its parts their ids.
pub(super) fn row(look: &Look, kind: &'static str, at: usize, one: Row) -> AnyElement {
    let name = div()
        .id(SharedString::from(format!("{kind}-name-{at}")))
        .min_w_0()
        .text_size(look.length(token::TEXT_SM))
        .line_height(look.length(token::LEADING_SM))
        .text_color(look.color(token::FG_DEFAULT))
        .child(one.name);
    let name = match one.hover {
        Some(said) => name.tooltip(tip(said)),
        None => name,
    };
    let about = one.about.map(|said| {
        div()
            .id(SharedString::from(format!("{kind}-about-{at}")))
            .min_w_0()
            // Two lines, then an ellipsis; the whole line is in the tooltip.
            .line_clamp(2)
            .text_ellipsis()
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_MUTED))
            .tooltip(tip(said.clone()))
            .child(said)
    });
    div()
        .id((kind, at))
        .flex()
        .items_start()
        .gap(look.length(token::SPACE_2))
        .px(look.length(token::SPACE_3))
        .py(look.length(token::SPACE_2))
        .border_t(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_SUBTLE))
        .child(div().flex_none().h(look.length(token::LEADING_SM)).flex().items_center().child(one.mark))
        .child(div().flex_1().min_w_0().flex().flex_col().child(name).children(about))
        .children(one.word.map(|(word, ink)| {
            div().flex_none().h(look.length(token::LEADING_SM)).flex().items_center().text_size(look.length(token::TEXT_XS)).text_color(ink).child(word)
        }))
        .children(one.end)
        .into_any_element()
}

/// A state as a 12px registry glyph in its hue, said in its tooltip.
pub(super) fn state_glyph(id: SharedString, name: &'static str, ink: Hsla, said: SharedString) -> AnyElement {
    labelled(div().id(id).flex_none().tooltip(tip(said.clone())).child(glyph(name, px(12.), ink)), Role::Image, said).into_any_element()
}

/// A section of the panel: a band in the state's hue with its line and a control, then the rows on the
/// sunken ground. `sub` is the line and what its tooltip says whole.
#[allow(clippy::too_many_arguments)]
pub(super) fn band_card(
    look: &Look,
    id: &'static str,
    icon: &'static str,
    hue: Token,
    ground: Token,
    title: &'static str,
    sub: Option<(SharedString, SharedString)>,
    end: Option<AnyElement>,
    rows: Vec<AnyElement>,
) -> gpui::Stateful<gpui::Div> {
    let ink = look.color(hue);
    let line = match sub {
        Some((said, whole)) => div()
            .id(SharedString::from(format!("{id}-said")))
            .flex_1()
            .min_w_0()
            .truncate()
            .text_color(look.color(token::FG_MUTED))
            .tooltip(tip(whole))
            .child(said)
            .into_any_element(),
        None => div().flex_1().into_any_element(),
    };
    let band = div()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .min_h(look.length(token::H_CONTROL))
        .px(look.length(token::SPACE_3))
        .py(look.length(token::SPACE_1))
        // The state's hue on the band's edge and in its ground.
        .border_l(look.length(token::BORDER_WIDTH) * 3.)
        .border_color(ink)
        .bg(look.color(ground))
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .child(glyph(icon, px(12.), ink))
        .child(div().flex_none().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(title))
        .child(line)
        .children(end);
    div()
        .id(id)
        .flex()
        .flex_col()
        .overflow_hidden()
        .rounded(look.length(token::RADIUS_LG))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_SUBTLE))
        .bg(look.color(token::BG_SUNKEN))
        .child(band)
        .children(rows)
}

/// What a restart costs a Drone that is working, on hover.

impl TitleBar {
    /// The build Fleet runs on, as a chip in the panel's head: a press opens Main and Preview.
    pub(super) fn build_chip(&self, held: &Build, look: &Look, cx: &Context<Self>) -> AnyElement {
        let working = held.restart().pending;
        let select_at = std::rc::Rc::clone(&self.at.select);
        let (name, hue, ground) = match held.report.on {
            BuildSource::Main => ("Main", token::STATUS_COMPLETED_SUCCESS, token::STATUS_COMPLETED_SUCCESS_BG),
            BuildSource::Preview => ("Preview", token::STATUS_RUNNING, token::STATUS_RUNNING_BG),
        };
        let ink = look.color(hue);
        let ring = Self::focus_ring(look);
        labelled(
            div()
                .id("build-select")
                .relative()
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .h(look.length(token::H_CONTROL_SM))
                .px(look.length(token::SPACE_2))
                .rounded_full()
                .border(look.length(token::BORDER_WIDTH))
                .border_color(gpui::transparent_black())
                .bg(look.color(ground))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(ink)
                .when(!working, |chip| {
                    chip.cursor_pointer()
                        .focusable()
                        .tab_stop(true)
                        .focus_visible(move |style| style.border_color(ring))
                        .hover(move |style| style.border_color(ink))
                        .on_click(cx.listener(|this, _, window, cx| this.build_select_pressed(window, cx)))
                })
                .when(working, |chip| chip.opacity(0.5))
                .child(canvas(move |bounds, _, _| select_at.set(Some(bounds)), |_, _, _, _| {}).absolute().size_full())
                .child(name)
                .child(glyph("chevron-down", look.length(token::SPACE_3), ink))
                .tooltip(tip("The build Fleet runs on")),
            Role::Button,
            "Build",
        )
        .into_any_element()
    }

    /// The build block of the Fleet popover: what runs, what waits to be added, and the press that builds it.
    pub(super) fn build_section(&self, held: &Build, look: &Look, cx: &Context<Self>) -> AnyElement {
        let report = &held.report;
        let offers = self.face.offers.as_ref();
        let busy = report.restarting.is_some() || report.reverting || report.swapping;
        let restarting = report.restarting.is_some();
        let work = story::held(report, offers);
        let keep_row = (!report.trying.is_empty()).then(|| {
            let amber = look.color(token::STATUS_AWAITING_REVIEW);
            let keep = Btn::new("keep-build", "Keep", Weight::Secondary).disabled(report.reverting).draw(look, cx).tooltip(tip("Keep this build and tell its Jobs to land"));
            let keep = if report.reverting { keep } else { keep.on_click(cx.listener(|this, _, _, cx| this.keep_pressed(cx))) };
            div()
                .id("keep-row")
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                .border_t(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .child(labelled(
                    div()
                        .id("not-kept")
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_1))
                        .h(look.length(token::H_CONTROL_SM))
                        .px(look.length(token::SPACE_2))
                        .rounded_full()
                        .bg(look.color(token::STATUS_AWAITING_REVIEW_BG))
                        .text_color(amber)
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                        .tooltip(tip("Keep has not been pressed on this build"))
                        .child(glyph("circle-dot", px(12.), amber))
                        .child("Not kept"),
                    Role::Image,
                    "Not kept",
                ))
                .child(div().flex_1())
                .child(labelled(keep, Role::Button, "Keep"))
                .children(self.revert_button(held, look, cx))
                .into_any_element()
        });
        let rows: Vec<AnyElement> = work
            .iter()
            .enumerate()
            .map(|(at, one)| {
                let (ink, said): (Hsla, &'static str) =
                    if one.offered { (look.color(token::STATUS_RUNNING), "Offered by its Job for the preview") } else { (look.color(token::FG_MUTED), "In this build") };
                row(
                    look,
                    "held",
                    at,
                    Row {
                        mark: state_glyph(format!("held-mark-{at}").into(), "git-branch", ink, said.into()),
                        name: one.name.clone().into(),
                        hover: (!one.branches.is_empty()).then(|| one.branches.join(", ").into()),
                        about: None,
                        word: one.offered.then_some(("offered", ink)),
                        end: None,
                    },
                )
            })
            .collect();
        let sub = story::running_said(held).map(|said| {
            let whole = match report.position {
                Some(at) if at.ahead > 0 => format!("{said}. {}", ahead_said(at.ahead)),
                _ => said.clone(),
            };
            (SharedString::from(said), SharedString::from(whole))
        });
        // Revert sits in the band once the build is kept; before, it stands beside Keep.
        let band_end = if restarting || keep_row.is_some() { None } else { self.revert_button(held, look, cx) };
        let running = labelled(
            band_card(look, "running", "play", token::STATUS_RUNNING, token::STATUS_RUNNING_BG, "Running", sub, band_end, keep_row.into_iter().chain(rows).collect()),
            Role::Group,
            "Running",
        );
        // Each press with the sentence that says what it builds.
        let refresh_row = offers.and_then(|offers| offers.refresh.as_ref());
        let on = report.on;
        let refreshing = report.restarting == Some(BuildSource::Preview);
        let updating = report.restarting == Some(BuildSource::Main);
        let said = |text: SharedString| -> AnyElement {
            div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(text).into_any_element()
        };
        let refresh = (on == BuildSource::Preview || refresh_row.is_some()).then(|| {
            let label = if refreshing { "Refreshing" } else { "Refresh" };
            let weight = if refresh_row.is_some() { Weight::Primary } else { Weight::Secondary };
            let button = Btn::new("refresh-build", label, weight)
                .icon("rotate-cw")
                .pending(refreshing)
                .disabled(busy)
                .full()
                .draw(look, cx)
                .flex_1()
                .tooltip(tip("Merge the offered branches onto latest main and restart Fleet"));
            let button = if busy { button } else { button.on_click(cx.listener(|this, _, window, cx| this.change_build(BuildSource::Preview, window, cx))) };
            let skip = refresh_row.map(|refresh| {
                let id = refresh.id.clone();
                let button = Btn::new("skip-offers", "Skip", Weight::Secondary).draw(look, cx).tooltip(tip("Hide what waits until the offers change"));
                labelled(button.on_click(cx.listener(move |this, _, window, cx| this.skip_offers_pressed(id.clone(), window, cx))), Role::Button, "Skip")
            });
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(labelled(button, Role::Button, label)).children(skip))
                .children((!restarting).then(|| said(story::refresh_said(offers).into())))
        });
        let update = (on == BuildSource::Main).then(|| {
            let idle = held.aligned && !restarting;
            let off = busy || idle;
            let label = if updating { "Updating to main" } else { "Update to main" };
            let button = Btn::new("update-build", label, Weight::Secondary)
                .icon("rotate-cw")
                .pending(updating)
                .disabled(off)
                .full()
                .draw(look, cx)
                .tooltip(tip(if idle { "On latest main" } else { "Restart Fleet on latest main" }));
            let button = if off { button } else { button.on_click(cx.listener(|this, _, window, cx| this.change_build(BuildSource::Main, window, cx))) };
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(labelled(button, Role::Button, label))
                .children((!restarting && !idle).then(|| said(story::UPDATE_SAID.into())))
        });
        // A restart under way says its stage where the sentences stood.
        let stage = matches!(held.position(), Position::Restarting { stage: Some(_), .. }).then(|| self.position_marks(held, look, cx));
        let failed = (!restarting).then(|| report.failed.clone()).flatten().map(|said| self.failed_line(look, "build-failed", said.into()));
        div()
            .id("fleet-build")
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .child(running)
            .children(self.waiting_section(held, look, cx))
            .children(update)
            .children(refresh)
            .children(stage)
            .children(failed)
            .into_any_element()
    }

    /// Revert, where a build before is kept: it asks first, in the dialog that says what it changes.
    fn revert_button(&self, held: &Build, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let report = &held.report;
        let said = held.revert_tip()?;
        let label = if report.reverting { "Reverting" } else { "Revert" };
        let off = report.restarting.is_some() || report.reverting || report.swapping;
        let button = Btn::new("revert-build", label, Weight::Secondary).icon("undo-2").pending(report.reverting).disabled(off).draw(look, cx).tooltip(tip(said));
        let button = if off { button } else { button.on_click(cx.listener(|this, _, window, cx| this.revert_pressed(window, cx))) };
        Some(labelled(button, Role::Button, label).into_any_element())
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
        // The chip stands at the panel's right, so the menu hangs from its right edge.
        self.open_menu(MenuKind::Build, entries, "Build", from, true, window, cx);
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
