//! The card in the thread for a page and a draft, in the Band look the owner approved on 14 Nov 2026: a header
//! band filled with where the artifact stands, its first screen in a sunken well cropped at 240, and the acts in a
//! footer. The sketch card (`sketch_card.rs`) is the reference for the look; what the acts do is each card's own,
//! passed in as one handler so a page and a draft share the frame and the keys and differ in what they send.

use std::rc::Rc;

use gpui::{div, prelude::*, px, AnyElement, Context, FontWeight, Role, SharedString, Window};
use ipc::{AnswerDocument, ArtifactBy, ArtifactFor, ArtifactForm, ArtifactRef, ArtifactState, DismissWaiting, DocumentHow, SessionId, ThreadDocument};

use super::acts::{refusal, Answer, Mode};
use super::artifacts_tab::{Picture, PictureKey};
use super::cards::{quote, settled_of, Said};
use super::rows::{mono, tip};
use super::sketch_card::{act, link_chip, mixed, Kind};
use super::{JobView, Page};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look, Token};

/// Where an artifact stands: the colour of its band and the word on its chip.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Stand {
    Asking,
    Approved,
    SentBack,
    Reading,
    Withdrawn,
    Dismissed,
}

impl Stand {
    pub(super) fn of(state: ArtifactState) -> Self {
        match state {
            ArtifactState::Asking => Stand::Asking,
            ArtifactState::Approved => Stand::Approved,
            ArtifactState::SentBack => Stand::SentBack,
            ArtifactState::Reading => Stand::Reading,
            ArtifactState::Withdrawn => Stand::Withdrawn,
            ArtifactState::Dismissed => Stand::Dismissed,
        }
    }

    pub(super) fn word(self) -> &'static str {
        match self {
            Stand::Asking => "Asking",
            Stand::Approved => "Approved",
            Stand::SentBack => "Sent back",
            Stand::Reading => "Reading",
            Stand::Withdrawn => "Withdrawn",
            Stand::Dismissed => "Dismissed",
        }
    }

    pub(super) fn ink(self) -> Token {
        match self {
            Stand::Asking => token::STATUS_AWAITING_REVIEW,
            Stand::Approved => token::STATUS_COMPLETED_SUCCESS,
            Stand::SentBack => token::STATUS_REJECTED,
            Stand::Reading => token::STATUS_NOT_STARTED,
            Stand::Withdrawn | Stand::Dismissed => token::STATUS_KILLED,
        }
    }

    fn tip(self) -> &'static str {
        match self {
            Stand::Asking => "Waiting on your answer",
            Stand::Approved => "You approved it",
            Stand::SentBack => "You sent it back with changes",
            Stand::Reading => "Shown to you, with no answer asked",
            Stand::Withdrawn => "The Job took it back",
            Stand::Dismissed => "You passed on it",
        }
    }
}

/// The kind's glyph and its colour: a sketch is a step's, a draft a setup's, a page a gate's.
pub(super) fn kind_look(form: ArtifactForm) -> (&'static str, Token, &'static str) {
    match form {
        ArtifactForm::Sketch => ("layers", token::KIND_STEP, "Sketch"),
        ArtifactForm::Draft => ("file-text", token::KIND_SETUP, "Draft"),
        ArtifactForm::Page => ("scroll-text", token::KIND_GATE, "Page"),
    }
}

/// The 26 px tile that carries the kind's glyph.
pub(super) fn tile(look: &Look, form: ArtifactForm) -> gpui::Div {
    let (glyph, ink, _) = kind_look(form);
    let ink = look.color(ink);
    div()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(look.length(token::SPACE_6) + look.length(token::BORDER_WIDTH) * 2.)
        .rounded(look.length(token::RADIUS_MD))
        .bg(ink.opacity(0.18))
        .child(gpui::svg().path(icon_path(glyph)).flex_none().size(look.length(token::TEXT_BASE)).text_color(ink))
}

