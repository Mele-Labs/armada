//! The panel beside the graph: what Fleet knows of the part that is selected, in nine sections. Each says why when it is empty,
//! so a bare heading never leaves him wondering whether it is still being read.

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString, Window};
use gpui_component::input::Textarea;
use ipc::{AdditionAnswer, PartOrigin, SystemPart, SystemPartAnswers, SystemPartKind};

use super::graph::{chip, glyph_of, hue_of, kind_of, kind_word};
use super::trail::Level;
use super::tree::{self, Row};
use super::{Compose, Read, SystemBlock, FEATURE_KEY};
use crate::labels::labelled;
use crate::look::{token, Look, Token};
use crate::settings::view::{bone, ring_colour};
use crate::titlebar::parts::{glyph, mono, tip, Btn, Weight};

/// The nine sections in the order they are drawn, each with its glyph.
pub(super) const SECTIONS: [(&str, &str); 9] = [
    ("Features", "layers"),
    ("Code", "folder-tree"),
    ("Working on it", "bot"),
    ("Owner", "user"),
    ("Looks like", "image"),
    ("Docs", "book-open"),
    ("Monitor", "activity"),
    ("Open issues", "circle-dot"),
    ("Open pull requests", "git-pull-request"),
];

pub(super) const NO_FEATURES: &str = "No feature owns files in it yet.";
pub(super) const NO_JOB: &str = "No Job is writing in it.";
pub(super) const NO_DOCS: &str = "No doc names it.";
pub(super) const NO_MONITOR: &str = "Nothing to watch is declared for it.";
pub(super) const NO_PULLS: &str = "None open.";

/// Why Looks like is empty: a part that draws a screen has none kept yet, any other is not one.
pub(super) fn looks_line(has_screen: bool) -> &'static str {
    if has_screen {
        "No shot kept yet."
    } else {
        "Not a screen."
    }
}

/// Why Open issues is empty: a project with a tracker has none that name the part, any other has no tracker to ask.
pub(super) fn issues_line(tracker: bool) -> &'static str {
    if tracker {
        "No open issues name it."
    } else {
        "No tracker connected."
    }
}

/// Why Code is empty, by what the part is.
pub(super) fn code_line(kind: SystemPartKind) -> &'static str {
    match kind {
        SystemPartKind::Person => "A person.",
        SystemPartKind::Outside => "Not in this repository.",
        _ => "No folder of its own.",
    }
}

/// The line under an owner's name: their share of the commits in the window.
pub(super) fn share_line(share: u8, days: u32) -> String {
    if share >= 100 {
        format!("every commit, last {days} days")
    } else {
        format!("{share}% of commits, last {days} days")
    }
}

pub(super) fn owner_line(kind: SystemPartKind, days: u32) -> String {
    match kind {
        SystemPartKind::Person | SystemPartKind::Outside => "Not ours to own.".to_string(),
        _ => format!("No commits in the last {days} days."),
    }
}

/// What the header says under the name: what kind of part it is and where Fleet found it.
pub(super) fn kind_line(part: &SystemPart) -> String {
    match &part.origin {
        PartOrigin::Detected { from } => format!("{} · {from}", kind_word(part.kind)),
        PartOrigin::Librarian { .. } => format!("{} · from the librarian", kind_word(part.kind)),
    }
}

/// Who the panel is about.
enum Subject {
    Project,
    Feature { slug: String, title: String },
    Part(Box<SystemPart>),
}

fn subject(this: &SystemBlock) -> Subject {
    let found = |id: &str| this.level().and_then(|map| map.parts.iter().find(|part| part.id == id)).or_else(|| this.top().and_then(|map| map.parts.iter().find(|part| part.id == id))).cloned();
    // On a feature's path the panel is about the feature until a part of the path is picked.
    if let (Level::Feature { slug, title }, None) = (this.here(), this.selected.as_ref()) {
        return Subject::Feature { slug: slug.clone(), title: title.clone() };
    }
    match this.panel_part().and_then(|id| found(&id)) {
        Some(part) => Subject::Part(Box::new(part)),
        None => Subject::Project,
    }
}

