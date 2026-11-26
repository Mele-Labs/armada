//! The Fleet build switcher in the Fleet popover, the Keep and Revert bar above the screen, and the
//! dialog that confirms a revert (`FleetBuild.tsx`, `trying-bar.tsx`).

use gpui::{
    anchored, canvas, deferred, div, prelude::*, px, Anchor, AnyElement, Context, FontWeight, Hsla, Pixels, Role, SharedString,
};
use ipc::{BuildSource, CheckoutBlocked, ClearBlocked};

use super::face::{ahead_said, behind_said, blocked_said, built_said, kept_said, refreshing, restarting as restarting_of, stage_word, Build, Position};
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
    // gpui clips a card's children to its box, not its rounded corners, so the band rounds its own: the top
    // two, or all four where no row follows. The card's radius less its hairline, so the curves sit inside it.
    let inner = look.length(token::RADIUS_LG) - look.length(token::BORDER_WIDTH);
    let alone = rows.is_empty();
    let band = div()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .rounded_t(inner)
        .when(alone, |band| band.rounded_b(inner))
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

    /// The build block of the Fleet popover, in two parts: the lists of what runs and what waits to be added,
    /// which scroll, and the presses that build it with the line that says how far it got, which stay in view.
    pub(super) fn build_section(&self, held: &Build, look: &Look, cx: &Context<Self>) -> (AnyElement, AnyElement) {
        let report = &held.report;
        let offers = self.face.offers.as_ref();
        let busy = restarting_of(report) || report.reverting || report.swapping;
        let restarting = restarting_of(report);
        let work = story::held(report, offers);
        let keep_row = (!report.trying.is_empty()).then(|| {
            let amber = look.color(token::STATUS_AWAITING_REVIEW);
            let keep = Btn::new("keep-build", "Keep", Weight::Secondary).disabled(report.reverting).draw(look, cx).track_focus(&self.list_stops.keep).tooltip(tip("Keep this build and tell its Jobs to land"));
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
                let (icon, ink, said, word): (&'static str, Hsla, &'static str, Option<&'static str>) = if one.landed {
                    ("git-merge", look.color(token::STATUS_COMPLETED_SUCCESS), "On main", Some("on main"))
                } else if one.offered {
                    ("git-branch", look.color(token::STATUS_RUNNING), "Offered by its Job for the preview", Some("offered"))
                } else {
                    ("git-branch", look.color(token::FG_MUTED), "In this build", None)
                };
                // The name is the branch where no Job names it, so the branch goes on hover only beside a title.
                let hover = (one.branches.len() > 1 || one.branches.first().is_some_and(|branch| *branch != one.name)).then(|| one.branches.join(", ").into());
                row(
                    look,
                    "held",
                    at,
                    Row {
                        mark: state_glyph(format!("held-mark-{at}").into(), icon, ink, said.into()),
                        name: one.name.clone().into(),
                        hover,
                        about: None,
                        word: word.map(|word| (word, ink)),
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
        let refreshing = refreshing(report);
        let updating = report.restarting == Some(BuildSource::Main);
        let said = |text: SharedString| -> AnyElement {
            div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(text).into_any_element()
        };
        // Another Mac paired: Refresh takes it along, and a Mac pressing alone is what froze the link.
        let partner = crate::pairing::refresh_partner(crate::pairing::macs(cx)).map(|other| other.name.clone());
        let refresh = (on == BuildSource::Preview || refresh_row.is_some()).then(|| {
            let label = match (refreshing.is_some(), partner.is_some()) {
                (true, _) => "Refreshing",
                (false, true) => "Refresh both Macs",
                (false, false) => "Refresh",
            };
            let weight = if refresh_row.is_some() { Weight::Primary } else { Weight::Secondary };
            let tooltip = match (&refreshing, &partner) {
                (Some(now), _) => now.tip(crate::pairing::macs(cx)),
                (None, Some(name)) => format!("Merge the offered branches onto latest main, restart Fleet, and have {name} build the same commit"),
                (None, None) => "Merge the offered branches onto latest main and restart Fleet".into(),
            };
            let button = Btn::new("refresh-build", label, weight).icon("rotate-cw").pending(refreshing.is_some()).disabled(busy).full().draw(look, cx).flex_1().tooltip(tip(tooltip));
            let button = if busy {
                button
            } else if partner.is_some() {
                button.on_click(cx.listener(|this, _, _, cx| this.refresh_both(cx)))
            } else {
                button.on_click(cx.listener(|this, _, window, cx| this.change_build(BuildSource::Preview, None, window, cx)))
            };
            // Beside it, the press that moves this Mac only. It stands while a restart is under way, off.
            let alone = partner.is_some().then(|| {
                let button = Btn::new("refresh-this-mac", "This Mac only", Weight::Secondary).disabled(busy).draw(look, cx).tooltip(tip("Merge the offered branches onto latest main and restart Fleet here only"));
                let button = if busy { button } else { button.on_click(cx.listener(|this, _, window, cx| this.change_build(BuildSource::Preview, None, window, cx))) };
                labelled(button, Role::Button, "This Mac only")
            });
            let skip = refresh_row.map(|refresh| {
                let id = refresh.id.clone();
                let button = Btn::new("skip-offers", "Skip", Weight::Secondary).draw(look, cx).tooltip(tip("Hide what waits until the offers change"));
                labelled(button.on_click(cx.listener(move |this, _, window, cx| this.skip_offers_pressed(id.clone(), window, cx))), Role::Button, "Skip")
            });
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(labelled(button, Role::Button, label)).children(alone).children(skip))
                .children((!restarting).then(|| {
                    // The line says it in a few words; hover names every Job it brings in.
                    let macs = crate::pairing::macs(cx);
                    let line: SharedString = story::refresh_said(offers, macs).into();
                    let whole = story::refresh_whole(offers, macs);
                    let shown = div().id("refresh-said").text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).tooltip(tip(whole)).child(line.clone());
                    labelled(shown, Role::Status, line).into_any_element()
                }))
                .children(refreshing.as_ref().and_then(|now| now.started_said(crate::pairing::macs(cx))).map(|started| {
                    let started: SharedString = started.into();
                    let line = div().id("refresh-started").text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(started.clone());
                    labelled(line, Role::Status, started).into_any_element()
                }))
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
            let button = if off { button } else { button.on_click(cx.listener(|this, _, window, cx| this.change_build(BuildSource::Main, None, window, cx))) };
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(labelled(button, Role::Button, label))
                .children((!restarting && !idle).then(|| said(story::UPDATE_SAID.into())))
        });
        // A restart under way says its stage where the sentences stood.
        let stage = matches!(held.position(), Position::Restarting { stage: Some(_), .. }).then(|| self.position_marks(held, look, cx));
        let blocked = (!restarting).then_some(report.blocked.as_ref()).flatten().map(|blocked| self.blocked_section(blocked, held.restart().target, busy, look, cx));
        let failed = (!restarting).then(|| report.failed.clone()).flatten().map(|said| self.failed_line(look, "build-failed", said.into()));
        let lists = div().id("fleet-build").flex().flex_col().gap(look.length(token::SPACE_3)).child(running).children(self.notes_row(look, cx)).children(self.waiting_section(held, look, cx)).into_any_element();
        let presses = div().id("fleet-build-presses").flex().flex_col().gap(look.length(token::SPACE_3)).children(update).children(refresh).children(blocked).children(stage).children(failed).into_any_element();
        (lists, presses)
    }

    /// Revert, where a build before is kept: it asks first, in the dialog that says what it changes.
    fn revert_button(&self, held: &Build, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let report = &held.report;
        let said = held.revert_tip()?;
        let label = if report.reverting { "Reverting" } else { "Revert" };
        let off = restarting_of(report) || report.reverting || report.swapping;
        let button = Btn::new("revert-build", label, Weight::Secondary).icon("undo-2").pending(report.reverting).disabled(off).draw(look, cx).track_focus(&self.list_stops.revert).tooltip(tip(said));
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

    /// The untracked files that stop the checkout's main updating, and the two presses that move them and restart
    /// onto `target`, the build the restart button would restart onto.
    fn blocked_section(&self, blocked: &CheckoutBlocked, target: BuildSource, busy: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let amber = look.color(token::STATUS_AWAITING_REVIEW);
        let said: SharedString = blocked_said(blocked.behind).into();
        let line = div()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .child(div().flex_none().h(look.length(token::LEADING_XS)).flex().items_center().child(state_glyph("blocked-mark".into(), "triangle-alert", amber, "Main cannot update".into())))
            .child(labelled(div().id("blocked-said").flex_1().min_w_0().text_color(look.color(token::FG_MUTED)).child(said.clone()), Role::Label, said));
        // A long list scrolls in place, so the presses under it stay in view.
        let files = labelled(
            div()
                .id("blocked-files")
                .flex()
                .flex_col()
                .max_h(look.length(token::LEADING_XS) * 6.)
                .overflow_y_scroll()
                .font_family(mono(look))
                .text_color(look.color(token::FG_DEFAULT))
                .children(blocked.files.iter().enumerate().map(|(at, path)| {
                    let path = SharedString::from(path.clone());
                    labelled(div().id(("blocked-file", at)).truncate().tooltip(tip(path.clone())).child(path.clone()), Role::ListItem, path)
                })),
            Role::List,
            "Untracked files",
        );
        let press = |id: &'static str, label: &'static str, weight: Weight, icon: &'static str, said: &'static str, clear: ClearBlocked| {
            let button = Btn::new(id, label, weight).icon(icon).disabled(busy).full().draw(look, cx).flex_1().tooltip(tip(said));
            let button = if busy { button } else { button.on_click(cx.listener(move |this, _, window, cx| this.change_build(target, Some(clear), window, cx))) };
            labelled(button, Role::Button, label)
        };
        let stash = press("blocked-stash", "Stash and refresh", Weight::Primary, "archive", "Keeps these files in a git stash, then updates main and restarts", ClearBlocked::Stash);
        let delete = press("blocked-delete", "Delete and refresh", Weight::Secondary, "trash-2", "Deletes these files for good, then updates main and restarts", ClearBlocked::Delete);
        labelled(
            div()
                .id("blocked")
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .child(line)
                .child(files)
                .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(stash).child(delete)),
            Role::Group,
            "Main cannot update",
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
                let started = crate::titlebar::refreshing(&held.report).and_then(|now| now.started_said(crate::pairing::macs(_cx)));
                let tip_said = match started {
                    Some(started) => format!("Restarting Fleet on {on}. {started}"),
                    None => format!("Restarting Fleet on {on}"),
                };
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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use gpui::{Entity, Modifiers, Role, TestAppContext, VisualTestContext};
    use ipc::{BuildSource, ClearBlocked, FleetBuildReport};

    use super::super::face::{build_of, Face};
    use super::super::{scenes, Asked, AskedSink, TitleBar};

    const PREVIEW: &str = r#""on":"preview","commit":"c04d8e7f6a5b4c3d2e1f0a9b8c7d6e5f4a3b2c1d","position":{"ahead":5,"behind":2}"#;
    const BLOCKED: &str = r#""blocked":{"behind":4,"files":[".armada/workflows/bug.yml",".armada/workflows/review.yml"]}"#;
    const SAID: &str = "Main is 4 commits behind origin. These untracked files block its update:";

    fn redraw(cx: &mut VisualTestContext) {
        for _ in 0..3 {
            cx.run_until_parked();
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }
    }

    /// A bar whose Fleet panel stands open on `rest` of a build report, and what the bar asked.
    fn opened<'a>(cx: &'a mut TestAppContext, rest: &str) -> (Entity<TitleBar>, &'a mut VisualTestContext, Rc<RefCell<Vec<Asked>>>) {
        opened_with(cx, rest, Vec::new())
    }

    /// The same, with `macs` held as the Gateway listed them.
    fn opened_with<'a>(cx: &'a mut TestAppContext, rest: &str, macs: Vec<ipc::Mac>) -> (Entity<TitleBar>, &'a mut VisualTestContext, Rc<RefCell<Vec<Asked>>>) {
        cx.update(crate::init_app);
        cx.update(|cx| crate::pairing::hold(macs, cx));
        let asked = Rc::new(RefCell::new(Vec::new()));
        let heard = Rc::clone(&asked);
        cx.update(|cx| cx.set_global(AskedSink(Rc::new(move |one: &Asked, _: &mut gpui::App| heard.borrow_mut().push(one.clone())))));
        let (bar, cx) = cx.add_window_view(|window, cx| TitleBar::new(window, cx));
        let report = ipc::decode::<FleetBuildReport>("FleetBuildReport", format!("{{{rest}}}").as_bytes()).expect("the report decodes");
        bar.update(cx, |bar, cx| {
            bar.reset(Face { build: Some(build_of(&report)), ..scenes::rest() });
            cx.notify();
        });
        redraw(cx);
        cx.update(|window, cx| bar.update(cx, |bar, cx| bar.fleet_pressed(window, cx)));
        redraw(cx);
        (bar, cx, asked)
    }

    fn drawn(cx: &mut VisualTestContext, role: Role, name: &str) -> Option<gpui::Bounds<gpui::Pixels>> {
        cx.update(|window, cx| cx.try_global::<crate::labels::LabelBook>().and_then(|book| book.find(window.window_handle().window_id(), role, name)).map(|one| one.shown))
    }

    fn changes(asked: &RefCell<Vec<Asked>>) -> Vec<Asked> {
        asked.borrow().iter().filter(|one| matches!(one, Asked::ChangeBuild { .. })).cloned().collect()
    }

    #[gpui::test]
    fn a_blocked_checkout_draws_how_far_behind_its_files_and_both_presses(cx: &mut TestAppContext) {
        let (_bar, cx, _) = opened(cx, &format!("{PREVIEW},{BLOCKED}"));
        assert!(drawn(cx, Role::Label, SAID).is_some(), "the line says how far main is behind");
        assert!(drawn(cx, Role::List, "Untracked files").is_some());
        for file in [".armada/workflows/bug.yml", ".armada/workflows/review.yml"] {
            assert!(drawn(cx, Role::ListItem, file).is_some(), "{file} is listed");
        }
        let stash = drawn(cx, Role::Button, "Stash and refresh").expect("Stash and refresh is drawn");
        let delete = drawn(cx, Role::Button, "Delete and refresh").expect("Delete and refresh is drawn");
        assert_eq!((stash.origin.y, stash.size.height), (delete.origin.y, delete.size.height), "the two presses share a line and a height");
    }

    #[gpui::test]
    fn nothing_is_drawn_for_a_checkout_that_is_not_blocked(cx: &mut TestAppContext) {
        let (_bar, cx, _) = opened(cx, PREVIEW);
        assert!(drawn(cx, Role::Button, "Refresh").is_some(), "the panel is open");
        assert!(drawn(cx, Role::Button, "This Mac only").is_none(), "one Mac: one Refresh, as before");
        assert!(drawn(cx, Role::Button, "Refresh both Macs").is_none());
        assert!(drawn(cx, Role::Button, "Stash and refresh").is_none());
        assert!(drawn(cx, Role::List, "Untracked files").is_none());
    }

    #[gpui::test]
    fn nothing_is_drawn_while_a_restart_is_under_way(cx: &mut TestAppContext) {
        let (_bar, cx, _) = opened(cx, &format!(r#"{PREVIEW},"restarting":"preview",{BLOCKED}"#));
        assert!(drawn(cx, Role::Button, "Refreshing").is_some(), "the panel is open on the restart");
        assert!(drawn(cx, Role::Label, SAID).is_none());
        assert!(drawn(cx, Role::Button, "Stash and refresh").is_none());
        assert!(drawn(cx, Role::Button, "Delete and refresh").is_none());
    }

    #[gpui::test]
    fn stash_and_refresh_restarts_onto_the_build_with_stash(cx: &mut TestAppContext) {
        let (bar, cx, asked) = opened(cx, &format!("{PREVIEW},{BLOCKED}"));
        let at = drawn(cx, Role::Button, "Stash and refresh").expect("Stash and refresh is drawn");
        cx.simulate_click(at.center(), Modifiers::none());
        redraw(cx);
        assert_eq!(changes(&asked), [Asked::ChangeBuild { build: BuildSource::Preview, clear: Some(ClearBlocked::Stash) }]);
        // A bare bar's label book is never cleared, so what went is read from the bar and not the book.
        let restarting = bar.read_with(cx, |bar, _| bar.face.build.as_ref().and_then(|held| held.report.restarting));
        assert_eq!(restarting, Some(BuildSource::Preview), "the restart is under way in the same frame, so the files and their presses go");
    }

    #[gpui::test]
    fn delete_and_refresh_restarts_onto_the_build_with_delete(cx: &mut TestAppContext) {
        let (_bar, cx, asked) = opened(cx, &format!(r#""on":"main","commit":"9f3c2a1b7d0e4c5a8b6f1d2e3a4b5c6d7e8f9a0b","position":{{"ahead":0,"behind":3}},{BLOCKED}"#));
        let at = drawn(cx, Role::Button, "Delete and refresh").expect("Delete and refresh is drawn");
        cx.simulate_click(at.center(), Modifiers::none());
        redraw(cx);
        assert_eq!(changes(&asked), [Asked::ChangeBuild { build: BuildSource::Main, clear: Some(ClearBlocked::Delete) }]);
    }

    #[gpui::test]
    fn a_paired_mac_makes_refresh_move_both_and_leaves_this_mac_only_beside_it(cx: &mut TestAppContext) {
        let (_bar, cx, asked) = opened_with(cx, PREVIEW, crate::pairing::mock_macs(true));
        assert!(drawn(cx, Role::Button, "Refresh").is_none(), "the lone Refresh is gone");
        let both = drawn(cx, Role::Button, "Refresh both Macs").expect("Refresh both Macs is drawn");
        let alone = drawn(cx, Role::Button, "This Mac only").expect("This Mac only is drawn");
        assert_eq!(both.origin.y, alone.origin.y, "the two presses share a line");
        cx.simulate_click(alone.center(), Modifiers::none());
        redraw(cx);
        assert_eq!(changes(&asked), [Asked::ChangeBuild { build: BuildSource::Preview, clear: None }], "This Mac only is today's change");
        assert!(!asked.borrow().contains(&Asked::RefreshBoth));
    }

    const CLAIMED: &str = r#""refresh":{"mac":"air","name":"Air","stage":"building_fleet","at":"2026-11-19T09:00:00Z"}"#;

    #[gpui::test]
    fn a_refresh_another_mac_started_reads_refreshing_here_and_its_keys_are_off_until_it_is_cleared(cx: &mut TestAppContext) {
        let (bar, cx, asked) = opened_with(cx, &format!("{PREVIEW},{CLAIMED}"), crate::pairing::mock_macs(true));
        assert!(drawn(cx, Role::Button, "Refreshing").is_some(), "the Refresh row reads Refreshing with no restart of this Mac's own");
        assert!(drawn(cx, Role::Status, "Started on Air").is_some(), "it names the Mac that started it");
        assert!(drawn(cx, Role::Button, "Refresh both Macs").is_none(), "the press is gone");
        let alone = drawn(cx, Role::Button, "This Mac only").expect("This Mac only stands, off");
        cx.simulate_click(alone.center(), Modifiers::none());
        redraw(cx);
        assert!(changes(&asked).is_empty() && !asked.borrow().contains(&Asked::RefreshBoth), "no press reaches Fleet while the claim stands: {:?}", asked.borrow());
        // Cleared: Fleet's next report carries no claim.
        let report = ipc::decode::<FleetBuildReport>("FleetBuildReport", format!("{{{PREVIEW}}}").as_bytes()).expect("the report decodes");
        bar.update(cx, |bar, cx| {
            bar.reset(Face { build: Some(build_of(&report)), ..scenes::rest() });
            cx.notify();
        });
        redraw(cx);
        // Reset shuts the panel; he opens it again. A bare bar's label book is never cleared, so Refreshing's stale entry is not asked; the press coming back is.
        cx.update(|window, cx| bar.update(cx, |bar, cx| bar.fleet_pressed(window, cx)));
        redraw(cx);
        assert!(drawn(cx, Role::Button, "Refresh both Macs").is_some(), "claim cleared, the keys are back");
        let both = drawn(cx, Role::Button, "Refresh both Macs").expect("drawn");
        cx.simulate_click(both.center(), Modifiers::none());
        redraw(cx);
        assert!(asked.borrow().contains(&Asked::RefreshBoth), "and press");
    }

    #[gpui::test]
    fn refresh_both_macs_asks_for_both(cx: &mut TestAppContext) {
        let (bar, cx, asked) = opened_with(cx, PREVIEW, crate::pairing::mock_macs(true));
        let at = drawn(cx, Role::Button, "Refresh both Macs").expect("Refresh both Macs is drawn");
        cx.simulate_click(at.center(), Modifiers::none());
        redraw(cx);
        assert!(asked.borrow().contains(&Asked::RefreshBoth));
        assert!(changes(&asked).is_empty());
        let restarting = bar.read_with(cx, |bar, _| bar.face.build.as_ref().and_then(|held| held.report.restarting));
        assert_eq!(restarting, Some(BuildSource::Preview), "the restart reads as it does for a single Refresh");
    }
}
