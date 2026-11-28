//! What the Ready to try tab and the thread's compact card are made of: a build's standing, its parts and its answer, and the
//! pieces the tab's detail draws it with. The Pictures first take the owner picked on 15 Nov 2026 stands in the detail: a green
//! head band, the title, what he approved beside what was built, his words, where to look with the steps, and a foot of four
//! answers. The thread draws no such card of its own: it holds one compact card per Job (`ready_compact`) that opens a build
//! whole in place.
//!
//! **A build is the build note of its branch at its tip, seen from the Job's thread**, so Fleet carries its state
//! (`ThreadReadyToTry`) and the thread moves when he answers. A press answers at once, as every card does (`acts`), and a
//! refusal takes the answer back and says Fleet's sentence on the build's row. One he has answered, or the Job has
//! withdrawn or replaced, is one line in the thread that opens the tab on a press.

use std::rc::Rc;

use chrono::{DateTime, Local};
use gpui::{canvas, div, prelude::*, px, size, AnyElement, App, Context, FontWeight, Role, SharedString, Window};
use ipc::{AnswerReadyToTry, ArtifactForm, Instant, NoteVerdict, PreviewOffers, PreviewState, ReadyAnswer, ReadyApproved, ReadyState, SentFile, SessionId, ThreadItem, ThreadReadyToTry};

use super::acts::{kbd, refusal, Answer, Mode};
use super::artifacts_tab::{Picture, PictureKey};
use super::band_card::{foot_box, kind_look, shapes, tile, well, Act, Do, Foot};
use super::cards::{quote, shown_scene};
use super::rows::{glyph, mono, tip, Size};
use super::sketch_card::{act, mixed, Kind};
use super::sketch_read::newest_card;
use super::{JobView, Page};
use crate::build_notes::{step_lines, StepLine};
use crate::labels::labelled;
use crate::look::{token, Look, Token};
use crate::scene::{Clock, Fit, Lens};

/// Whether an id names a Ready to try card (`try:<branch>@<tip>`) or a Job's one item to try (`try:<session_id>`), and not an artifact.
pub(super) fn is_card(id: &str) -> bool {
    id.starts_with("try:")
}

/// The card an id names in a thread, with where it stands. A card's own id (`try:<branch>@<tip>`) names that card; the Job's
/// one Needs you row (`try:<session>`, no `@`) names its newest open card, else its newest card.
pub(super) fn card_named(items: &[ThreadItem], id: &str) -> Option<(usize, String)> {
    let cards = || items.iter().enumerate().rev().filter_map(|(at, item)| match item {
        ThreadItem::ReadyToTry(card) => Some((at, card)),
        _ => None,
    });
    let found = if id.contains('@') {
        cards().find(|(_, card)| card.id == id)
    } else {
        cards().find(|(_, card)| card.state == ReadyState::Open).or_else(|| cards().next())
    };
    found.map(|(at, card)| (at, card.id.clone()))
}

/// Where Fleet takes his answer to a card.
pub(super) const ANSWER_READY: &str = "/sessions/ready_to_try/answer";

/// The tab's two pictures each get at least this many points across, or the two stand one to a row.
pub(super) const PICTURE_LEAST: f32 = 520.;

/// Where a build stands: the colour of its band and the words on its chip.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Standing {
    Open,
    Later,
    Good,
    /// He landed it without trying it in his preview.
    Skipped,
    Rework,
    Withdrawn,
    Replaced,
}

impl Standing {
    /// What the card is as Fleet sent it, or as he just answered it before Fleet's row says so.
    pub(super) fn of(card: &ThreadReadyToTry, pressed: Option<&Answer>) -> Self {
        match pressed {
            Some(Answer::Approved) => return Standing::Good,
            Some(Answer::Skipped) => return Standing::Skipped,
            Some(Answer::SentBack(_)) => return Standing::Rework,
            Some(Answer::Later(_)) => return Standing::Later,
            _ => {}
        }
        match card.state {
            ReadyState::Open => Standing::Open,
            ReadyState::Later => Standing::Later,
            ReadyState::Answered if card.verdict == Some(NoteVerdict::NeedsWork) => Standing::Rework,
            ReadyState::Answered if card.skipped => Standing::Skipped,
            ReadyState::Answered => Standing::Good,
            ReadyState::Withdrawn => Standing::Withdrawn,
            ReadyState::Replaced => Standing::Replaced,
        }
    }

    pub(super) fn word(self) -> &'static str {
        match self {
            Standing::Open | Standing::Later => "Ready to try",
            Standing::Good => "Looks good",
            Standing::Skipped => "Skipped",
            Standing::Rework => "Needs rework",
            Standing::Withdrawn => "Withdrawn",
            Standing::Replaced => "Replaced by a newer build",
        }
    }

    pub(super) fn ink(self) -> Token {
        match self {
            Standing::Open => token::STATUS_COMPLETED_SUCCESS,
            Standing::Later | Standing::Replaced | Standing::Skipped => token::STATUS_NOT_STARTED,
            Standing::Good => token::STATUS_COMPLETED_SUCCESS,
            Standing::Rework => token::STATUS_REJECTED,
            Standing::Withdrawn => token::STATUS_KILLED,
        }
    }

    pub(super) fn tip(self) -> &'static str {
        match self {
            Standing::Open => "Waiting on you to try it",
            Standing::Later => "You asked to be reminded",
            Standing::Good => "You said it looks good",
            Standing::Skipped => "You landed it without trying it",
            Standing::Rework => "You sent it back for rework",
            Standing::Withdrawn => "The Job took the offer back",
            Standing::Replaced => "The Job offered the branch again at a newer tip",
        }
    }

    /// A build with nothing left to ask is one line in the thread.
    pub(super) fn folds(self) -> bool {
        matches!(self, Standing::Good | Standing::Skipped | Standing::Rework | Standing::Withdrawn | Standing::Replaced)
    }

    /// A build still to answer: open, or put off until a time.
    pub(super) fn asks(self) -> bool {
        !self.folds()
    }
}

/// What a build says while the running preview does not hold it: he cannot try it yet.
pub(super) const NOT_IN_PREVIEW: &str = "Not in your preview yet";

/// The tooltip of that state.
pub(super) const NOT_IN_PREVIEW_TIP: &str = "The running preview does not hold this build yet";

/// What a build says while Fleet is still merging and compiling its offer: Refresh preview cannot take it yet.
pub(super) const CHECKING_FOR_PREVIEW: &str = "Checking that the build will work";

/// What a build says once Fleet's check failed its offer.
pub(super) const OFF_PREVIEW_LIST: &str = "Off the preview list";

/// Where a build stands against the preview: in it, or why not yet. **The offer's check decides**, not only the running build,
/// since Refresh preview takes only an offer Fleet has checked: a card that offered it while the check ran rebuilt without the branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Preview {
    /// The running preview holds it at its tip.
    In,
    /// Ready for the preview, which does not hold it yet: Refresh preview brings it in.
    NotIn,
    /// Fleet is still merging it onto main and compiling it.
    Checking,
    /// Fleet's check failed it, with the first error or conflict where Fleet said one.
    Off(Option<String>),
}

impl Preview {
    /// The build as the card and the offers Fleet last listed say it. No offer for its branch at its tip, or no list yet, reads
    /// as ready, as the card always has.
    pub(super) fn of(card: &ThreadReadyToTry, offers: Option<&PreviewOffers>) -> Preview {
        if card.in_build {
            return Preview::In;
        }
        match offers.and_then(|offers| offers.offers.iter().find(|one| one.branch == card.branch && one.tip == card.tip)) {
            Some(one) if one.state == PreviewState::Checking => Preview::Checking,
            Some(one) if one.state == PreviewState::Failed => Preview::Off(one.why.clone()),
            _ => Preview::NotIn,
        }
    }