pub(super) fn draw(this: &mut SystemBlock, look: &Look, stacked: bool, window: &mut Window, cx: &mut Context<SystemBlock>) -> AnyElement {
    let subject = subject(this);
    let (name, header) = match &subject {
        Subject::Project => (this.project_name().to_string(), head(this, look, glyph_name(SystemPartKind::Piece), token::ACCENT, this.project_name().to_string(), "project".to_string(), String::new(), Vec::new(), cx)),
        Subject::Feature { slug, title } => {
            let open = slug.clone();
            let buttons = vec![labelled(
                Btn::new("system-open-feature", "Open feature page", Weight::Secondary).draw(look, cx).on_click(cx.listener(move |this, _, _, cx| this.open_feature(&open, cx))),
                Role::Button,
                "Open feature page",
            )
            .into_any_element()];
            (title.clone(), head(this, look, "layers", token::SKETCH_GREEN, title.clone(), "feature".to_string(), String::new(), buttons, cx))
        }
        Subject::Part(part) => {
            let kind = kind_of(part.kind, part.within.as_deref(), this.top());
            (part.name.clone(), head(this, look, glyph_of(kind), hue_of(kind), part.name.clone(), kind_line(part), part.summary.clone(), buttons(this, part, look, cx), cx))
        }
    };
    // The project and a feature have the nine sections too, under a stand-in for the part the sections are drawn for.
    let stood = match &subject {
        Subject::Part(part) => (**part).clone(),
        Subject::Project => stand_in(String::new(), this.project_name()),
        Subject::Feature { slug, title } => stand_in(format!("{FEATURE_KEY}{slug}"), title),
    };
    let sections: Vec<AnyElement> = match this.answers.get(&stood.id).cloned() {
        None | Some(Read::Reading) => vec![skeleton(look)],
        Some(Read::Failed(why)) => vec![div().pt(look.length(token::SPACE_3)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(why)).into_any_element()],
        Some(Read::Ready(answers)) => sections(this, &stood, &answers, look, window, cx),
    };
    let frame = div().id("system-panel").flex_none().border_color(look.color(token::BORDER_SUBTLE)).bg(look.color(token::BG_SUNKEN)).p(look.length(token::SPACE_4));
    // Beside the graph the panel scrolls on its own; under it, it is as tall as it is and the block scrolls.
    let frame = if stacked { frame.w_full().border_t(look.length(token::BORDER_WIDTH)) } else { frame.w(look.length(token::W_DOCK_MIN)).h_full().min_h_0().overflow_y_scroll().border_l(look.length(token::BORDER_WIDTH)) };
    labelled(
        frame.child(header).children(sections),
        Role::Region,
        format!("About {name}"),
    )
    .into_any_element()
}

/// A part that is not one, to draw the sections of the whole project or of a feature: no folder, no pieces, no librarian.
fn stand_in(id: String, name: &str) -> SystemPart {
    SystemPart { id, name: name.to_string(), kind: SystemPartKind::Piece, summary: String::new(), within: None, manifest_id: None, paths: Vec::new(), zooms: false, origin: PartOrigin::Detected { from: String::new() } }
}

fn glyph_name(kind: SystemPartKind) -> &'static str {
    glyph_of(kind)
}