/// The state chip: a dot and its word, in the state's colour. `words`, where Fleet gave any, is its tooltip.
pub(super) fn state_chip(look: &Look, id: String, stand: Stand, words: Option<&str>, named: Option<&str>) -> AnyElement {
    let ink = look.color(stand.ink());
    let said: SharedString = words.map(|words| words.to_string()).unwrap_or_else(|| stand.tip().to_string()).into();
    labelled(
        div()
            .id(SharedString::from(id))
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .px(look.length(token::SPACE_2))
            .rounded_full()
            .bg(ink.opacity(0.16))
            .text_size(look.length(token::TEXT_2XS))
            .line_height(look.length(token::LEADING_2XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
            .text_color(ink)
            .tooltip(tip(said))
            .child(gpui::svg().path(icon_path("circle-dot")).flex_none().size(look.length(token::TEXT_2XS)).text_color(ink))
            .child(stand.word()),
        Role::Status,
        named.unwrap_or(stand.word()).to_string(),
    )
    .into_any_element()
}

/// The revision chip: `r3` in mono, who made it and when in its tooltip.
pub(super) fn revision_chip(look: &Look, id: String, number: u32, said: String) -> AnyElement {
    labelled(
        div()
            .id(SharedString::from(id))
            .flex_none()
            .px(look.length(token::SPACE_1) + look.length(token::BORDER_WIDTH) * 2.)
            .rounded(look.length(token::RADIUS_SM))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_2XS))
            .line_height(look.length(token::LEADING_2XS))
            .text_color(look.color(token::FG_MUTED))
            .tooltip(tip(said))
            .child(format!("r{number}")),
        Role::Label,
        format!("Revision {number}"),
    )
    .into_any_element()
}

/// The pill that says the newest revision is his, in his colour, with the revision it names in its tooltip.
pub(super) fn yours_pill(look: &Look, id: String, said: String) -> AnyElement {
    let ink = look.color(token::YOU);
    labelled(
        div()
            .id(SharedString::from(id))
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .px(look.length(token::SPACE_2))
            .rounded_full()
            .bg(ink.opacity(0.16))
            .text_size(look.length(token::TEXT_2XS))
            .line_height(look.length(token::LEADING_2XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
            .text_color(ink)
            .tooltip(tip(said))
            .child(gpui::svg().path(icon_path("pencil")).flex_none().size(look.length(token::TEXT_2XS)).text_color(ink))
            .child("Your edit"),
        Role::Label,
        "Your edit",
    )
    .into_any_element()
}

/// What the header draws.
pub(super) struct Head<'a> {
    pub key: &'a str,
    pub title: &'a str,
    pub form: ArtifactForm,
    pub stand: Stand,
    pub words: Option<&'a str>,
    /// The newest revision's number and what its tooltip says.
    pub revision: Option<(u32, String)>,
    /// The newest revision is his: what the pill's tooltip says.
    pub yours: Option<String>,
    pub for_line: Option<&'a ArtifactFor>,
    /// What the chip is named for VoiceOver where its word is not the whole of it: `Approved with your fixes`.
    pub named: Option<&'a str>,
}

/// The header band: the kind's tile, the title, the revision and where it stands, and under them what it is for.
pub(super) fn band(look: &Look, head: Head) -> AnyElement {
    let ink = head.stand.ink();
    let top = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(look.length(token::SPACE_3))
        .child(tile(look, head.form))
        .child(
            div()
                .flex_grow(1.)
                .min_w_0()
                .text_size(look.length(token::TEXT_BASE))
                .line_height(look.length(token::LEADING_BASE))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(look.color(token::FG_DEFAULT))
                .child(SharedString::from(head.title.to_string())),
        )
        .children(head.revision.map(|(number, said)| revision_chip(look, format!("revision-tag:{}", head.key), number, said)))
        .children(head.yours.map(|said| yours_pill(look, format!("yours:{}", head.key), said)))
        .child(state_chip(look, format!("state:{}", head.key), head.stand, head.words, head.named));
    let line = head.for_line.filter(|line| line.text.as_deref().is_some_and(|text| !text.trim().is_empty()) || !line.links.is_empty());
    let under = line.map(|line| {
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_MUTED))
            .child(
                div()
                    .flex()
                    .gap(look.length(token::SPACE_1))
                    .min_w_0()
                    .max_w_full()
                    .child(div().flex_none().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child("For:"))
                    .children(line.text.clone().map(|text| div().flex_1().min_w_0().child(SharedString::from(text)))),
            )
            .children(line.links.iter().map(|link| link_chip(look, link)))
    });
    div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_1) + look.length(token::BORDER_WIDTH) * 2.)
        .px(look.length(token::SPACE_4))
        .py(look.length(token::SPACE_3))
        .bg(mixed(look, ink, 0.18))
        .border_b(look.length(token::BORDER_WIDTH))
        .border_color(look.color(ink).opacity(0.4))
        .child(top)
        .children(under)
        .into_any_element()
}