    /// Whether Refresh preview would bring the build in.
    pub(super) fn refreshes(&self) -> bool {
        *self == Preview::NotIn
    }

    /// The chip's words while the preview does not hold the build; `None` once it does.
    pub(super) fn word(&self) -> Option<&'static str> {
        match self {
            Preview::In => None,
            Preview::NotIn => Some(NOT_IN_PREVIEW),
            Preview::Checking => Some(CHECKING_FOR_PREVIEW),
            Preview::Off(_) => Some(OFF_PREVIEW_LIST),
        }
    }

    /// The tooltip of that state.
    pub(super) fn tip(&self) -> SharedString {
        match self {
            Preview::In => SharedString::default(),
            Preview::NotIn => NOT_IN_PREVIEW_TIP.into(),
            Preview::Checking => "Fleet is merging this build onto main and compiling it. Refresh preview can take it once that passes".into(),
            Preview::Off(Some(why)) => format!("Fleet's check failed it: {why}").into(),
            Preview::Off(None) => "Fleet's check failed it, and the Job is told why".into(),
        }
    }
}

/// The Refresh keys under a head band: live where a build here is ready for the preview, else off with why.
pub(super) struct RefreshKeys {
    pub ready: bool,
    pub why: SharedString,
}

/// An open build the running preview does not hold. It reads as pending, never as ready.
pub(super) fn pending(standing: Standing, preview: &Preview) -> bool {
    standing == Standing::Open && *preview != Preview::In
}

/// The label and ink of the built picture's frame: green once the running preview holds the build, amber until then.
pub(super) fn built_label(preview: &Preview) -> (&'static str, Token) {
    match preview.word() {
        None => ("Built \u{b7} in the preview", token::STATUS_COMPLETED_SUCCESS),
        Some(word) => (word, token::STATUS_AWAITING_REVIEW),
    }
}

/// What the pictures row holds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Pictures {
    /// The approved drawing beside the built picture.
    Both,
    /// The built picture alone, across the whole row.
    Built,
    /// What he approved alone: the Job attached no shot.
    Approved,
    /// No row at all.
    None,
}

/// What a card holds, as its parts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Parts {
    pub pictures: Pictures,
    pub words: bool,
    /// The look sentence, the steps, or both.
    pub fold: bool,
    pub steps: Vec<StepLine>,
}

impl Parts {
    pub(super) fn of(card: &ThreadReadyToTry) -> Parts {
        let pictures = match (card.approved.is_some(), !card.pictures.is_empty()) {
            (true, true) => Pictures::Both,
            (false, true) => Pictures::Built,
            (true, false) => Pictures::Approved,
            (false, false) => Pictures::None,
        };
        let look = card.look.as_deref().is_some_and(|look| !look.trim().is_empty());
        Parts {
            pictures,
            words: card.words.as_deref().is_some_and(|words| !words.trim().is_empty()),
            fold: look || !card.steps.is_empty(),
            steps: step_lines(&card.steps),
        }
    }

    /// The act Show me how runs: the first step that has one the registry knows. None where no step has.
    pub(super) fn first_go(&self) -> Option<&crate::build_notes::Go> {
        self.steps.iter().find_map(|line| line.go.as_ref())
    }
}

/// Whether the two pictures stand one to a row: each would be under `least` points across.
pub(super) fn stacked(inner: f32, gap: f32, least: f32) -> bool {
    (inner - gap) / 2. < least
}

/// The body of his answer, as it goes to Fleet.
pub(super) fn answer_body(job: &str, card: &ThreadReadyToTry, answer: ReadyAnswer) -> Result<String, String> {
    ipc::encode(&AnswerReadyToTry { session_id: SessionId::carried(job.to_string()), branch: card.branch.clone(), tip: card.tip.clone(), answer }).map_err(|why| format!("{why:?}"))
}

/// The time a card shows for a stamp: the clock alone today, the day before it on any other.
pub(super) fn clock(at: &Instant, now: DateTime<Local>) -> String {
    let Some(at) = super::acts::at_of(at) else { return String::new() };
    let time = crate::settings::time::time_of(at, ipc::time_format::TimeFormat { seconds: false, ..crate::settings::time::held() });
    if at.date_naive() == now.date_naive() {
        time
    } else {
        format!("{} {time}", at.format("%-d %b"))
    }
}

/// How tall a picture's box is: five of the card's 48 px steps.
fn picture_height(look: &Look) -> gpui::Pixels {
    look.length(token::SPACE_12) * 5.
}

/// A quiet 2xs label in a muted ink, for the head of a picture.
fn small(look: &Look) -> gpui::Div {
    div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
}

impl JobView {
    /// A build in the thread: the one line of a build that is settled, and nothing for one the compact card at the thread's end
    /// lists.
    pub(super) fn ready_card(&self, card: &ThreadReadyToTry, look: &Look, now: DateTime<Local>, page: &Page) -> Option<AnyElement> {
        let standing = Standing::of(card, page.cards.answers.get(&card.id));
        standing.folds().then(|| self.ready_folded(card, standing, look, now))
    }

    /// The Job's one compact card, drawn after the thread's items at `ix`. None where he has answered every build before Fleet has
    /// said so.
    pub(super) fn ready_pinned(&self, ix: usize, page: &Page, look: &Look, now: DateTime<Local>, cx: &Context<Self>) -> Option<AnyElement> {
        let held = self.thread(self.shown.as_ref()?, cx)?;
        let all = super::ready_compact::builds(&held.items, &page.cards.answers);
        all.iter().any(|build| build.standing.asks()).then(|| self.ready_compact(&all, ix, page, look, now, true, cx))
    }

    /// The builds to try as the first card's body, at its list place `ix`: the Refresh keys and a row for each. None where he has
    /// answered every build before Fleet has said so.
    pub(super) fn ready_inside(&self, ix: usize, page: &Page, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let held = self.thread(self.shown.as_ref()?, cx)?;
        let all = super::ready_compact::builds(&held.items, &page.cards.answers);
        all.iter().any(|build| build.standing.asks()).then(|| self.ready_compact(&all, ix, page, look, Local::now(), false, cx))
    }

    /// What each press on a build's answers does. Looks good answers, Needs rework opens its box and Send sends it, Show me how
    /// runs the first step's act.
    pub(super) fn ready_run(&self, ix: usize, card: &ThreadReadyToTry, parts: Parts) -> Do {
        let (key, card) = (card.id.clone(), card.clone());
        Rc::new(move |this, act, window, cx| match act {
            Act::Approve => this.answer_ready(ix, &card, ReadyAnswer::LooksGood, window, cx),
            Act::Skip => this.answer_ready(ix, &card, ReadyAnswer::Skip, window, cx),
            // With his words in the box the press sends them with the mark; with the box empty it puts the keys there.
            Act::Ask => {
                let words = this.typed_in(&key, Mode::Note, cx).trim().to_string();
                if words.is_empty() {
                    this.ask_for_changes(ix, &key, window, cx);
                } else {
                    this.answer_ready(ix, &card, ReadyAnswer::NeedsRework { words }, window, cx);
                }
            }
            Act::Send => {
                let words = this.typed_in(&key, Mode::Note, cx).trim().to_string();
                if !words.is_empty() {
                    this.answer_ready(ix, &card, ReadyAnswer::NeedsRework { words }, window, cx);
                }
            }
            Act::Cancel => this.shut_band_note(ix, &key, window, cx),
            Act::Open => {
                if let Some(go) = parts.first_go() {
                    window.dispatch_action(Box::new(crate::keymap::Act { id: go.act, slot: 0, digit: go.digit }), cx);
                }
            }
            Act::Dismiss | Act::Archive | Act::Edit => {}
        })
    }