/// The buttons under the header: Look inside where the part has an inside, Open folder where it has code, and the librarian's three.
fn buttons(this: &SystemBlock, part: &SystemPart, look: &Look, cx: &mut Context<SystemBlock>) -> Vec<AnyElement> {
    let mut out = Vec::new();
    if part.zooms && this.part_selected().is_some_and(|selected| selected.id == part.id) {
        out.push(
            labelled(
                Btn::new("system-look-inside", "Look inside ↵", Weight::Primary).draw(look, cx).tooltip(tip("Zoom into it")).on_click(cx.listener(|this, _, window, cx| this.look_inside(window, cx))),
                Role::Button,
                "Look inside",
            )
            .into_any_element(),
        );
    }
    if !part.paths.is_empty() {
        let id = part.id.clone();
        out.push(
            labelled(Btn::new("system-open-folder", "Open folder", Weight::Secondary).draw(look, cx).tooltip(tip("Show its code in Finder")).on_click(cx.listener(move |this, _, _, cx| this.open_folder(&id, cx))), Role::Button, "Open folder")
                .into_any_element(),
        );
    }
    if let PartOrigin::Librarian { addition, .. } = &part.origin {
        let (keep, remove, rework) = (addition.clone(), addition.clone(), addition.clone());
        out.push(labelled(Btn::new("system-keep", "Keep", Weight::Primary).draw(look, cx).tooltip(tip("It joins the map as if it were found")).on_click(cx.listener(move |this, _, window, cx| this.answer_addition(&keep, AdditionAnswer::Keep, window, cx))), Role::Button, "Keep").into_any_element());
        out.push(
            labelled(
                Btn::new("system-rework", "Work with the librarian", Weight::Secondary).draw(look, cx).tooltip(tip("Tell it what is wrong and it redoes the part")).on_click(cx.listener(move |this, _, window, cx| this.open_rework_box(&rework, window, cx))),
                Role::Button,
                "Work with the librarian",
            )
            .into_any_element(),
        );
        out.push(labelled(Btn::new("system-remove", "Remove", Weight::Secondary).draw(look, cx).tooltip(tip("It leaves the map and is not added again")).on_click(cx.listener(move |this, _, window, cx| this.answer_addition(&remove, AdditionAnswer::Remove, window, cx))), Role::Button, "Remove").into_any_element());
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn head(this: &SystemBlock, look: &Look, mark: &'static str, hue: Token, name: String, kind: String, summary: String, buttons: Vec<AnyElement>, cx: &mut Context<SystemBlock>) -> AnyElement {
    let ink = look.color(hue);
    let why = this.part_selected().and_then(|part| match &part.origin {
        PartOrigin::Librarian { why, .. } => Some(why.clone()),
        _ => None,
    });
    let note = rework_box(this, look, cx);
    div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_2))
        .child(
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .child(
                    div()
                        .flex_none()
                        .size(look.length(token::SPACE_8) + look.length(token::SPACE_1))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(look.length(token::RADIUS_LG))
                        .bg(ink.opacity(super::graph::tint::TILE_LIT))
                        .child(glyph(mark, look.length(token::SPACE_4) + look.length(token::SPACE_1) / 2., ink)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(labelled(div().id("system-panel-name").truncate().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(name.clone())), Role::Heading, name))
                        .child(div().truncate().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(kind))),
                ),
        )
        .children((!summary.is_empty()).then(|| div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(summary))))
        .children(why.map(|why| div().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(format!("From the librarian: {why}")))))
        .children((!buttons.is_empty()).then(|| div().flex().flex_wrap().gap(look.length(token::SPACE_2)).children(buttons)))
        .children(note)
        .into_any_element()
}

/// The note to the librarian, under the buttons of a part it added.
fn rework_box(this: &SystemBlock, look: &Look, cx: &mut Context<SystemBlock>) -> Option<AnyElement> {
    let Some(Compose::Rework { addition, input, .. }) = &this.compose else { return None };
    let blank = input.read(cx).value().trim().is_empty();
    let addition = addition.clone();
    let send = labelled(
        Btn::new("system-rework-send", "Send note", Weight::Primary).disabled(blank).draw(look, cx).on_click(cx.listener(move |this, _, window, cx| {
            let note = this.words(cx).trim().to_string();
            if !note.is_empty() {
                this.answer_addition(&addition, AdditionAnswer::Rework { note }, window, cx);
            }
        })),
        Role::Button,
        "Send note",
    );
    Some(compose_box(look, input, send.into_any_element(), cx))
}