/// The card's frame: rounded at radius-lg times one and a half, a raised fill, a hairline and a shadow. A narrow
/// thread lets it reach a little past the words, as the sketch card does, so the acts fit one row.
pub(super) fn frame(look: &Look, bleed: bool) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .when(bleed, |body| body.mx(-(look.length(token::SPACE_4) + look.length(token::SPACE_1))))
        .overflow_hidden()
        .rounded(look.length(token::RADIUS_LG) * 1.5)
        .bg(look.color(token::BG_RAISED))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_DEFAULT))
        .shadow(look.shadows(token::SHADOW_CARD))
}

/// The most the first screen shows, in px: five of the card's 48 px steps.
pub(super) fn most(look: &Look) -> gpui::Pixels {
    look.length(token::SPACE_12) * 5.
}

/// The first screen's well: sunken, a hairline edge, cut at 240 with a fade at its foot where it runs on.
pub(super) fn well(look: &Look, cut: bool, inside: AnyElement) -> AnyElement {
    let sunken = look.color(token::BG_SUNKEN);
    div()
        .px(look.length(token::SPACE_4))
        .py(look.length(token::SPACE_3))
        .child(
            div()
                .relative()
                .overflow_hidden()
                .max_h(most(look))
                .bg(sunken)
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .px(look.length(token::SPACE_4))
                .py(look.length(token::SPACE_3))
                .child(inside)
                .when(cut, |well| {
                    well.child(
                        div()
                            .absolute()
                            .left_0()
                            .right_0()
                            .bottom_0()
                            .h(look.length(token::SPACE_12))
                            .bg(gpui::linear_gradient(180., gpui::linear_color_stop(sunken.opacity(0.), 0.), gpui::linear_color_stop(sunken, 1.))),
                    )
                })
                .into_any_element(),
        )
        .into_any_element()
}

/// The shapes of a screen that has not come, as the page's own tab draws them.
fn shapes(look: &Look) -> AnyElement {
    use gpui::AnimationExt as _;
    let bar = |width: f32| div().h(look.length(token::SPACE_3)).w(gpui::relative(width)).rounded(look.length(token::RADIUS_SM)).bg(look.color(token::FG_DEFAULT).opacity(0.07));
    labelled(
        div().id("page-card-loading").w_full().child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .child(bar(0.4))
                .child(bar(1.0))
                .child(bar(0.85))
                .child(bar(0.6))
                .with_animation(
                    "page-card-loading-pulse",
                    gpui::Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(gpui::pulsating_between(0.5, 1.)),
                    |shapes, at| shapes.opacity(at),
                ),
        ),
        Role::Status,
        "Loading",
    )
    .into_any_element()
}

/// What a card's acts are, which the handler of each card answers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Act {
    Approve,
    Ask,
    Send,
    Cancel,
    Dismiss,
    Archive,
    Open,
    Edit,
}

/// The act a key names on a card with nothing open in it. A and C answer and E edits only while it asks; D dismisses
/// while it asks and archives where it does not; Enter opens.
pub(super) fn key_act(key: &str, asking: bool, editable: bool, openable: bool) -> Option<Act> {
    match key {
        "a" if asking => Some(Act::Approve),
        "c" if asking => Some(Act::Ask),
        "e" if asking && editable => Some(Act::Edit),
        "d" => Some(if asking { Act::Dismiss } else { Act::Archive }),
        "enter" if openable => Some(Act::Open),
        _ => None,
    }
}

/// What a card does for each act.
pub(super) type Do = Rc<dyn Fn(&mut JobView, Act, &mut Window, &mut Context<JobView>)>;

/// What the footer needs to draw and to answer.
pub(super) struct Foot {
    pub ix: usize,
    pub key: String,
    /// Put to him and not answered: Approve and Ask for changes show, and Dismiss where Archive would.
    pub asking: bool,
    /// Archive is offered where it does not ask, unless it is folded away already.
    pub archivable: bool,
    pub openable: bool,
    pub editable: bool,
    /// Whether this footer's note box takes ⌘↵ itself; a card with a commit of its own says no.
    pub commit: bool,
    /// The route Fleet carries the answer on, for the row to dim where it does not run.
    pub route: &'static str,
    pub run: Do,
}