    /// His answer: the card reads it in this frame, Fleet is told off the main thread, and a refusal takes it back with Fleet's
    /// sentence on the build. The build opened moves to the next open one. `ix` is the thread item the card is drawn at, or
    /// `SHEET` where the review overlay hosts it.
    pub(super) fn answer_ready(&mut self, ix: usize, card: &ThreadReadyToTry, answer: ReadyAnswer, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let pressed = match &answer {
            ReadyAnswer::LooksGood => Answer::Approved,
            ReadyAnswer::Skip => Answer::Skipped,
            ReadyAnswer::NeedsRework { words } => Answer::SentBack(words.clone()),
            ReadyAnswer::Remind { until } => Answer::Later(until.as_str().to_string()),
        };
        let next = match (self.pages.get(&job), self.thread(&job, cx)) {
            (Some(page), Some(held)) => super::ready_compact::next_open(&super::ready_compact::builds(&held.items, &page.cards.answers), &card.id),
            _ => None,
        };
        let body = answer_body(&job, card, answer);
        self.settle(ix, card.id.clone(), pressed, ANSWER_READY, body, window, cx);
        // The build's own item turns from nothing to its one line.
        if let (Some(page), Some(held)) = (self.pages.get(&job), self.thread(&job, cx)) {
            if let Some(at) = held.items.iter().position(|item| matches!(item, ThreadItem::ReadyToTry(one) if one.id == card.id)) {
                page.remeasure(at);
            }
        }
        // A refusal came back at once, so the build stays open with Fleet's sentence on it; else the next open one opens.
        let mut advance = None;
        if let Some(cards) = self.cards_mut() {
            if !cards.said.contains_key(&card.id) {
                advance = next.clone();
                if cards.ready_pick.as_deref().is_none_or(|pick| pick == card.id) {
                    cards.ready_pick = Some(next.unwrap_or_else(|| card.id.clone()));
                }
            }
        }
        // In the review panel the next open build comes up, as a build in the thread's card does.
        if let Some(next) = advance.filter(|_| ix == super::acts::SHEET && self.review_open()) {
            self.review_at(&next, window, cx);
        }
    }