/// A bordered box for a sentence with its button under it. Esc puts it away and ⌘↩ presses the button.
fn compose_box(look: &Look, input: &gpui::Entity<gpui_component::input::TextareaState>, send: AnyElement, cx: &mut Context<SystemBlock>) -> AnyElement {
    div()
        .id("system-compose")
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_2))
        .p(look.length(token::SPACE_2))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
        .bg(look.color(token::BG_RAISED))
        .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
            let key = &event.keystroke;
            if key.key == "escape" {
                this.close_compose(window, cx);
                cx.stop_propagation();
            } else if key.key == "enter" && key.modifiers.platform {
                match &this.compose {
                    Some(Compose::Feature { .. }) => this.start_job(window, cx),
                    Some(Compose::Rework { addition, .. }) => {
                        let (addition, note) = (addition.clone(), this.words(cx).trim().to_string());
                        if !note.is_empty() {
                            this.answer_addition(&addition, AdditionAnswer::Rework { note }, window, cx);
                        }
                    }
                    None => {}
                }
                cx.stop_propagation();
            }
        }))
        .child(Textarea::new(input).appearance(false))
        .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(send))
        .into_any_element()
}

// ---- the sections ---------------------------------------------------------------------------------------

fn skeleton(look: &Look) -> AnyElement {
    labelled(
        div().id("system-panel-loading").flex().flex_col().gap(look.length(token::SPACE_4)).pt(look.length(token::SPACE_4)).children((0..4).map(|_| {
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(bone(look, look.length(token::SPACE_12) * 1.5, look.length(token::SPACE_3)))
                .child(bone(look, look.length(token::SPACE_12) * 3., look.length(token::SPACE_3)))
        })),
        Role::Image,
        "Loading",
    )
    .into_any_element()
}