/// The footer's padding and top hairline, around whatever the card puts in it.
pub(super) fn foot_box(look: &Look) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_2))
        .px(look.length(token::SPACE_4))
        .py(look.length(token::SPACE_3))
        .border_t(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_SUBTLE))
}

fn ghost(look: &Look, foot: &Foot, id: &str, name: &'static str, cap: &'static str, said: &'static str, act_is: Act, cx: &Context<JobView>) -> AnyElement {
    let (run, key) = (foot.run.clone(), format!("{id}:{}", foot.key));
    labelled(
        act(look, key, name, Kind::Ghost, cap, true).tooltip(tip(said)).on_click(cx.listener(move |this, _, window, cx| run(this, act_is, window, cx))),
        Role::Button,
        name,
    )
    .into_any_element()
}

impl JobView {
    /// The footer: Approve and Ask for changes while it asks, then Dismiss where it asks and Archive where it does not,
    /// and Open. Ask for changes opens the note box in their place. `extras` stand in the right group before Dismiss.
    pub(super) fn band_footer(&self, foot: &Foot, extras: Vec<AnyElement>, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = foot.key.as_str();
        let mut column = div().flex().flex_col().gap(look.length(token::SPACE_2));
        if (!foot.asking || !page.cards.modes.contains_key(key)) && self.holds_kept(key) {
            page.cards.kept_seen.borrow_mut().push((key.to_string(), foot.ix, foot.asking));
        }
        if let Some(why) = page.cards.said.get(key) {
            column = column.child(refusal(look, key, why.clone()));
        }
        if foot.asking && page.cards.modes.get(key) == Some(&Mode::Note) {
            return column.child(self.band_note(foot, page, look, cx)).into_any_element();
        }
        let group = || div().flex().flex_none().items_center().gap(look.length(token::SPACE_2));
        let row = div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_2));
        let row = if foot.asking {
            let (approve, ask) = (foot.run.clone(), foot.run.clone());
            row.child(
                group()
                    .child(labelled(
                        act(look, format!("approve:{key}"), "Approve", Kind::Approve, "A", true)
                            .tooltip(tip("Approve it as it stands"))
                            .on_click(cx.listener(move |this, _, window, cx| approve(this, Act::Approve, window, cx))),
                        Role::Button,
                        "Approve",
                    ))
                    .child(labelled(
                        act(look, format!("ask:{key}"), "Ask for changes", Kind::Secondary, "C", true)
                            .tooltip(tip("Send it back with a note"))
                            .on_click(cx.listener(move |this, _, window, cx| ask(this, Act::Ask, window, cx))),
                        Role::Button,
                        "Ask for changes",
                    )),
            )
        } else {
            row
        };
        let edit = (foot.asking && foot.editable).then(|| ghost(look, foot, "edit", "Edit", "E", "Change its words before you answer", Act::Edit, cx));
        let away = if foot.asking {
            Some(ghost(look, foot, "dismiss", "Dismiss", "D", "Put it away with no answer", Act::Dismiss, cx))
        } else if foot.archivable {
            Some(ghost(look, foot, "archive", "Archive", "D", "Fold it away from your list", Act::Archive, cx))
        } else {
            None
        };
        let open = foot.openable.then(|| ghost(look, foot, "open", "Open", "\u{21b5}", "Read it on its own", Act::Open, cx));
        let row = row.child(group().flex_shrink_1().flex_wrap().min_w_0().ml_auto().justify_end().children(extras).children(edit).children(away).children(open));
        column.child(super::acts::gated(row.id(SharedString::from(format!("answers:{key}"))), self.off_shown_to(foot.route, cx))).into_any_element()
    }

    /// What should change: the note, with Cancel and Send under it.
    fn band_note(&self, foot: &Foot, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = foot.key.as_str();
        let blank = self.typed_in(key, Mode::Note, cx).trim().is_empty();
        if foot.commit {
            let run = foot.run.clone();
            // `⌘Enter` in the box sends it back, as Send does.
            page.keys.commit(key, Some(Rc::new(move |this: &mut JobView, window: &mut Window, cx: &mut Context<JobView>| run(this, Act::Send, window, cx))));
        }
        let reveal = page.cards.reveal.get(key).cloned().map(|flag| {
            let (me, ix) = (cx.entity().downgrade(), foot.ix);
            gpui::canvas(
                move |_, _, cx| {
                    if flag.replace(false) {
                        let me = me.clone();
                        cx.defer(move |cx| {
                            let _ = me.update(cx, |this, cx| this.reveal_card(ix, cx));
                        });
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full()
        });
        let (esc, cancel, send) = (foot.run.clone(), foot.run.clone(), foot.run.clone());
        div()
            .relative()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .children(reveal)
            .capture_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" && !event.keystroke.modifiers.modified() {
                    cx.stop_propagation();
                    esc(this, Act::Cancel, window, cx);
                }
            }))
            .child(
                div()
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .text_color(look.color(token::FG_MUTED))
                    .child("What should change?"),
            )
            .children(self.box_of(key, Mode::Note, "What should change?", look, cx))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(look.length(token::SPACE_2))
                    .child(labelled(
                        act(look, format!("cancel:{key}"), "Cancel", Kind::Ghost, "Esc", true)
                            .tooltip(tip("Keep it as it is"))
                            .on_click(cx.listener(move |this, _, window, cx| cancel(this, Act::Cancel, window, cx))),
                        Role::Button,
                        "Cancel",
                    ))
                    .child(labelled(
                        act(look, format!("send-back:{key}"), "Send", Kind::Primary, "\u{2318}\u{21b5}", !blank).when(!blank, |button| {
                            button.tooltip(tip("Send it back with this note")).on_click(cx.listener(move |this, _, window, cx| send(this, Act::Send, window, cx)))
                        }),
                        Role::Button,
                        "Send",
                    )),
            )
            .into_any_element()
    }

    /// The card is whole in the thread again, now that it has its new height.
    fn reveal_card(&mut self, ix: usize, cx: &mut Context<Self>) {
        if let Some(page) = self.shown.as_ref().and_then(|id| self.pages.get(id)) {
            if ix != super::acts::SHEET {
                page.list.scroll_to_reveal_item(page.entry(ix));
            }
        }
        cx.notify();
    }

    /// Ask for changes: the note box opens under the card, and is revealed again once it is drawn at its height.
    pub(super) fn ask_for_changes(&mut self, ix: usize, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.open_box(ix, key, Mode::Note, "", window, cx);
        if let Some(cards) = self.cards_mut() {
            cards.reveal.insert(key.to_string(), Rc::new(std::cell::Cell::new(true)));
        }
    }

    /// The note shuts and the keys go back to the card.
    pub(super) fn shut_note(&mut self, ix: usize, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.close_box(ix, key, cx);
        let spot = self.shown.as_ref().and_then(|id| self.pages.get(id)).and_then(|page| page.keys.spot_of(&format!("card:{key}")));
        if let Some(spot) = spot {
            window.focus(&spot.focus, cx);
        }
    }

    /// Archive folds the artifact away in his Artifacts tab with no verdict (`archive_artifact`).
    pub(super) fn archive_card(&mut self, ix: usize, key: &str, artifact: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        self.archived_here(&job, artifact, true);
        let body = ipc::encode(&ArtifactRef { artifact: artifact.to_string(), session_id: Some(SessionId::carried(job.to_string())) }).map_err(|why| format!("{why:?}"));
        self.post_card(job, ix, key.to_string(), "/sessions/artifacts/archive", body, window, cx);
        cx.notify();
    }

    /// Whether the card sits in a narrow thread.
    pub(super) fn band_narrow(&self, look: &Look) -> bool {
        self.pane.is_some_and(|width| width < look.length(token::SPACE_12) * 10.)
    }

    /// How wide a page's first screen draws, in whole points: the thread's width less the card's own insets and the well's.
    fn first_screen_width(&self, look: &Look) -> u32 {
        let card = self.pane.unwrap_or(px(720.)) - look.length(token::SPACE_4) * 2. + if self.band_narrow(look) { (look.length(token::SPACE_4) + look.length(token::SPACE_1)) * 2. } else { px(0.) };
        let inset = look.length(token::SPACE_4) * 4. + look.length(token::BORDER_WIDTH) * 4.;
        f32::from(card - inset).max(120.).round() as u32
    }

    /// A page in the thread, one card per page with its newest revision (`ThreadDocument`).
    pub(super) fn document_card(&self, ix: usize, card: &ThreadDocument, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = card.id.clone();
        let job = self.shown.clone().unwrap_or_default();
        let settled = settled_of(card.settled.as_ref(), page.cards.answers.get(&key));
        let kept = self.artifact_of(&job, &card.page);
        let archived = kept.is_some_and(|one| one.archived.is_some());
        let dismissed = page.cards.answers.get(&key) == Some(&Answer::Withdrawn) || kept.is_some_and(|one| matches!(one.state, ArtifactState::Dismissed | ArtifactState::Withdrawn) && settled.is_none());
        let stand = match (&settled, dismissed) {
            (Some((Said::Approved, _)), _) => Stand::Approved,
            (Some((Said::SentBack, _)), _) => Stand::SentBack,
            (None, true) => kept.filter(|one| one.state == ArtifactState::Withdrawn).map_or(Stand::Dismissed, |_| Stand::Withdrawn),
            (None, false) if card.asked => Stand::Asking,
            (None, false) => Stand::Reading,
        };
        let asking = stand == Stand::Asking && !archived;
        let here = ix == super::acts::SHEET;
        let revision = Some((card.revision, kept.and_then(|one| one.revisions.iter().find(|r| r.number == card.revision)).map_or_else(|| format!("Revision {}", card.revision), |r| revision_said(r.number, r.by, r.at.as_str()))));
        let yours = kept.and_then(|one| one.revisions.last()).filter(|last| last.by == ArtifactBy::Owner && last.number == card.revision).map(|last| format!("r{} is your edit", last.number));
        let head = band(
            look,
            Head { key: &key, title: &card.title, form: ArtifactForm::Page, stand, words: kept.and_then(|one| one.words.as_deref()), revision, yours, for_line: card.for_line.as_ref(), named: None },
        );
        // The first screen: the page's own picture where one has come, shapes while it comes, nothing where it was refused.
        let width = self.first_screen_width(look);
        let want = PictureKey { job: job.to_string(), page: card.page.clone(), revision: card.revision, width, dark: look.tone() == crate::look::Tone::Dark };
        let screen = (!here).then(|| match self.picture_of(&want) {
            Some(Picture::Ready(image, height)) => {
                let (image, height) = (image.clone(), *height);
                Some(well(look, height > f32::from(most(look)) - f32::from(look.length(token::SPACE_3)) * 2., gpui::img(image).w(px(width as f32)).h(px(height)).into_any_element()))
            }
            Some(Picture::Failed) => None,
            _ => Some(self.asking_picture(&want, well(look, true, shapes(look)), cx)),
        });
        let (name, page_name, id) = (card.title.clone(), card.page.clone(), key.clone());
        let run: Do = {
            let (id, page_name) = (id.clone(), page_name.clone());
            let revision = card.revision;
            Rc::new(move |this, act, window, cx| {
                let session = SessionId::carried(this.shown.clone().unwrap_or_default().to_string());
                let answer = |how: DocumentHow, words: Option<String>| {
                    ipc::encode(&AnswerDocument { session_id: session.clone(), document: format!("document:{page_name}"), revision: revision.to_string(), how, words }).map_err(|why| format!("{why:?}"))
                };
                match act {
                    Act::Approve => this.settle(ix, id.clone(), Answer::Approved, "/sessions/document/answer", answer(DocumentHow::Approved, None), window, cx),
                    Act::Ask => this.ask_for_changes(ix, &id, window, cx),
                    Act::Send => {
                        let note = this.typed_in(&id, Mode::Note, cx).trim().to_string();
                        if !note.is_empty() {
                            this.settle(ix, id.clone(), Answer::SentBack(note.clone()), "/sessions/document/answer", answer(DocumentHow::SentBack, Some(note)), window, cx);
                        }
                    }
                    Act::Cancel => this.shut_note(ix, &id, window, cx),
                    Act::Dismiss => {
                        let body = ipc::encode(&DismissWaiting { session_id: session.clone(), item_id: format!("document:{page_name}"), reason: None, misread: false }).map_err(|why| format!("{why:?}"));
                        this.settle(ix, id.clone(), Answer::Withdrawn, "/sessions/waiting/dismiss", body, window, cx);
                    }
                    Act::Archive => this.archive_card(ix, &id, &page_name, window, cx),
                    Act::Open => this.open_document_tab(&page_name, window, cx),
                    Act::Edit => {}
                }
            })
        };
        let foot = Foot { ix, key: key.clone(), asking, archivable: !archived, openable: !here, editable: false, commit: true, route: "/sessions/document/answer", run: run.clone() };
        let words = settled.as_ref().and_then(|(said, words)| (*said == Said::SentBack).then_some(words.as_deref()).flatten());
        // The band and the screen are one press that opens the page.
        let open = run.clone();
        let press = div()
            .id(SharedString::from(format!("page-press:{key}")))
            .flex()
            .flex_col()
            .when(!here, |press| press.cursor_pointer().on_click(cx.listener(move |this, _, window, cx| open(this, Act::Open, window, cx))))
            .child(head)
            .children(screen.flatten());
        let spot = page.keys.spot(&format!("card:{key}"), cx);
        let keys = run;
        let frame = frame(look, self.band_narrow(look))
            .id(SharedString::from(format!("document:{key}")))
            .track_focus(&spot.focus)
            .in_focus(|style| style.border_color(look.color(token::ACCENT)))
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.modifiers.modified() {
                    return;
                }
                let noting = this.shown.as_ref().and_then(|job| this.pages.get(job)).is_some_and(|page| page.cards.modes.contains_key(&id));
                if noting {
                    return;
                }
                if let Some(act) = key_act(event.keystroke.key.as_str(), asking, false, !here) {
                    cx.stop_propagation();
                    keys(this, act, window, cx);
                }
            }))
            .child(press)
            .children(words.map(|words| div().px(look.length(token::SPACE_4)).pb(look.length(token::SPACE_3)).child(quote(look, words))))
            .child(foot_box(look).child(self.band_footer(&foot, Vec::new(), page, look, cx)));
        labelled(frame, Role::Article, format!("Document: {name}, Revision {}", card.revision)).into_any_element()
    }
}