    /// The head band: the state's chip, the Job, and the time at the right in mono. `key` names its parts, so a thread's band and
    /// a tab's differ. `pending` is the tooltip of a build the preview does not hold yet; `refresh` puts the Refresh keys under the band.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn ready_head(&self, key: &str, standing: Standing, word: SharedString, at: &Instant, offered: Option<&Instant>, pending: Option<SharedString>, refresh: Option<RefreshKeys>, now: DateTime<Local>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let band = if pending.is_some() { token::STATUS_AWAITING_REVIEW } else { standing.ink() };
        let ink = look.color(band);
        let chip = labelled(
            div()
                .id(SharedString::from(format!("state:{key}")))
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .px(look.length(token::SPACE_2))
                .rounded_full()
                .bg(ink.opacity(0.16))
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_2XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(ink)
                .tooltip(tip(pending.unwrap_or_else(|| standing.tip().into())))
                .child(gpui::svg().path(crate::assets::icon_path("circle-dot")).flex_none().size(look.length(token::TEXT_2XS)).text_color(ink))
                .child(word.clone()),
            Role::Status,
            word,
        );
        let job = self.shown.as_ref().and_then(|job| self.title_of(job, cx));
        let seen = offered.map(|offered| format!("Offered {}", clock(offered, now)));
        let strip = refresh.map(|keys| self.refresh_strip(key, keys, look, cx));
        let band = div()
            .id(SharedString::from(format!("ready-head:{key}")))
            .flex()
            .items_center()
            .min_w_0()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .rounded_t(look.length(token::RADIUS_LG) * 1.5 - look.length(token::BORDER_WIDTH))
            .bg(mixed(look, band, 0.18))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(ink.opacity(0.4))
            .child(chip)
            .child(
                div()
                    .id(SharedString::from(format!("ready-job:{key}")))
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .when_some(job, |one, job| one.tooltip(tip(job.clone())).child(job)),
            )
            .child(
                div()
                    .id(SharedString::from(format!("ready-time:{key}")))
                    .flex_none()
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .text_color(look.color(token::FG_MUTED))
                    .when_some(seen, |one, seen| one.tooltip(tip(seen)))
                    .child(SharedString::from(clock(at, now))),
            );
        match strip {
            None => band.into_any_element(),
            Some(strip) => div().flex().flex_col().child(band).child(strip).into_any_element(),
        }
    }

    /// The preview rebuild under way, whichever Refresh preview started it, on this Mac or another: Fleet's one build report says
    /// so, so every key and card reads the same. `None` while no rebuild runs.
    pub(super) fn preview_refreshing(&self, cx: &App) -> Option<crate::titlebar::Refreshing> {
        crate::titlebar::refreshing(self.fleet.as_ref()?.read(cx).held().build.as_ref()?)
    }

    /// Where a build stands against the preview, read against the offers Fleet last listed.
    pub(super) fn preview_of(&self, card: &ThreadReadyToTry, cx: &App) -> Preview {
        Preview::of(card, self.fleet.as_ref().and_then(|fleet| fleet.read(cx).held().offers.as_ref()))
    }

    /// The strip under a head band while a build is not in the preview: the title bar's Refresh keys, and under them what Refresh
    /// builds. Off, with why, while no build here is ready for it, so a press never rebuilds without the branch.
    pub(super) fn refresh_strip(&self, key: &str, keys: RefreshKeys, look: &Look, cx: &Context<Self>) -> AnyElement {
        let offers = self.fleet.as_ref().and_then(|fleet| fleet.read(cx).held().offers.as_ref());
        let macs = crate::pairing::macs(cx);
        let started = self.preview_refreshing(cx).and_then(|now| now.started_said(macs));
        // The line says it in a few words; hover names every Job Refresh brings in.
        let whole: Option<SharedString> = (started.is_none() && keys.ready).then(|| crate::titlebar::refresh_whole(offers, macs).into());
        let line: SharedString = match started {
            Some(started) => started.into(),
            None if keys.ready => crate::titlebar::refresh_said(offers, macs).into(),
            None => keys.why.clone(),
        };
        div()
            .id(SharedString::from(format!("ready-refresh:{key}")))
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(self.refresh_preview_keys(key, &keys, look, cx))
            .child(labelled(
                div()
                    .id(SharedString::from(format!("ready-refresh-said:{key}")))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .when_some(whole, |shown, whole| shown.tooltip(tip(whole)))
                    .child(line.clone()),
                Role::Status,
                line,
            ))
            .into_any_element()
    }

    /// Refresh preview, where a build is not in the running preview yet: the title bar's build switcher's Refresh keys, pressed here.
    /// Refresh both Macs and This Mac only where another Mac is paired, Refresh preview where none is. While a rebuild runs, from a
    /// press here or anywhere else, it reads Refreshing with its stage and takes no press.
    fn refresh_preview_keys(&self, key: &str, keys: &RefreshKeys, look: &Look, cx: &Context<Self>) -> AnyElement {
        use crate::titlebar::parts::{Btn, Weight};
        if let Some(now) = self.preview_refreshing(cx) {
            let stage = now.tip(crate::pairing::macs(cx));
            let ink = look.color(token::FG_MUTED);
            return labelled(
                act(look, format!("refresh-preview:{key}"), "Refreshing", Kind::Secondary, "", true)
                    .cursor_default()
                    .child(crate::titlebar::parts::spinner(SharedString::from(format!("refreshing:{key}")), look.length(token::TEXT_XS), ink, cx))
                    .tooltip(tip(stage)),
                Role::Status,
                "Refreshing",
            )
            .into_any_element();
        }
        let partner = crate::pairing::refresh_partner(crate::pairing::macs(cx)).map(|other| other.name.clone());
        let off = !keys.ready;
        let label = if partner.is_some() { "Refresh both Macs" } else { "Refresh preview" };
        let said: SharedString = match (&partner, off) {
            (_, true) => keys.why.clone(),
            (Some(name), false) => format!("Merge the offered branches onto latest main, restart Fleet, and have {name} build the same commit").into(),
            (None, false) => "Merge the offered branches onto latest main and restart Fleet".into(),
        };
        let both = Btn::new(SharedString::from(format!("refresh-preview:{key}")), label, Weight::Primary).icon("rotate-cw").disabled(off).draw(look, cx).tooltip(tip(said));
        let both = if off { both } else { both.tab_index(0).on_click(cx.listener(|this, _, _, cx| this.refresh_preview(cx))) };
        // Beside it, the press that moves this Mac only, as the title bar's.
        let alone = partner.is_some().then(|| {
            let said: SharedString = if off { keys.why.clone() } else { "Merge the offered branches onto latest main and restart Fleet here only".into() };
            let button = Btn::new(SharedString::from(format!("refresh-this-mac:{key}")), "This Mac only", Weight::Secondary).disabled(off).draw(look, cx).tooltip(tip(said));
            let button = if off { button } else { button.tab_index(0).on_click(cx.listener(|this, _, _, cx| this.refresh_this_mac(cx))) };
            labelled(button, Role::Button, "This Mac only")
        });
        div().flex().items_center().gap(look.length(token::SPACE_2)).child(labelled(both, Role::Button, label)).children(alone).into_any_element()
    }

    /// This Mac only: Fleet here restarts onto the preview, and the other Mac stays where it is.
    fn refresh_this_mac(&mut self, cx: &mut Context<Self>) {
        if let Some(fleet) = self.fleet.clone() {
            let act = crate::fleet::BuildAct::Change(ipc::ChangeFleetBuild { build: ipc::BuildSource::Preview, adopt: false, clear: None });
            fleet.update(cx, |fleet, cx| fleet.build_act(act, cx));
        }
    }

    /// The same act as the Refresh row of the title bar's build switcher: Fleet restarts onto a preview of the offered branches, both Macs where another is paired.
    fn refresh_preview(&mut self, cx: &mut Context<Self>) {
        if let Some(fleet) = self.fleet.clone() {
            let act = crate::pairing::refresh_act(crate::pairing::macs(cx));
            fleet.update(cx, |fleet, cx| fleet.build_act(act, cx));
        }
    }

    /// A build whole, at the width it is given, drawn at thread item `ix` (or `SHEET` in the overlay): head band, title, pictures, his words, where to look with the steps,
    /// and the foot of four answers while it still asks.
    pub(super) fn ready_detail(&self, card: &ThreadReadyToTry, ix: usize, wide: f32, page: &Page, now: DateTime<Local>, look: &Look, cx: &Context<Self>) -> AnyElement {
        self.ready_build(card, ix, wide, page, now, look, cx, true)
    }

    /// The open build under its row in the compact card: the same body without a frame or head of its own, so the card stays one.
    pub(super) fn ready_detail_inside(&self, card: &ThreadReadyToTry, ix: usize, wide: f32, page: &Page, now: DateTime<Local>, look: &Look, cx: &Context<Self>) -> AnyElement {
        self.ready_build(card, ix, wide, page, now, look, cx, false)
    }

    #[allow(clippy::too_many_arguments)]
    fn ready_build(&self, card: &ThreadReadyToTry, ix: usize, wide: f32, page: &Page, now: DateTime<Local>, look: &Look, cx: &Context<Self>, framed: bool) -> AnyElement {
        let key = card.id.clone();
        let standing = Standing::of(card, page.cards.answers.get(&key));
        let parts = Parts::of(card);
        let asking = standing.asks();
        let job = self.shown.clone().unwrap_or_default();
        let run = self.ready_run(ix, card, parts.clone());
        let foot = Foot { ix, key: key.clone(), asking, archivable: false, openable: false, editable: false, commit: true, route: ANSWER_READY, run };
        let preview = self.preview_of(card, cx);
        let waits = pending(standing, &preview);
        let ink = look.color(if waits { token::STATUS_AWAITING_REVIEW } else { standing.ink() });
        let word: SharedString = match (standing, super::ready_compact::until(card, &page.cards.answers)) {
            (Standing::Later, Some(until)) => format!("Later until {}", clock(&until, now)).into(),
            _ if waits => preview.word().unwrap_or(NOT_IN_PREVIEW).into(),
            _ => standing.word().into(),
        };
        let refresh = (asking && preview != Preview::In).then(|| RefreshKeys { ready: preview.refreshes(), why: preview.tip() });
        let head = self.ready_head(&format!("detail:{key}"), standing, word, &card.at, Some(&card.offered_at), waits.then(|| preview.tip()), refresh, now, look, cx);
        // The Judge reads the branch as its diff against main. Its press stands at the title's end, or at the corner of a card
        // unfolded under its row, and what it found stands under the card's parts.
        let judge = self.judge_on_card(ix, &key, ipc::JudgeArtifact { kind: ipc::JudgeKind::Branch, id: card.branch.clone(), title: card.about.clone() }, page, look, cx);
        let (judge_press, judge_notes) = judge.map_or((None, None), |judge| (Some(judge.press), judge.notes));
        let notes = judge_notes.map(|notes| div().px(look.length(token::SPACE_4)).pt(look.length(token::SPACE_3)).child(notes));
        let title = div()
            .id(SharedString::from(format!("ready-title:{key}")))
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .pt(look.length(token::SPACE_3))
            .text_size(look.length(token::TEXT_BASE))
            .line_height(look.length(token::LEADING_BASE))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(look.color(token::FG_DEFAULT))
            .child(div().flex_1().min_w_0().child(SharedString::from(card.about.clone())));
        let inner = wide - f32::from(look.length(token::SPACE_4)) * 2. - f32::from(look.length(token::BORDER_WIDTH)) * 2.;
        let pictures = self.ready_pictures(&job, card, &parts, inner, look, cx);
        let words = card.words.as_deref().filter(|words| !words.trim().is_empty()).map(|words| {
            div().px(look.length(token::SPACE_4)).pt(look.length(token::SPACE_3)).child(quote(look, &format!("\u{201c}{}\u{201d}", words.trim())))
        });
        let changed = parts.fold.then(|| self.ready_changed(card, &parts, look));
        // He cannot judge a build he cannot run: the answers wait until the preview holds it. Refresh preview stands in the head or the built picture's frame.
        let foot_row = (asking && card.in_build).then(|| foot_box(look).child(self.ready_footer(&foot, card, &parts, page, look, cx)));
        let name = format!("Ready to try: {}", card.about);
        // With no answer row under it, the last part keeps a gap above the card's foot.
        let bottom = if foot_row.is_none() { look.length(token::SPACE_4) } else { px(0.) };
        if !framed {
            // Unfolded under its row, which already names it: no title of its own, and a well that reads as the row's inside.
            let body = div()
                .id(SharedString::from(format!("ready:{key}")))
                .relative()
                .flex()
                .flex_col()
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(token::SURFACE_WELL))
                .border_l(look.length(token::BORDER_WIDTH) * 2.)
                .border_color(ink.opacity(0.45))
                .children(pictures)
                .children(words)
                .children(changed)
                .children(notes)
                .children(foot_row)
                .children(judge_press.map(|press| div().absolute().top(look.length(token::SPACE_1)).right(look.length(token::SPACE_1)).child(press)))
                .pb(bottom);
            return labelled(body, Role::Article, name).into_any_element();
        }
        labelled(
            super::band_card::frame(look, false)
                .id(SharedString::from(format!("ready:{key}")))
                .border_color(ink.opacity(0.45))
                .child(head)
                .child(title.children(judge_press))
                .children(pictures)
                .children(words)
                .children(changed)
                .children(notes)
                .children(foot_row)
                .pb(bottom),
            Role::Article,
            name,
        )
        .into_any_element()
    }

    /// The pictures row: what he approved beside what was built, or the built one across the whole row, or nothing. `inner` is the
    /// card's width less its own insets.
    fn ready_pictures(&self, job: &str, card: &ThreadReadyToTry, parts: &Parts, inner: f32, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        if parts.pictures == Pictures::None {
            return None;
        }
        let gap = look.length(token::SPACE_3);
        let inner = inner - f32::from(look.length(token::SPACE_4)) * 2.;
        let both = parts.pictures == Pictures::Both;
        let stack = both && stacked(inner, f32::from(gap), PICTURE_LEAST);
        let each = if both && !stack { (inner - f32::from(gap)) / 2. } else { inner };
        let approved = card.approved.as_ref().filter(|_| matches!(parts.pictures, Pictures::Both | Pictures::Approved)).map(|approved| self.ready_approved(job, card, approved, each, look, cx));
        let built = card.pictures.first().filter(|_| matches!(parts.pictures, Pictures::Both | Pictures::Built)).map(|path| self.ready_built(card, path, each, look, cx));
        Some(
            div()
                .id(SharedString::from(format!("ready-pictures:{}", card.id)))
                .flex()
                .gap(gap)
                .min_w_0()
                .px(look.length(token::SPACE_4))
                .pt(look.length(token::SPACE_3))
                .when(stack, |row| row.flex_col())
                .children(approved)
                .children(built)
                .into_any_element(),
        )
    }

    /// One picture's frame: its label over a sunken box the picture is centred in.
    fn ready_frame(&self, id: String, label: SharedString, extra: Option<AnyElement>, ink: Token, body: AnyElement, look: &Look) -> gpui::Stateful<gpui::Div> {
        let ink_color = look.color(ink);
        div()
            .id(SharedString::from(id))
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_SUNKEN))
            .child(
                small(look)
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_1) + look.length(token::BORDER_WIDTH) * 2.)
                    .bg(mixed(look, ink, 0.12))
                    .border_b(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .text_color(ink_color)
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(look.length(token::SPACE_2))
                    .child(label)
                    .children(extra),
            )
            .child(div().flex().items_center().justify_center().h(picture_height(look)).p(look.length(token::SPACE_2)).child(body))
    }

    /// The picture the Job built, which a press opens in the view that fills the app.
    fn ready_built(&self, card: &ThreadReadyToTry, path: &str, each: f32, look: &Look, cx: &Context<Self>) -> AnyElement {
        let file = super::pictures::read_picture(path).unwrap_or_else(|| SentFile { id: path.to_string(), name: path.rsplit('/').next().unwrap_or(path).to_string(), media_type: String::new() });
        let room = size((each - f32::from(look.length(token::SPACE_2)) * 2. - f32::from(look.length(token::BORDER_WIDTH)) * 2.).max(120.), f32::from(picture_height(look) - look.length(token::SPACE_2) * 2.));
        let body = self.picture_in(&file, room, look, cx).unwrap_or_else(|| {
            div().text_size(look.length(token::TEXT_XS)).font_family(mono(look)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(file.name.clone())).into_any_element()
        });
        // The Refresh keys stand under the card's head band, so the frame only says where the build stands.
        let (label, ink) = built_label(&self.preview_of(card, cx));
        self.ready_frame(format!("ready-built:{}", card.id), label.into(), None, ink, body, look).into_any_element()
    }

    /// What he approved, at the revision he approved, drawn as the card of its kind draws it. A press reads it large in its own tab.
    fn ready_approved(&self, job: &str, card: &ThreadReadyToTry, approved: &ReadyApproved, each: f32, look: &Look, cx: &Context<Self>) -> AnyElement {
        let height = picture_height(look) - look.length(token::SPACE_2) * 2.;
        let width = (each - f32::from(look.length(token::SPACE_2)) * 2. - f32::from(look.length(token::BORDER_WIDTH)) * 2.).max(120.);
        let drawn: Option<AnyElement> = match approved.form {
            ArtifactForm::Sketch => self.thread(job, cx).and_then(|held| newest_card(&held.head, &held.items, &approved.artifact)).and_then(|sketch| {
                let revisions = sketch.revisions.len();
                let at = (revisions > 0 && approved.revision >= 1 && approved.revision as usize <= revisions).then(|| approved.revision as usize - 1);
                let (scene_key, scene) = shown_scene(sketch, at);
                scene.map(|scene| {
                    div()
                        .relative()
                        .w(px(width))
                        .h(height)
                        .overflow_hidden()
                        .child(crate::scene::view(SharedString::from(format!("ready:{}:{scene_key}", card.id)), scene, Fit::Preview, Clock::default(), Lens::default()))
                        .into_any_element()
                })
            }),
            ArtifactForm::Page => {
                let dark = look.tone() == crate::look::Tone::Dark;
                let want = PictureKey { job: job.to_string(), page: approved.artifact.clone(), revision: approved.revision, width: width.round() as u32, dark };
                match self.picture_of(&want) {
                    Some(Picture::Ready(image, tall)) => Some(div().w(px(width)).h(height).overflow_hidden().child(gpui::img(image.clone()).w(px(width)).h(px(*tall))).into_any_element()),
                    Some(Picture::Failed) => None,
                    _ => Some(self.asking_picture(&want, well(look, true, shapes(look)), cx)),
                }
            }
            ArtifactForm::Draft => None,
        };
        // A draft, or one that cannot be drawn, stands as its kind's tile and its title.
        let body = drawn.unwrap_or_else(|| {
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(tile(look, approved.form))
                .child(div().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(approved.title.clone())))
                .into_any_element()
        });
        let label = match approved.approved_at.as_ref().and_then(super::acts::at_of) {
            Some(at) => format!("You approved \u{b7} {}", at.format("%-d %b")),
            None => "You approved".to_string(),
        };
        let (artifact, named) = (approved.artifact.clone(), format!("{}, {}", kind_look(approved.form).2, approved.title));
        labelled(
            self.ready_frame(format!("ready-approved:{}", card.id), label.into(), None, token::YOU, body, look)
                .cursor_pointer()
                .tooltip(tip(format!("Read {} larger", approved.title)))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.open_named(&artifact, window, cx);
                })),
            Role::Button,
            format!("You approved: {named}"),
        )
        .into_any_element()
    }

    /// What changed and where to look: the look sentence, then the steps, each with its key where it has an act.
    fn ready_changed(&self, card: &ThreadReadyToTry, parts: &Parts, look: &Look) -> AnyElement {
        let head = labelled(
            div()
                .id(SharedString::from(format!("ready-changed:{}", card.id)))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(look.color(token::FG_MUTED))
                .child("What changed and where to look"),
            Role::Heading,
            "What changed and where to look",
        );
        let look_line = card.look.as_deref().map(str::trim).filter(|words| !words.is_empty()).map(|words| {
            div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(words.to_string()))
        });
        let steps: Vec<AnyElement> = parts
            .steps
            .iter()
            .map(|line| {
                div()
                    .id(SharedString::from(format!("ready-step:{}:{}", card.id, line.number)))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .child(
                        div()
                            .flex_none()
                            .size(look.length(token::SPACE_6))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .bg(look.color(token::ACCENT_MUTED))
                            .font_family(mono(look))
                            .text_size(look.length(token::TEXT_XS))
                            .child(line.number.to_string()),
                    )
                    .child(div().flex_1().min_w_0().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).child(SharedString::from(line.say.clone())))
                    .children(line.go.as_ref().filter(|go| !go.keycap.is_empty()).map(|go| kbd(look, &go.keycap)))
                    .into_any_element()
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .pt(look.length(token::SPACE_3))
            // Room under the last step before the foot's hairline (his note of 27 Nov).
            .pb(look.length(token::SPACE_4))
            .child(head)
            .children(look_line)
            .children((!steps.is_empty()).then(|| labelled(div().id(SharedString::from(format!("ready-steps:{}", card.id))).flex().flex_col().gap(look.length(token::SPACE_2)).children(steps), Role::List, "Steps")))
            .into_any_element()
    }

    /// The foot: Looks good, Needs rework, Show me how and Skip, a spacer, then Remind me. Needs rework opens its box under them.
    fn ready_footer(&self, foot: &Foot, card: &ThreadReadyToTry, parts: &Parts, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = foot.key.as_str();
        let mut column = div().flex().flex_col().gap(look.length(token::SPACE_2));
        if !page.cards.modes.contains_key(key) && self.holds_kept(key) {
            page.cards.kept_seen.borrow_mut().push((key.to_string(), foot.ix, true, false));
        }
        if let Some(why) = page.cards.said.get(key) {
            column = column.child(refusal(look, key, why.clone()));
        }
        let (good, rework, how, skip) = (foot.run.clone(), foot.run.clone(), foot.run.clone(), foot.run.clone());
        let button = |id: &str, said: &'static str, kind: Kind, cap: &str, on: bool| act(look, format!("{id}:{key}"), said, kind, cap, on).tab_index(0).focus_visible(|style| style.border_color(look.color(token::ACCENT)));
        let remind = page.keys.spot(&format!("remind:{key}"), cx);
        let drawn = remind.bounds.clone();
        let (ix, held) = (foot.ix, card.clone());
        let shows = parts.first_go().is_some();
        let row = div()
            .id(SharedString::from(format!("answers:{key}")))
            .flex()
            .flex_wrap()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(labelled(
                button("good", "Looks good", Kind::Approve, "1", true).tooltip(tip("It does what you asked: the Job lands it")).on_click(cx.listener(move |this, _, window, cx| good(this, Act::Approve, window, cx))),
                Role::Button,
                "Looks good",
            ))
            .child(labelled(
                button("rework", "Needs rework", Kind::Secondary, "2", true).tooltip(tip("Send it back with what should change")).on_click(cx.listener(move |this, _, window, cx| rework(this, Act::Ask, window, cx))),
                Role::Button,
                "Needs rework",
            ))
            .child(labelled(
                button("how", "Show me how", Kind::Secondary, "3", shows).when(shows, |one| one.tooltip(tip("Go through the steps")).on_click(cx.listener(move |this, _, window, cx| how(this, Act::Open, window, cx)))),
                Role::Button,
                "Show me how",
            ))
            .child(labelled(
                button("skip", "Skip", Kind::Secondary, "4", true).tooltip(tip("Land it without trying it in your preview")).on_click(cx.listener(move |this, _, window, cx| skip(this, Act::Skip, window, cx))),
                Role::Button,
                "Skip",
            ))
            .child(div().flex_1())
            .child(labelled(
                button("remind", "Remind me", Kind::Ghost, "", true)
                    .relative()
                    .child(glyph(look, "chevron-down", Size::Small, look.color(token::FG_MUTED)))
                    .child(canvas(move |bounds, _, _| drawn.set(bounds), |_, _, _, _| {}).absolute().top_0().left_0().size_full())
                    .tooltip(tip("Bring this build back later"))
                    .on_click(cx.listener(move |this, _, window, cx| this.open_remind(ix, &held, window, cx))),
                Role::Button,
                "Remind me",
            ));
        let column = column.child(super::acts::gated(row, self.off_shown_to(ANSWER_READY, cx)));
        // The box for what should change stands under the answers, and Needs rework stays among them: pressed with words in the
        // box it sends them with the mark, and with the box empty it puts the keys there.
        if page.cards.modes.get(key) == Some(&Mode::Note) {
            return column.child(self.band_note(foot, page, look, cx)).into_any_element();
        }
        column.into_any_element()
    }

    /// Remind me's menu under its button: each choice of the Later ladder with the time it brings the build back.
    fn open_remind(&mut self, ix: usize, card: &ThreadReadyToTry, window: &mut Window, cx: &mut Context<Self>) {
        use crate::dropdown::{Dropdown, Entry, Picked, Row};
        let Some(page) = self.shown.as_ref().and_then(|id| self.pages.get(id)) else { return };
        let bounds = page.keys.spot(&format!("remind:{}", card.id), cx).bounds.get();
        let now = Local::now();
        let choices = crate::operator::face::later_choices(now);
        let entries = choices.iter().map(|(id, label, at)| Entry::Row(Row::new(id, *label).detail(crate::operator::face::back_at(*at, now)))).collect();
        let back = window.focused(cx);
        let menu = cx.new(|cx| Dropdown::new("Remind me", entries, super::head::below_left(bounds), gpui::Anchor::TopLeft, true, cx).at_point(window, cx));
        let (again, held) = (back.clone(), card.clone());
        let picked = cx.subscribe_in(&menu, window, move |this, _, picked: &Picked, window, cx| {
            let Picked::Entry { id: choice, .. } = picked;
            this.tabs_menu = None;
            if let Some(back) = again.as_ref() {
                window.focus(back, cx);
            }
            if let Some((_, _, at)) = choices.iter().find(|(id, ..)| id == choice) {
                this.answer_ready(ix, &held, ReadyAnswer::Remind { until: Instant::carried(at.to_rfc3339()) }, window, cx);
            }
            cx.notify();
        });
        let dismissed = cx.subscribe_in(&menu, window, move |this, _, _: &gpui::DismissEvent, window, cx| {
            this.tabs_menu = None;
            if let Some(back) = back.as_ref() {
                window.focus(back, cx);
            }
            cx.notify();
        });
        let keys = gpui::Focusable::focus_handle(menu.read(cx), cx);
        window.defer(cx, move |window, cx| window.focus(&keys, cx));
        self.tabs_menu = Some(super::tabs::AddMenu { menu, _subs: vec![picked, dismissed] });
        cx.notify();
    }

    /// A build he answered, or the Job withdrew or replaced, in one line at its own time: where it stands, its title and the time.
    fn ready_folded(&self, card: &ThreadReadyToTry, standing: Standing, look: &Look, now: DateTime<Local>) -> AnyElement {
        let key = card.id.clone();
        let ink = look.color(standing.ink());
        let chip = labelled(
            div()
                .id(SharedString::from(format!("state:{key}")))
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
                .tooltip(tip(standing.tip()))
                .child(gpui::svg().path(crate::assets::icon_path("circle-dot")).flex_none().size(look.length(token::TEXT_2XS)).text_color(ink))
                .child(standing.word()),
            Role::Status,
            standing.word(),
        )
        .into_any_element();
        let about = div()
            .id(SharedString::from(format!("ready-about:{key}")))
            .flex_1()
            .min_w_0()
            .truncate()
            .text_size(look.length(token::TEXT_XS))
            .text_color(look.color(token::FG_MUTED))
            .tooltip(tip(card.about.clone()))
            .child(SharedString::from(card.about.clone()))
            .into_any_element();
        let time = div()
            .id(SharedString::from(format!("ready-time:{key}")))
            .flex_none()
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_2XS))
            .text_color(look.color(token::FG_MUTED))
            .child(SharedString::from(clock(&card.at, now)))
            .into_any_element();
        let line = self.quiet_base(&format!("ready:{key}"), "circle-dot", vec![chip, about, time], look);
        labelled(line, Role::ListItem, format!("Ready to try: {}, {}", card.about, standing.word())).into_any_element()
    }

    /// The Job's newest open build whole, for the band's focus view of its one Ready to try row: the same detail, with Looks good,
    /// Needs rework, Show me how, Remind me and Refresh preview, and never the generic question deck with its Answer button.
    pub(super) fn ready_in_band(&self, job: &SharedString, look: &Look, cx: &Context<Self>) -> AnyElement {
        let drawn = (|| {
            let (page, held) = (self.pages.get(job)?, self.thread(job, cx)?);
            let (at, _) = card_named(&held.items, &format!("try:{job}"))?;
            let ThreadItem::ReadyToTry(card) = held.items.get(at)? else { return None };
            let wide = f32::from(self.pane.unwrap_or(px(720.)) - look.length(token::SPACE_4) * 2.);
            Some(self.ready_detail(card, super::acts::SHEET, wide, page, Local::now(), look, cx))
        })();
        drawn.unwrap_or_else(|| div().into_any_element())
    }

    /// A press on a Job's builds to try, on its Needs you row or its For you row: the review panel opens over the window with each build
    /// still to answer listed and one chosen: the build the press names, else the newest. A build already answered, and a Job with none
    /// still asking, go to the line in the thread as before, and so does a panel that could not open.
    pub(super) fn open_ready_review(&mut self, job: &str, id: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let pick = self.thread(job, cx).and_then(|held| {
            let mut asking = held.items.iter().rev().filter_map(|item| match item {
                ThreadItem::ReadyToTry(card) if matches!(card.state, ReadyState::Open | ReadyState::Later) => Some(card.id.clone()),
                _ => None,
            });
            // A build's own id names that build, and the Job's row its newest.
            if id.contains('@') { asking.find(|asking| asking == id) } else { asking.next() }
        });
        let Some(pick) = pick else { return self.go_to_ready(job, id, window, cx) };
        self.open_review_with(super::review::Scope::Job(job.to_string()), None, Some(&pick), window, cx);
        self.review_open() || self.go_to_ready(job, id, window, cx)
    }

    /// The build an item in Needs you, or a press on a build, names: opened in place under the compact card at the thread's end,
    /// and the thread following its tail to it. A build already settled is one line, and the thread scrolls to that line. A Job
    /// whose thread holds no such build opens nothing.
    pub(super) fn go_to_ready(&mut self, job: &str, id: &str, _window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some((id, at, asks)) = self.thread(job, cx).and_then(|held| {
            let (at, id) = card_named(&held.items, id)?;
            let asks = matches!(held.items.get(at), Some(ThreadItem::ReadyToTry(card)) if matches!(card.state, ReadyState::Open | ReadyState::Later));
            Some((id, at, asks))
        }) else {
            return false;
        };
        let Some(page) = self.pages.get_mut(job) else { return false };
        page.cards.ready_pick = Some(id);
        match page.ready_at().filter(|_| asks) {
            Some(card) => {
                page.remeasure(card);
                page.list.set_follow_mode(gpui::FollowMode::Tail);
            }
            None => {
                page.remeasure(at);
                page.list.scroll_to(gpui::ListOffset { item_ix: page.entry(at), offset_in_item: gpui::px(0.) });
            }
        }
        cx.notify();
        true
    }

    /// Mock data: Fleet takes his answer as it does, and refuses it in its own words (`answer_ready_to_try`).
    pub(super) fn mock_ready_answered(&mut self, job: &SharedString, body: Result<String, String>, cx: &mut Context<Self>) -> Result<(), String> {
        let asked = body.and_then(|body| ipc::decode::<AnswerReadyToTry>("AnswerReadyToTry", body.as_bytes()).map_err(|why| why.to_string()))?;
        self.mock_ready.push(asked.clone());
        let id = format!("try:{}@{}", asked.branch, asked.tip);
        let held = self.mock.get_mut(job.as_str()).ok_or_else(|| format!("that job's thread holds no ready to try card for {} at {}", asked.branch, asked.tip))?;
        let at = held
            .items
            .iter()
            .position(|item| matches!(item, ThreadItem::ReadyToTry(card) if card.id == id))
            .ok_or_else(|| format!("that job's thread holds no ready to try card for {} at {}", asked.branch, asked.tip))?;
        let ThreadItem::ReadyToTry(card) = &mut held.items[at] else { return Ok(()) };
        match card.state {
            ReadyState::Answered => return Err("that card is answered".into()),
            ReadyState::Withdrawn => return Err("that card is withdrawn".into()),
            ReadyState::Replaced => return Err("that card is replaced".into()),
            ReadyState::Open | ReadyState::Later => {}
        }
        match asked.answer {
            ReadyAnswer::LooksGood | ReadyAnswer::Skip => (card.state, card.verdict, card.later_until) = (ReadyState::Answered, Some(NoteVerdict::Good), None),
            ReadyAnswer::NeedsRework { words } => {
                if words.trim().is_empty() {
                    return Err("words on the note: there are none to send".into());
                }
                (card.state, card.verdict, card.later_until) = (ReadyState::Answered, Some(NoteVerdict::NeedsWork), None);
            }
            ReadyAnswer::Remind { until } => {
                let then = DateTime::parse_from_rfc3339(until.as_str()).map_err(|_| "a time to remind him at: that one could not be read".to_string())?;
                if then.with_timezone(&Local) <= Local::now() {
                    return Err("a time to remind him at that has not passed".into());
                }
                (card.state, card.later_until) = (ReadyState::Later, Some(until));
            }
        }
        held.version += 1;
        held.mark_dirty(at);
        self.sync(job, cx);
        cx.notify();
        Ok(())
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    pub(in crate::job_view) fn card(wire: &str) -> ThreadReadyToTry {
        let base = r#""id":"try:native/chip@aaaa1111","at":"2026-11-15T14:02:00Z","offered_at":"2026-11-15T14:00:00Z","branch":"native/chip","tip":"aaaa1111","about":"Sketch boxes drawn as cards","in_build":true"#;
        let wire = format!("{{{base}{wire}}}");
        ipc::decode::<ThreadReadyToTry>("card", wire.as_bytes()).expect("a card decodes")
    }

    pub(in crate::job_view) fn thread_card(wire: &str) -> ThreadItem {
        ThreadItem::ReadyToTry(card(wire))
    }

    #[test]
    fn the_jobs_ready_to_try_row_names_its_newest_open_card_and_a_card_id_names_that_card() {
        let one = |id: &str, state: &str| ThreadItem::ReadyToTry(card(&format!(r#","state":"{state}""#)).tap_id(id));
        let items = [one("try:native/a@1111", "open"), one("try:native/b@2222", "open"), one("try:native/c@3333", "answered")];
        assert_eq!(card_named(&items, "try:job-1"), Some((1, "try:native/b@2222".into())), "the newest open card");
        assert_eq!(card_named(&items, "try:native/a@1111"), Some((0, "try:native/a@1111".into())));
        assert_eq!(card_named(&items, "try:native/z@9999"), None, "a card the thread lacks opens nothing");
        let settled = [one("try:native/a@1111", "answered"), one("try:native/c@3333", "withdrawn")];
        assert_eq!(card_named(&settled, "try:job-1"), Some((1, "try:native/c@3333".into())), "with none open, the newest card");
        assert_eq!(card_named(&[], "try:job-1"), None);
    }

    trait TapId {
        fn tap_id(self, id: &str) -> Self;
    }

    impl TapId for ThreadReadyToTry {
        fn tap_id(mut self, id: &str) -> Self {
            self.id = id.to_string();
            self
        }
    }

    #[test]
    fn a_card_with_an_approved_artifact_and_a_shot_draws_both_pictures_and_without_the_artifact_the_shot_alone() {
        let both = card(r#","state":"open","approved":{"artifact":"sketch:Chip","title":"Chip","form":"sketch","revision":2},"pictures":["/work/shots/chip.png"]"#);
        assert_eq!(Parts::of(&both).pictures, Pictures::Both);
        let alone = card(r#","state":"open","pictures":["/work/shots/chip.png"]"#);
        assert_eq!(Parts::of(&alone).pictures, Pictures::Built, "with nothing approved the built picture takes the row");
        let none = card(r#","state":"open""#);
        assert_eq!(Parts::of(&none).pictures, Pictures::None);
        let drawn_only = card(r#","state":"open","approved":{"artifact":"sketch:Chip","title":"Chip","form":"sketch","revision":1}"#);
        assert_eq!(Parts::of(&drawn_only).pictures, Pictures::Approved);
    }

    #[test]
    fn his_words_and_where_to_look_stand_only_where_the_card_has_them() {
        let bare = card(r#","state":"open""#);
        let parts = Parts::of(&bare);
        assert!(!parts.words && !parts.fold && parts.steps.is_empty());
        let blank = card(r#","state":"open","words":"  ","look":" ""#);
        assert!(!Parts::of(&blank).words && !Parts::of(&blank).fold, "words with nothing in them are not drawn");
        let full = card(r#","state":"open","words":"Make them richer.","look":"Open any sketch.","steps":[{"say":"Press Projects","act":"go_projects"},{"say":"Look at the toast"}]"#);
        let parts = Parts::of(&full);
        assert!(parts.words && parts.fold);
        assert_eq!(parts.steps.iter().map(|line| line.number).collect::<Vec<_>>(), [1, 2]);
        assert!(parts.steps[1].go.is_none(), "a step with no act has no key");
    }

    #[test]
    fn show_me_how_runs_the_first_act_the_registry_knows() {
        let acts = crate::generated::actions::ACTIONS.first().map(|row| row.id).expect("the registry holds acts");
        let with = card(&format!(r#","state":"open","steps":[{{"say":"Nowhere"}},{{"say":"Go","act":"{acts}"}},{{"say":"Never","act":"not_an_act"}}]"#));
        assert_eq!(Parts::of(&with).first_go().map(|go| go.act), Some(acts));
        let without = card(r#","state":"open","steps":[{"say":"Read it"},{"say":"Never","act":"not_an_act"}]"#);
        assert!(Parts::of(&without).first_go().is_none());
    }

    #[test]
    fn each_state_reads_as_the_standing_it_folds_to() {
        let words = |state: &str, extra: &str| Standing::of(&card(&format!(r#","state":"{state}"{extra}"#)), None);
        assert_eq!(words("open", ""), Standing::Open);
        assert_eq!(words("later", r#","later_until":"2026-11-16T09:00:00Z""#), Standing::Later);
        assert_eq!(words("answered", r#","verdict":"good""#), Standing::Good);
        assert_eq!(words("answered", r#","verdict":"needs_work""#), Standing::Rework);
        assert_eq!(words("withdrawn", ""), Standing::Withdrawn);
        assert_eq!(words("replaced", ""), Standing::Replaced);
        let said: Vec<&str> = [Standing::Good, Standing::Rework, Standing::Withdrawn, Standing::Replaced].iter().map(|one| one.word()).collect();
        assert_eq!(said, ["Looks good", "Needs rework", "Withdrawn", "Replaced by a newer build"]);
        assert!([Standing::Good, Standing::Rework, Standing::Withdrawn, Standing::Replaced].iter().all(|one| one.folds()));
        assert!(Standing::Open.asks() && Standing::Later.asks());
    }

    #[test]
    fn what_he_pressed_stands_over_what_fleet_last_sent() {
        let open = card(r#","state":"open""#);
        assert_eq!(Standing::of(&open, Some(&Answer::Approved)), Standing::Good);
        assert_eq!(Standing::of(&open, Some(&Answer::SentBack("More".into()))), Standing::Rework);
        assert_eq!(Standing::of(&open, Some(&Answer::Later("2026-11-16T09:00:00Z".into()))), Standing::Later);
        assert_eq!(Standing::of(&open, Some(&Answer::Skipped)), Standing::Skipped);
    }

    /// A build he skipped is one line like one he approved, and says Skipped where that says Looks good.
    #[test]
    fn a_skipped_build_reads_skipped_and_folds() {
        assert_eq!(Standing::Skipped.word(), "Skipped");
        assert_ne!(Standing::Skipped.word(), Standing::Good.word());
        assert!(Standing::Skipped.folds() && !Standing::Skipped.asks());
    }

    #[test]
    fn a_card_fleet_kept_as_skipped_reads_skipped_when_read_again() {
        assert_eq!(Standing::of(&card(r#","state":"answered","verdict":"good","skipped":true"#), None), Standing::Skipped);
        assert_eq!(Standing::of(&card(r#","state":"answered","verdict":"good""#), None), Standing::Good);
    }

    #[test]
    fn two_pictures_stand_one_to_a_row_when_each_would_be_under_520_points() {
        assert!(!stacked(1060., 12., PICTURE_LEAST), "524 across each");
        assert!(stacked(1040., 12., PICTURE_LEAST), "514 across each");
    }

    #[test]
    fn an_answer_goes_with_the_job_the_branch_and_the_tip_it_is_for() {
        let held = card(r#","state":"open""#);
        let body = answer_body("job-1", &held, ReadyAnswer::NeedsRework { words: "Darker edge".into() }).expect("it encodes");
        let said: serde_json::Value = serde_json::from_str(&body).expect("it is json");
        assert_eq!(said["session_id"], "job-1");
        assert_eq!(said["branch"], "native/chip");
        assert_eq!(said["tip"], "aaaa1111");
        assert_eq!(said["answer"], serde_json::json!({"kind": "needs_rework", "words": "Darker edge"}));
        let good: serde_json::Value = serde_json::from_str(&answer_body("job-1", &held, ReadyAnswer::LooksGood).expect("it encodes")).expect("json");
        assert_eq!(good["answer"], serde_json::json!({"kind": "looks_good"}));
        let skip: serde_json::Value = ipc::decode("skip", answer_body("job-1", &held, ReadyAnswer::Skip).expect("it encodes").as_bytes()).expect("json");
        assert_eq!(skip["answer"], serde_json::json!({"kind": "skip"}));
        assert_eq!((skip["branch"].as_str(), skip["tip"].as_str()), (Some("native/chip"), Some("aaaa1111")));
        let remind: serde_json::Value = serde_json::from_str(&answer_body("job-1", &held, ReadyAnswer::Remind { until: Instant::carried("2026-11-16T09:00:00+00:00") }).expect("it encodes")).expect("json");
        assert_eq!(remind["answer"], serde_json::json!({"kind": "remind", "until": "2026-11-16T09:00:00+00:00"}));
    }
}