fn section(look: &Look, at: usize, body: AnyElement) -> AnyElement {
    let (title, mark) = SECTIONS[at];
    div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_2))
        .mt(look.length(token::SPACE_3))
        .pt(look.length(token::SPACE_3))
        .border_t(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_SUBTLE))
        .child(
            labelled(
                div()
                    .id(SharedString::from(format!("system-section-{at}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .text_size(look.length(token::TEXT_2XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_SUBTLE))
                    .child(glyph(mark, look.length(token::SPACE_4) - look.length(token::SPACE_1) / 2., look.color(token::FG_SUBTLE)))
                    .child(SharedString::from(title.to_uppercase())),
                Role::Heading,
                title,
            ),
        )
        .child(body)
        .into_any_element()
}

fn none(look: &Look, words: &str) -> AnyElement {
    div().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(words.to_string())).into_any_element()
}

/// A line of a section: what it says, and the quiet words at its right.
fn line(look: &Look, left: AnyElement, right: Option<AnyElement>) -> gpui::Div {
    div().flex().items_baseline().gap(look.length(token::SPACE_2)).min_h(look.length(token::H_ROW)).text_size(look.length(token::TEXT_XS)).child(div().flex_1().min_w_0().child(left)).children(right)
}

fn quiet(look: &Look, words: impl Into<SharedString>) -> AnyElement {
    div().flex_none().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(words.into()).into_any_element()
}

fn id_text(look: &Look, words: impl Into<SharedString>) -> AnyElement {
    div().flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(words.into()).into_any_element()
}

/// A line he can press: hover, a ring when the keys are on it, and a name for VoiceOver.
fn pressable(look: &Look, key: String, said: String, row: gpui::Div, press: impl Fn(&mut SystemBlock, &mut Window, &mut Context<SystemBlock>) + 'static, cx: &mut Context<SystemBlock>) -> AnyElement {
    let ring = ring_colour(look);
    labelled(
        row.id(SharedString::from(key))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(gpui::transparent_black())
            .cursor_pointer()
            .hover(|style| style.bg(look.color(token::BG_HOVER)))
            .focusable()
            .tab_stop(true)
            .focus_visible(move |style| style.bg(look.color(token::BG_HOVER)).border_color(ring))
            .on_click(cx.listener(move |this, _, window, cx| press(this, window, cx))),
        Role::Button,
        said,
    )
    .into_any_element()
}

fn sections(this: &SystemBlock, part: &SystemPart, answers: &SystemPartAnswers, look: &Look, window: &Window, cx: &mut Context<SystemBlock>) -> Vec<AnyElement> {
    let hue = look.color(token::SKETCH_GREEN);
    let mut out = Vec::new();

    // Features
    let mut rows: Vec<AnyElement> = Vec::new();
    if answers.features.is_empty() {
        rows.push(none(look, NO_FEATURES));
    }
    for feature in &answers.features {
        let slug = feature.slug.clone();
        let tile = div().flex_none().size(look.length(token::SPACE_6)).flex().items_center().justify_center().rounded(look.length(token::RADIUS_MD)).bg(hue.opacity(super::graph::tint::TILE)).child(glyph("layers", look.length(token::SPACE_4) - look.length(token::SPACE_1), hue));
        let row = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::H_ROW_ACTION))
            .px(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_XS))
            .child(tile)
            .child(div().flex_1().min_w_0().truncate().child(SharedString::from(feature.title.clone())))
            .child(glyph("chevron-right", look.length(token::SPACE_4), look.color(token::FG_SUBTLE)));
        rows.push(pressable(look, format!("system-feature-row-{}", feature.slug), format!("{}, open the feature", feature.title), row, move |this, _, cx| this.open_feature(&slug, cx), cx));
    }
    let add = div()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_2))
        .h(look.length(token::H_ROW_ACTION))
        .px(look.length(token::SPACE_2))
        .text_size(look.length(token::TEXT_XS))
        .text_color(look.color(token::FG_MUTED))
        .child(
            div()
                .flex_none()
                .size(look.length(token::SPACE_6))
                .flex()
                .items_center()
                .justify_center()
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_dashed()
                .border_color(look.color(token::BORDER_STRONG))
                .child(glyph("plus", look.length(token::SPACE_4) - look.length(token::SPACE_1), look.color(token::FG_MUTED))),
        )
        .child("New feature…");
    rows.push(pressable(look, "system-new-feature".into(), "New feature".into(), add, |this, window, cx| this.open_feature_box(window, cx), cx));
    if let Some(Compose::Feature { input, .. }) = &this.compose {
        let blank = input.read(cx).value().trim().is_empty();
        let send = labelled(
            Btn::new("system-start-job", "Start a Job", Weight::Primary).disabled(blank).pending(this.starting).draw(look, cx).on_click(cx.listener(|this, _, window, cx| this.start_job(window, cx))),
            Role::Button,
            "Start a Job",
        );
        let place = match this.here() {
            Level::Project => this.project_name().to_string(),
            _ => part.name.clone(),
        };
        rows.push(
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .child(compose_box(look, input, send.into_any_element(), cx))
                .child(div().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(format!("Starts a Job in {}, beginning at {place}.", this.project_name()))))
                .into_any_element(),
        );
    }
    out.push(section(look, 0, div().flex().flex_col().children(rows).into_any_element()));

    // Code
    let code = if answers.code.is_empty() {
        none(look, code_line(part.kind))
    } else {
        let toggled = this.toggled.get(&part.id).cloned().unwrap_or_default();
        let rows = tree::rows(&answers.code, &toggled, &this.listings);
        let drawn = rows.into_iter().map(|row| tree_row(this, part, row, look, window, cx));
        labelled(div().id("system-code").flex().flex_col().children(drawn), Role::Tree, format!("Code of {}", part.name)).into_any_element()
    };
    out.push(section(look, 1, code));

    // Working on it
    let working = if answers.working.is_empty() {
        none(look, NO_JOB)
    } else {
        let live = look.color(token::ACCENT);
        div()
            .flex()
            .flex_col()
            .children(answers.working.iter().map(|job| {
                let id = job.session_id.as_str().to_string();
                let row = line(
                    look,
                    div()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .child(labelled(div().id(SharedString::from(format!("system-live-{id}"))).flex_none().size(look.length(token::SPACE_2)).rounded_full().bg(live).tooltip(tip("A Job is writing in it")), Role::Image, "A Job is writing in it"))
                        .child(div().truncate().child(SharedString::from(job.title.clone())))
                        .into_any_element(),
                    None,
                )
                .px(look.length(token::SPACE_2));
                let key = format!("system-job-{id}");
                pressable(look, key, format!("Open {}", job.title), row, move |this, _, cx| this.open_job(&id, cx), cx)
            }))
            .into_any_element()
    };
    out.push(section(look, 2, working));

    // Owner
    let owners = if answers.owners.is_empty() {
        none(look, &owner_line(part.kind, answers.owner_days))
    } else {
        div().flex().flex_col().children(answers.owners.iter().map(|owner| line(look, SharedString::from(owner.name.clone()).into_any_element(), Some(quiet(look, share_line(owner.share, answers.owner_days)))).into_any_element())).into_any_element()
    };
    out.push(section(look, 3, owners));

    // Looks like
    let looks = match &answers.looks {
        Some(path) => labelled(
            div()
                .id("system-shot")
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .overflow_hidden()
                .child(gpui::img(std::path::PathBuf::from(path)).w_full()),
            Role::Image,
            format!("{} as it looks", part.name),
        )
        .into_any_element(),
        None => none(look, looks_line(answers.has_screen)),
    };
    out.push(section(look, 4, looks));

    // Docs
    let docs = if answers.docs.is_empty() {
        none(look, NO_DOCS)
    } else {
        div()
            .flex()
            .flex_col()
            .children(answers.docs.iter().enumerate().map(|(at, path)| {
                let (open, part) = (path.clone(), part.clone());
                let row = line(look, div().truncate().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).child(SharedString::from(path.clone())).into_any_element(), None).px(look.length(token::SPACE_2));
                pressable(look, format!("system-doc-{at}"), format!("Open {path}"), row, move |this, _, cx| this.open_path(&open, Some(&part), cx), cx)
            }))
            .into_any_element()
    };
    out.push(section(look, 5, docs));

    // Monitor
    let monitor = if answers.monitor.is_empty() {
        none(look, NO_MONITOR)
    } else {
        div()
            .flex()
            .flex_col()
            .children(answers.monitor.iter().map(|watch| {
                let hue = match watch.kind.as_str() {
                    "server" | "run" => token::SKETCH_GREEN,
                    "health" => token::SKETCH_TEAL,
                    "check" => token::SKETCH_AMBER,
                    _ => token::SKETCH_GREY,
                };
                line(look, div().truncate().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).child(SharedString::from(watch.label.clone())).into_any_element(), Some(chip(look, hue, SharedString::from(watch.kind.clone())).into_any_element()))
                    .px(look.length(token::SPACE_2))
                    .into_any_element()
            }))
            .into_any_element()
    };
    out.push(section(look, 6, monitor));

    // Open issues
    let issues = if answers.issues.is_empty() {
        none(look, issues_line(answers.tracker))
    } else {
        div()
            .flex()
            .flex_col()
            .children(answers.issues.iter().map(|issue| {
                line(look, div().flex().gap(look.length(token::SPACE_2)).child(id_text(look, issue.id.clone())).child(div().min_w_0().truncate().child(SharedString::from(issue.title.clone()))).into_any_element(), None).px(look.length(token::SPACE_2)).into_any_element()
            }))
            .into_any_element()
    };
    out.push(section(look, 7, issues));

    // Open pull requests
    let pulls = if answers.pulls.is_empty() {
        none(look, NO_PULLS)
    } else {
        div()
            .flex()
            .flex_col()
            .children(answers.pulls.iter().map(|pull| {
                line(look, div().flex().gap(look.length(token::SPACE_2)).child(id_text(look, format!("#{}", pull.number))).child(div().min_w_0().truncate().child(SharedString::from(pull.title.clone()))).into_any_element(), Some(quiet(look, pull.author.clone())))
                    .px(look.length(token::SPACE_2))
                    .into_any_element()
            }))
            .into_any_element()
    };
    out.push(section(look, 8, pulls));
    out
}