/// Who kept a revision and when, for its chip's tooltip.
pub(super) fn revision_said(number: u32, by: ArtifactBy, at: &str) -> String {
    let who = match by {
        ArtifactBy::Agent => "the Job",
        ArtifactBy::Owner => "you",
    };
    let when = super::acts::at_of(&ipc::Instant::carried(at)).map_or_else(String::new, |at| format!(", {}", at.format("%-d %b %H:%M")));
    format!("r{number} by {who}{when}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_answers_only_while_the_card_asks_and_d_archives_where_it_does_not() {
        assert_eq!(key_act("a", true, false, true), Some(Act::Approve));
        assert_eq!(key_act("c", true, false, true), Some(Act::Ask));
        assert_eq!(key_act("d", true, false, true), Some(Act::Dismiss));
        assert_eq!(key_act("a", false, false, true), None);
        assert_eq!(key_act("c", false, false, true), None);
        assert_eq!(key_act("d", false, false, true), Some(Act::Archive));
        assert_eq!(key_act("enter", false, false, true), Some(Act::Open));
        assert_eq!(key_act("enter", true, false, false), None, "a card in its own tab has nothing to open");
    }

    #[test]
    fn edit_has_its_key_only_where_the_card_can_be_edited_and_asks() {
        assert_eq!(key_act("e", true, true, true), Some(Act::Edit));
        assert_eq!(key_act("e", true, false, true), None);
        assert_eq!(key_act("e", false, true, true), None);
    }

    #[test]
    fn each_state_has_its_own_colour_and_word() {
        let all = [Stand::Asking, Stand::Approved, Stand::SentBack, Stand::Reading, Stand::Withdrawn, Stand::Dismissed];
        let words: std::collections::HashSet<_> = all.iter().map(|one| one.word()).collect();
        assert_eq!(words.len(), all.len());
        assert_eq!(Stand::of(ArtifactState::SentBack), Stand::SentBack);
        assert_eq!(Stand::Withdrawn.ink(), Stand::Dismissed.ink(), "withdrawn and dismissed share the killed colour");
    }

    #[test]
    fn a_revision_names_who_kept_it() {
        assert!(revision_said(3, ArtifactBy::Agent, "2026-11-14T10:42:00Z").starts_with("r3 by the Job"));
        assert!(revision_said(2, ArtifactBy::Owner, "2026-11-14T09:15:00Z").starts_with("r2 by you"));
    }
}