/// One row of the code tree: a folder that opens and shuts, or a file, with the room under an open folder that is being read.
fn tree_row(this: &SystemBlock, part: &SystemPart, row: Row, look: &Look, _window: &Window, cx: &mut Context<SystemBlock>) -> AnyElement {
    let indent = look.length(token::SPACE_3) * row.depth as f32;
    let ink = if row.dir { look.color(token::FG_DEFAULT) } else { look.color(token::FG_MUTED) };
    let base = div()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_1))
        .h(look.length(token::H_ROW))
        .pl(indent + look.length(token::SPACE_1))
        .pr(look.length(token::SPACE_1))
        .font_family(mono(look))
        .text_size(look.length(token::TEXT_2XS))
        .text_color(ink)
        .whitespace_nowrap();
    let _ = this;
    if !row.dir {
        return labelled(
            base.id(SharedString::from(format!("system-file-{}", row.path)))
                .child(div().flex_none().size(look.length(token::SPACE_4)))
                .child(glyph("file", look.length(token::SPACE_4) - look.length(token::SPACE_1), look.color(token::FG_SUBTLE)))
                .child(SharedString::from(row.words.clone())),
            Role::TreeItem,
            row.words.clone(),
        )
        .into_any_element();
    }
    let ring = ring_colour(look);
    let (part_id, pressed, left, right) = (part.id.clone(), row.clone(), row.clone(), row.clone());
    let (part_left, part_right) = (part.id.clone(), part.id.clone());
    let shut = !row.open;
    let head = labelled(
        base.id(SharedString::from(format!("system-folder-{}", row.path)))
            .rounded(look.length(token::RADIUS_SM))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(gpui::transparent_black())
            .cursor_pointer()
            .hover(|style| style.bg(look.color(token::BG_HOVER)))
            .focusable()
            .tab_stop(true)
            .focus_visible(move |style| style.bg(look.color(token::BG_HOVER)).border_color(ring))
            .aria_expanded(row.open)
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| match event.keystroke.key.as_str() {
                "right" if !right.open => {
                    this.toggle_folder(&part_right, &right, cx);
                    cx.stop_propagation();
                }
                "left" if left.open => {
                    this.toggle_folder(&part_left, &left, cx);
                    cx.stop_propagation();
                }
                _ => {}
            }))
            .on_click(cx.listener(move |this, _, _, cx| this.toggle_folder(&part_id, &pressed, cx)))
            .child(glyph(if shut { "chevron-right" } else { "chevron-down" }, look.length(token::SPACE_4), look.color(token::FG_SUBTLE)))
            .child(glyph(if shut { "folder" } else { "folder-open" }, look.length(token::SPACE_4) - look.length(token::SPACE_1), look.color(token::SKETCH_AMBER)))
            .child(SharedString::from(format!("{}/", row.words))),
        Role::TreeItem,
        format!("{} folder", row.words),
    );
    let under = if row.reading {
        Some(div().pl(indent + look.length(token::SPACE_8)).py(look.length(token::SPACE_1)).child(bone(look, look.length(token::SPACE_12) * 1.5, look.length(token::SPACE_3))).into_any_element())
    } else {
        row.failed.clone().map(|why| div().pl(indent + look.length(token::SPACE_8)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(why)).into_any_element())
    };
    div().flex().flex_col().child(head).children(under).into_any_element()
}
