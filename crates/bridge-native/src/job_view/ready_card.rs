//! The Ready to try card, in the Pictures first take the owner picked on 15 Nov 2026: a green head band, the title, what he
//! approved beside what was built, his words, a fold holding where to look, and a foot of four answers.
//!
//! **The card is the build note of its branch at its tip, seen from the Job's thread**, so Fleet carries its state
//! (`ThreadReadyToTry`) and the thread moves when he answers. A press answers at once, as every card does (`acts`), and a
//! refusal takes the answer back and says Fleet's sentence on the card's own row. One he has answered, or the Job has
//! withdrawn or replaced, is one line that opens on press; one put off is one quiet line with its wake time.

use std::rc::Rc;

use chrono::{DateTime, Local};
use gpui::{canvas, div, prelude::*, px, size, AnyElement, Context, FontWeight, Hsla, KeyDownEvent, Rgba, Role, SharedString, Window};
use ipc::{AnswerReadyToTry, ArtifactForm, Instant, NoteVerdict, ReadyAnswer, ReadyApproved, ReadyState, SentFile, SessionId, ThreadItem, ThreadReadyToTry};

use super::acts::{kbd, refusal, Answer, Mode};
use super::artifacts_tab::{Picture, PictureKey};
use super::band_card::{foot_box, frame, kind_look, shapes, tile, well, Act, Do, Foot};
use super::cards::{quote, shown_scene};
use super::rows::{chevron, glyph, mono, tip, Size};
use super::sketch_card::{act, mixed, Kind};
use super::sketch_read::newest_card;
use super::{JobView, Page};
use crate::build_notes::{step_lines, StepLine};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::scene::{Clock, Fit, Lens};

/// Whether an id names a Ready to try card (`try:<branch>@<tip>`) and not an artifact.
pub(super) fn is_card(id: &str) -> bool {
    id.starts_with("try:")
}

/// The card an id names in a thread, with where it stands. A card's own id (`try:<branch>@<tip>`) names that card; the Job's
/// one Needs you row (`try:<session>`, no `@`) names its newest open card, else its newest card.
fn card_named(items: &[ThreadItem], id: &str) -> Option<(usize, String)> {
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

/// Each picture is at least this many points across, or the two stand one to a row.
const PICTURE_LEAST: f32 = 300.;

/// Where a card stands: the colour of its band and the words on its chip.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Standing {
    Open,
    Later,
    Good,
    Rework,
    Withdrawn,
    Replaced,
}

impl Standing {
    /// What the card is as Fleet sent it, or as he just answered it before Fleet's row says so.
    pub(super) fn of(card: &ThreadReadyToTry, pressed: Option<&Answer>) -> Self {
        match pressed {
            Some(Answer::Approved) => return Standing::Good,
            Some(Answer::SentBack(_)) => return Standing::Rework,
            Some(Answer::Later(_)) => return Standing::Later,
            _ => {}
        }
        match card.state {
            ReadyState::Open => Standing::Open,
            ReadyState::Later => Standing::Later,
            ReadyState::Answered if card.verdict == Some(NoteVerdict::NeedsWork) => Standing::Rework,
            ReadyState::Answered => Standing::Good,
            ReadyState::Withdrawn => Standing::Withdrawn,
            ReadyState::Replaced => Standing::Replaced,
        }
    }

    pub(super) fn word(self) -> &'static str {
        match self {
            Standing::Open | Standing::Later => "Ready to try",
            Standing::Good => "Looks good",
            Standing::Rework => "Needs rework",
            Standing::Withdrawn => "Withdrawn",
            Standing::Replaced => "Replaced by a newer build",
        }
    }

    fn ink(self) -> Token {
        match self {
            Standing::Open => token::STATUS_COMPLETED_SUCCESS,
            Standing::Later | Standing::Replaced => token::STATUS_NOT_STARTED,
            Standing::Good => token::STATUS_COMPLETED_SUCCESS,
            Standing::Rework => token::STATUS_REJECTED,
            Standing::Withdrawn => token::STATUS_KILLED,
        }
    }

    fn tip(self) -> &'static str {
        match self {
            Standing::Open => "Waiting on you to try it",
            Standing::Later => "You asked to be reminded",
            Standing::Good => "You said it looks good",
            Standing::Rework => "You sent it back for rework",
            Standing::Withdrawn => "The Job took the offer back",
            Standing::Replaced => "The Job offered the branch again at a newer tip",
        }
    }

    /// A card with nothing left to ask is one line.
    pub(super) fn folds(self) -> bool {
        matches!(self, Standing::Good | Standing::Rework | Standing::Withdrawn | Standing::Replaced)
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
    /// The fold holds the look sentence, the steps, or both.
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

    /// The act Show me how runs: the first step that has one the registry knows. None where no step has, and it opens the fold.
    pub(super) fn first_go(&self) -> Option<&crate::build_notes::Go> {
        self.steps.iter().find_map(|line| line.go.as_ref())
    }
}

/// Whether the two pictures stand one to a row: each would be under [`PICTURE_LEAST`] points across.
pub(super) fn stacked(inner: f32, gap: f32) -> bool {
    (inner - gap) / 2. < PICTURE_LEAST
}

/// The body of his answer, as it goes to Fleet.
pub(super) fn answer_body(job: &str, card: &ThreadReadyToTry, answer: ReadyAnswer) -> Result<String, String> {
    ipc::encode(&AnswerReadyToTry { session_id: SessionId::carried(job.to_string()), branch: card.branch.clone(), tip: card.tip.clone(), answer }).map_err(|why| format!("{why:?}"))
}

/// The time a card shows for a stamp: the clock alone today, the day before it on any other.
fn clock(at: &Instant, now: DateTime<Local>) -> String {
    let Some(at) = super::acts::at_of(at) else { return String::new() };
    let time = crate::settings::time::time_of(at, ipc::time_format::TimeFormat { seconds: false, ..crate::settings::time::held() });
    if at.date_naive() == now.date_naive() {
        time
    } else {
        format!("{} {time}", at.format("%-d %b"))
    }
}

/// Two colours blended `at` of the way from `from` to `to`.
fn blend(from: Hsla, to: Hsla, at: f32) -> Hsla {
    let (a, b) = (Rgba::from(from), Rgba::from(to));
    let by = |x: f32, y: f32| x + (y - x) * at;
    Rgba { r: by(a.r, b.r), g: by(a.g, b.g), b: by(a.b, b.b), a: 1. }.into()
}

/// How tall a picture's box is: four of the card's 48 px steps.
fn picture_height(look: &Look) -> gpui::Pixels {
    look.length(token::SPACE_12) * 4.
}

/// A quiet 2xs label in a muted ink, for the head of a picture.
fn small(look: &Look) -> gpui::Div {
    div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
}

impl JobView {
    /// The card in the thread (`ThreadReadyToTry`).
    pub(super) fn ready_card(&self, ix: usize, card: &ThreadReadyToTry, page: &Page, look: &Look, now: DateTime<Local>, cx: &Context<Self>) -> AnyElement {
        let key = card.id.clone();
        let standing = Standing::of(card, page.cards.answers.get(&key));
        let parts = Parts::of(card);
        let probe = self.go_probe(&key, page, cx);
        match standing {
            Standing::Later => {
                let until = match page.cards.answers.get(&key) {
                    Some(Answer::Later(until)) => Some(Instant::carried(until.clone())),
                    _ => card.later_until.clone(),
                };
                return self.ready_later(ix, card, until.as_ref(), probe, page, look, now, cx);
            }
            Standing::Good | Standing::Rework | Standing::Withdrawn | Standing::Replaced if !self.open_in_card(&format!("{key}:open")) => {
                return self.ready_folded(ix, card, standing, probe, page, look, now, cx);
            }
            _ => {}
        }
        let open = standing == Standing::Open;
        let job = self.shown.clone().unwrap_or_default();
        let run = self.ready_run(ix, card, parts.clone());
        let foot = Foot { ix, key: key.clone(), asking: open, archivable: false, openable: false, editable: false, commit: true, route: ANSWER_READY, run: run.clone() };
        let narrow = self.band_narrow(look);
        let ink = look.color(standing.ink());
        let spot = page.keys.spot(&format!("card:{key}"), cx);
        let keys = run;
        let id = key.clone();
        let head = self.ready_head(ix, card, standing, now, look, cx);
        let title = div()
            .id(SharedString::from(format!("ready-title:{key}")))
            .px(look.length(token::SPACE_4))
            .pt(look.length(token::SPACE_3))
            .text_size(look.length(token::TEXT_BASE))
            .line_height(look.length(token::LEADING_BASE))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(look.color(token::FG_DEFAULT))
            .child(SharedString::from(card.about.clone()));
        let pictures = self.ready_pictures(&job, card, &parts, narrow, page, look, cx);
        let words = card.words.as_deref().filter(|words| !words.trim().is_empty()).map(|words| {
            div().px(look.length(token::SPACE_4)).pt(look.length(token::SPACE_3)).child(quote(look, &format!("\u{201c}{}\u{201d}", words.trim())))
        });
        let fold = parts.fold.then(|| self.ready_fold(ix, card, &parts, page, look, cx));
        let foot_row = open.then(|| foot_box(look).child(self.ready_footer(&foot, card, &parts, page, look, cx)));
        let mut frame = frame(look, narrow)
            .id(SharedString::from(format!("ready:{key}")))
            .border_color(ink.opacity(0.45))
            .track_focus(&spot.focus)
            .in_focus(|style| style.border_color(look.color(token::ACCENT)))
            .relative()
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.modifiers.modified() || !open {
                    return;
                }
                let noting = this.shown.as_ref().and_then(|job| this.pages.get(job)).is_some_and(|page| page.cards.modes.contains_key(&id));
                if noting {
                    return;
                }
                let act = match event.keystroke.key.as_str() {
                    "1" => Act::Approve,
                    "2" => Act::Ask,
                    "3" => Act::Open,
                    _ => return,
                };
                cx.stop_propagation();
                keys(this, act, window, cx);
            }))
            .child(head)
            .child(title)
            .children(pictures)
            .children(words)
            .children(fold)
            .children(foot_row)
            .children(probe);
        // Pulsed once by the press on its Needs you item.
        let pulse = page.cards.pulse.as_ref().filter(|(pulsed, _)| *pulsed == key).map(|(_, n)| *n);
        let name = format!("Ready to try: {}", card.about);
        match pulse {
            Some(n) => {
                use gpui::AnimationExt as _;
                let (base, accent, raised) = (ink.opacity(0.45), look.color(token::ACCENT), look.color(token::BG_RAISED));
                frame = frame.border_color(base);
                let pulsing = frame.with_animation(
                    SharedString::from(format!("ready-pulse:{key}:{n}")),
                    gpui::Animation::new(look.duration(token::DURATION_PULSE)).with_easing(gpui::ease_out_quint()),
                    move |frame, at| frame.border_color(blend(base, accent, 1. - at)).bg(blend(raised, accent, 0.14 * (1. - at))),
                );
                labelled(div().id(SharedString::from(format!("ready-pulse-wrap:{key}"))).w_full().child(pulsing), Role::Article, name).into_any_element()
            }
            None => labelled(frame, Role::Article, name).into_any_element(),
        }
    }

    /// What each press on the card does. Looks good answers, Needs rework opens its box and Send sends it, Show me how runs the
    /// first step's act or opens the fold.
    fn ready_run(&self, ix: usize, card: &ThreadReadyToTry, parts: Parts) -> Do {
        let (key, card) = (card.id.clone(), card.clone());
        Rc::new(move |this, act, window, cx| match act {
            Act::Approve => this.answer_ready(ix, &card, ReadyAnswer::LooksGood, window, cx),
            Act::Ask => this.ask_for_changes(ix, &key, window, cx),
            Act::Send => {
                let words = this.typed_in(&key, Mode::Note, cx).trim().to_string();
                if !words.is_empty() {
                    this.answer_ready(ix, &card, ReadyAnswer::NeedsRework { words }, window, cx);
                }
            }
            Act::Cancel => this.shut_band_note(ix, &key, window, cx),
            Act::Open => match parts.first_go() {
                Some(go) => window.dispatch_action(Box::new(crate::keymap::Act { id: go.act, slot: 0, digit: go.digit }), cx),
                None if parts.fold => {
                    if !this.open_in_card(&format!("{key}:fold")) {
                        this.flip_in_card(ix, &format!("{key}:fold"), cx);
                    }
                }
                None => {}
            },
            Act::Dismiss | Act::Archive | Act::Edit => {}
        })
    }

    /// His answer: the card reads it in this frame, Fleet is told off the main thread, and a refusal takes it back with Fleet's
    /// sentence on the card.
    pub(super) fn answer_ready(&mut self, ix: usize, card: &ThreadReadyToTry, answer: ReadyAnswer, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let pressed = match &answer {
            ReadyAnswer::LooksGood => Answer::Approved,
            ReadyAnswer::NeedsRework { words } => Answer::SentBack(words.clone()),
            ReadyAnswer::Remind { until } => Answer::Later(until.as_str().to_string()),
        };
        let body = answer_body(&job, card, answer);
        self.settle(ix, card.id.clone(), pressed, ANSWER_READY, body, window, cx);
    }

    /// The head band: the state's chip, the Job, and the time at the right in mono. A card he opened again folds on a press here.
    fn ready_head(&self, ix: usize, card: &ThreadReadyToTry, standing: Standing, now: DateTime<Local>, look: &Look, cx: &Context<Self>) -> AnyElement {
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
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(ink)
                .tooltip(tip(standing.tip()))
                .child(gpui::svg().path(crate::assets::icon_path("circle-dot")).flex_none().size(look.length(token::TEXT_2XS)).text_color(ink))
                .child(standing.word()),
            Role::Status,
            standing.word(),
        );
        let job = self.shown.as_ref().and_then(|job| self.title_of(job, cx));
        let again = !matches!(standing, Standing::Open);
        let open_key = format!("{key}:open");
        let line = div()
            .id(SharedString::from(format!("ready-head:{key}")))
            .flex()
            .items_center()
            .min_w_0()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .rounded_t(look.length(token::RADIUS_LG) * 1.5 - look.length(token::BORDER_WIDTH))
            .bg(mixed(look, standing.ink(), 0.18))
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
                    .when_some(job.clone(), |one, job| one.tooltip(tip(job.clone())).child(job)),
            )
            .child(
                div()
                    .id(SharedString::from(format!("ready-time:{key}")))
                    .flex_none()
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .text_color(look.color(token::FG_MUTED))
                    .tooltip(tip(format!("Offered {}", clock(&card.offered_at, now))))
                    .child(SharedString::from(clock(&card.at, now))),
            )
            .when(again, |line| line.cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.flip_in_card(ix, &open_key, cx))));
        line.into_any_element()
    }

    /// The pictures row: what he approved beside what was built, or the built one across the whole row, or nothing.
    #[allow(clippy::too_many_arguments)]
    fn ready_pictures(&self, job: &str, card: &ThreadReadyToTry, parts: &Parts, narrow: bool, page: &Page, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        if parts.pictures == Pictures::None {
            return None;
        }
        let gap = look.length(token::SPACE_3);
        let pane = self.pane.unwrap_or(px(720.));
        let wide = if narrow { look.length(token::SPACE_4) + look.length(token::SPACE_1) } else { px(0.) };
        let inner = f32::from(pane - look.length(token::SPACE_4) * 2. + wide * 2. - look.length(token::SPACE_4) * 2. - look.length(token::BORDER_WIDTH) * 2.);
        let both = parts.pictures == Pictures::Both;
        let stack = both && stacked(inner, f32::from(gap));
        let each = if both && !stack { (inner - f32::from(gap)) / 2. } else { inner };
        let approved = card.approved.as_ref().filter(|_| matches!(parts.pictures, Pictures::Both | Pictures::Approved)).map(|approved| self.ready_approved(job, card, approved, each, page, look, cx));
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
    fn ready_frame(&self, id: String, label: SharedString, ink: Token, body: AnyElement, look: &Look) -> gpui::Stateful<gpui::Div> {
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
                    .child(label),
            )
            .child(div().flex().items_center().justify_center().h(picture_height(look)).p(look.length(token::SPACE_2)).child(body))
    }

    /// The picture the Job built, which a press opens large over the window.
    fn ready_built(&self, card: &ThreadReadyToTry, path: &str, each: f32, look: &Look, cx: &Context<Self>) -> AnyElement {
        let file = super::pictures::read_picture(path).unwrap_or_else(|| SentFile { id: path.to_string(), name: path.rsplit('/').next().unwrap_or(path).to_string(), media_type: String::new() });
        let room = size((each - f32::from(look.length(token::SPACE_2)) * 2. - f32::from(look.length(token::BORDER_WIDTH)) * 2.).max(120.), f32::from(picture_height(look) - look.length(token::SPACE_2) * 2.));
        let body = self.picture_in(&file, room, look, cx).unwrap_or_else(|| {
            div().text_size(look.length(token::TEXT_XS)).font_family(mono(look)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(file.name.clone())).into_any_element()
        });
        let label = if card.in_build { "Built \u{b7} in the preview" } else { "Built" };
        self.ready_frame(format!("ready-built:{}", card.id), label.into(), token::STATUS_COMPLETED_SUCCESS, body, look).into_any_element()
    }

    /// What he approved, at the revision he approved, drawn as the card of its kind draws it. A press reads it large in its own tab.
    #[allow(clippy::too_many_arguments)]
    fn ready_approved(&self, job: &str, card: &ThreadReadyToTry, approved: &ReadyApproved, each: f32, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let _ = page;
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
            self.ready_frame(format!("ready-approved:{}", card.id), label.into(), token::YOU, body, look)
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

    /// The fold: where to look and the steps, shut until he opens it.
    fn ready_fold(&self, ix: usize, card: &ThreadReadyToTry, parts: &Parts, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let _ = page;
        let held = format!("{}:fold", card.id);
        let open = self.open_in_card(&held);
        let toggled = held.clone();
        let head = labelled(
            div()
                .id(SharedString::from(format!("ready-fold:{}", card.id)))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_SM))
                .cursor_pointer()
                .tab_index(0)
                .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(look.color(token::FG_MUTED))
                .hover(|style| style.text_color(look.color(token::FG_DEFAULT)))
                .child(chevron(look, open))
                .child("What changed and where to look")
                .on_click(cx.listener(move |this, _, _, cx| this.flip_in_card(ix, &toggled, cx))),
            Role::Button,
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
            .child(head)
            .when(open, |fold| {
                fold.children(look_line).children((!steps.is_empty()).then(|| {
                    labelled(div().id(SharedString::from(format!("ready-steps:{}", card.id))).flex().flex_col().gap(look.length(token::SPACE_2)).children(steps), Role::List, "Steps")
                }))
            })
            .into_any_element()
    }

    /// The foot: Looks good, Needs rework and Show me how, a spacer, then Remind me. Needs rework opens its box in their place.
    fn ready_footer(&self, foot: &Foot, card: &ThreadReadyToTry, parts: &Parts, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let key = foot.key.as_str();
        let mut column = div().flex().flex_col().gap(look.length(token::SPACE_2));
        if !page.cards.modes.contains_key(key) && self.holds_kept(key) {
            page.cards.kept_seen.borrow_mut().push((key.to_string(), foot.ix, true));
        }
        if let Some(why) = page.cards.said.get(key) {
            column = column.child(refusal(look, key, why.clone()));
        }
        if page.cards.modes.get(key) == Some(&Mode::Note) {
            return column.child(self.band_note(foot, page, look, cx)).into_any_element();
        }
        let (good, rework, how) = (foot.run.clone(), foot.run.clone(), foot.run.clone());
        let button = |id: &str, said: &'static str, kind: Kind, cap: &str, on: bool| act(look, format!("{id}:{key}"), said, kind, cap, on).tab_index(0).focus_visible(|style| style.border_color(look.color(token::ACCENT)));
        let remind = page.keys.spot(&format!("remind:{key}"), cx);
        let drawn = remind.bounds.clone();
        let (ix, held) = (foot.ix, card.clone());
        let shows = parts.fold || parts.first_go().is_some();
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
            .child(div().flex_1())
            .child(labelled(
                button("remind", "Remind me", Kind::Ghost, "", true)
                    .relative()
                    .child(glyph(look, "chevron-down", Size::Small, look.color(token::FG_MUTED)))
                    .child(canvas(move |bounds, _, _| drawn.set(bounds), |_, _, _, _| {}).absolute().top_0().left_0().size_full())
                    .tooltip(tip("Bring this card back later"))
                    .on_click(cx.listener(move |this, _, window, cx| this.open_remind(ix, &held, window, cx))),
                Role::Button,
                "Remind me",
            ));
        column.child(super::acts::gated(row, self.off_shown_to(ANSWER_READY, cx))).into_any_element()
    }

    /// Remind me's menu under its button: each choice of the Later ladder with the time it brings the card back.
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

    /// A card he answered, or the Job withdrew or replaced, in one line: where it stands, its title and the time. A press opens it.
    #[allow(clippy::too_many_arguments)]
    fn ready_folded(&self, ix: usize, card: &ThreadReadyToTry, standing: Standing, probe: Option<AnyElement>, page: &Page, look: &Look, now: DateTime<Local>, cx: &Context<Self>) -> AnyElement {
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
        let spot = page.keys.spot(&format!("card:{key}"), cx);
        let held = format!("{key}:open");
        let keyed = held.clone();
        let line = self
            .quiet_base(&format!("ready:{key}"), "circle-dot", vec![chip, about, time], look)
            .track_focus(&spot.focus)
            .in_focus(|style| style.border_color(look.color(token::ACCENT)))
            .relative()
            .cursor_pointer()
            .hover(|style| style.border_color(look.color(token::BORDER_STRONG)))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.modifiers.modified() || !matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    return;
                }
                cx.stop_propagation();
                this.flip_in_card(ix, &keyed, cx);
            }))
            .on_click(cx.listener(move |this, _, _, cx| this.flip_in_card(ix, &held, cx)))
            .children(probe);
        labelled(line, Role::Article, format!("Ready to try: {}, {}", card.about, standing.word())).into_any_element()
    }

    /// A card put off: its title and when it comes back, on one quiet line.
    #[allow(clippy::too_many_arguments)]
    fn ready_later(&self, _ix: usize, card: &ThreadReadyToTry, until: Option<&Instant>, probe: Option<AnyElement>, page: &Page, look: &Look, now: DateTime<Local>, cx: &Context<Self>) -> AnyElement {
        let key = card.id.clone();
        let back = until
            .and_then(super::acts::at_of)
            .map(|at| format!("Back {}", crate::operator::face::back_at(at, now)))
            .unwrap_or_else(|| "Back later".to_string());
        let parts = vec![
            div().id(SharedString::from(format!("ready-about:{key}"))).flex_1().min_w_0().truncate().text_color(look.color(token::FG_MUTED)).child(SharedString::from(card.about.clone())).into_any_element(),
            div()
                .id(SharedString::from(format!("ready-back:{key}")))
                .flex_none()
                .font_family(mono(look))
                .text_size(look.length(token::TEXT_2XS))
                .text_color(look.color(token::FG_MUTED))
                .tooltip(tip(Standing::Later.tip()))
                .child(SharedString::from(back.clone()))
                .into_any_element(),
        ];
        let spot = page.keys.spot(&format!("card:{key}"), cx);
        let line = self.quiet_base(&format!("ready-later:{key}"), "clock", parts, look).track_focus(&spot.focus).in_focus(|style| style.border_color(look.color(token::ACCENT))).relative().children(probe);
        labelled(line, Role::Article, format!("Ready to try: {}, {back}", card.about)).into_any_element()
    }

    // ---- going to a card from Needs you -----------------------------------------------------------

    /// The card an item in Needs you names: scrolled to, given the keys, and pulsed once as its draw reaches it. A Job whose thread
    /// holds no such card opens nothing.
    pub(super) fn go_to_ready(&mut self, job: &str, id: &str, _window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some((at, id)) = self.thread(job, cx).and_then(|held| card_named(&held.items, id)) else { return false };
        let Some(page) = self.pages.get_mut(job) else { return false };
        *page.cards.go.borrow_mut() = Some(id);
        page.list.scroll_to_reveal_item(page.entry(at));
        cx.notify();
        true
    }

    /// Where the card draws, once, as its draw reaches it: a canvas that notices the request and defers its answer off the draw.
    fn go_probe(&self, key: &str, page: &Page, cx: &Context<Self>) -> Option<AnyElement> {
        if page.cards.go.borrow().as_deref() != Some(key) {
            return None;
        }
        let (me, key) = (cx.entity().downgrade(), key.to_string());
        Some(
            canvas(
                move |_, window, cx| {
                    let (me, key) = (me.clone(), key.clone());
                    window.defer(cx, move |window, cx| {
                        let _ = me.update(cx, |this, cx| this.arrived_at_ready(&key, window, cx));
                    });
                },
                |_, _, _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .into_any_element(),
        )
    }

    fn arrived_at_ready(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let Some(page) = self.pages.get_mut(&job) else { return };
        if page.cards.go.borrow_mut().take().as_deref() != Some(key) {
            return;
        }
        let focus = page.keys.spot(&format!("card:{key}"), cx).focus;
        page.cards.pulses += 1;
        let n = page.cards.pulses;
        page.cards.pulse = Some((key.to_string(), n));
        window.focus(&focus, cx);
        let (held, wait) = (key.to_string(), cx.look().duration(token::DURATION_PULSE));
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(wait + std::time::Duration::from_millis(100)).await;
            let _ = this.update(cx, |this, cx| {
                if let Some(page) = this.pages.get_mut(&job) {
                    if page.cards.pulse == Some((held, n)) {
                        page.cards.pulse = None;
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
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
            ReadyAnswer::LooksGood => (card.state, card.verdict, card.later_until) = (ReadyState::Answered, Some(NoteVerdict::Good), None),
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
mod tests {
    use super::*;

    pub(super) fn card(wire: &str) -> ThreadReadyToTry {
        let base = r#""id":"try:native/chip@aaaa1111","at":"2026-11-15T14:02:00Z","offered_at":"2026-11-15T14:00:00Z","branch":"native/chip","tip":"aaaa1111","about":"Sketch boxes drawn as cards","in_build":true"#;
        let wire = format!("{{{base}{wire}}}");
        ipc::decode::<ThreadReadyToTry>("card", wire.as_bytes()).expect("a card decodes")
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
    fn his_words_and_the_fold_stand_only_where_the_card_has_them() {
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
    fn show_me_how_runs_the_first_act_the_registry_knows_and_otherwise_opens_the_fold() {
        let acts = crate::generated::actions::ACTIONS.first().map(|row| row.id).expect("the registry holds acts");
        let with = card(&format!(r#","state":"open","steps":[{{"say":"Nowhere"}},{{"say":"Go","act":"{acts}"}},{{"say":"Never","act":"not_an_act"}}]"#));
        assert_eq!(Parts::of(&with).first_go().map(|go| go.act), Some(acts));
        let without = card(r#","state":"open","steps":[{"say":"Read it"},{"say":"Never","act":"not_an_act"}]"#);
        assert!(Parts::of(&without).first_go().is_none());
    }

    #[test]
    fn each_state_reads_as_the_one_line_it_folds_to() {
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
        assert!(!Standing::Open.folds() && !Standing::Later.folds());
    }

    #[test]
    fn what_he_pressed_stands_over_what_fleet_last_sent() {
        let open = card(r#","state":"open""#);
        assert_eq!(Standing::of(&open, Some(&Answer::Approved)), Standing::Good);
        assert_eq!(Standing::of(&open, Some(&Answer::SentBack("More".into()))), Standing::Rework);
        assert_eq!(Standing::of(&open, Some(&Answer::Later("2026-11-16T09:00:00Z".into()))), Standing::Later);
    }

    #[test]
    fn two_pictures_stand_one_to_a_row_when_each_would_be_under_300_points() {
        assert!(!stacked(640., 12.), "314 across each");
        assert!(stacked(600., 12.), "294 across each");
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
        let remind: serde_json::Value = serde_json::from_str(&answer_body("job-1", &held, ReadyAnswer::Remind { until: Instant::carried("2026-11-16T09:00:00+00:00") }).expect("it encodes")).expect("json");
        assert_eq!(remind["answer"], serde_json::json!({"kind": "remind", "until": "2026-11-16T09:00:00+00:00"}));
    }

    // ---- drawn, on the mock Fleet's `ready-to-try` scenario ---------------------------------------

    use super::super::parts_fold::{drawn, drawn_like, redraw};
    use gpui::Role;

    /// The scenario's thread for `job` on screen in a window tall enough to draw all of it.
    fn on_screen<'a>(cx: &'a mut gpui::TestAppContext, job: &str) -> (gpui::Entity<JobView>, &'a mut gpui::VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let jobs = crate::mock::jobs("ready-to-try").expect("the fixture decodes");
        let threads = crate::mock::threads("ready-to-try").expect("the threads decode");
        let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
        let (view, cx) = cx.add_window_view({
            let list = list.clone();
            move |window, cx| JobView::new(list, Some(threads), window, cx)
        });
        cx.simulate_resize(gpui::size(gpui::px(1400.), gpui::px(6000.)));
        let job = job.to_string();
        list.update(cx, |list, cx| list.open(job.into(), cx));
        redraw(cx);
        (view, cx)
    }

    /// Where a card with this title stands in the shown Job's thread, and the card.
    fn card_titled(view: &gpui::Entity<JobView>, about: &str, cx: &mut gpui::VisualTestContext) -> (usize, ThreadReadyToTry) {
        view.read_with(cx, |view, cx| {
            let job = view.shown.clone().expect("a Job is shown");
            let held = view.thread(&job, cx).expect("it holds a thread");
            held.items
                .iter()
                .enumerate()
                .find_map(|(ix, item)| match item {
                    ThreadItem::ReadyToTry(card) if card.about == about => Some((ix, card.clone())),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("the thread holds a card titled {about}"))
        })
    }

    const FULL: &str = "Sketch boxes drawn as cards";

    #[gpui::test]
    fn a_card_draws_its_title_pictures_words_fold_and_the_four_answers(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit");
        let _ = view;
        assert!(drawn(cx, Role::Article, &format!("Ready to try: {FULL}")));
        assert!(drawn(cx, Role::Button, "You approved: Sketch, Merge line flow"), "what he approved, under its label");
        assert!(drawn(cx, Role::Button, "sketch-cards.png"), "what was built");
        assert!(drawn(cx, Role::Button, "What changed and where to look"));
        for name in ["Looks good", "Needs rework", "Show me how", "Remind me"] {
            assert!(drawn(cx, Role::Button, name), "{name} is on the card");
        }
    }

    #[gpui::test]
    fn a_card_with_no_approved_artifact_gives_its_one_picture_the_whole_row(cx: &mut gpui::TestAppContext) {
        let (_view, cx) = on_screen(cx, "tool-search");
        assert!(drawn(cx, Role::Article, "Ready to try: Search finds a Job by its branch"));
        assert!(drawn(cx, Role::Button, "queue-search.png"));
        assert!(drawn_like(cx, Role::Button, "You approved").is_empty(), "nothing approved, so no second picture");
    }

    #[gpui::test]
    fn a_card_with_no_picture_draws_no_pictures(cx: &mut gpui::TestAppContext) {
        let (_view, cx) = on_screen(cx, "tool-agent");
        assert!(drawn(cx, Role::Article, "Ready to try: The reviewer's brief names its files"));
        assert!(drawn_like(cx, Role::Button, "You approved").is_empty());
        assert!(!drawn(cx, Role::Button, "queue-search.png"));
    }

    #[gpui::test]
    fn a_card_answered_withdrawn_replaced_or_put_off_folds_to_one_line_each(cx: &mut gpui::TestAppContext) {
        let (_view, cx) = on_screen(cx, "tool-done");
        for line in [
            "Ready to try: Chip takes the stale colour, Looks good",
            "Ready to try: Clock sits under the chip, Needs rework",
            "Ready to try: Arrows with words, Withdrawn",
            "Ready to try: Rail layout for the Jobs list, Replaced by a newer build",
        ] {
            assert!(drawn(cx, Role::Article, line), "{line} stands as a line");
        }
        assert!(!drawn_like(cx, Role::Article, "Ready to try: Toast pile drawn as a stack, Back ").is_empty(), "a card put off says when it comes back");
        for gone in ["Looks good", "Needs rework", "Show me how", "Remind me"] {
            assert!(!drawn(cx, Role::Button, gone), "a settled card has no {gone} button");
        }
    }

    #[gpui::test]
    fn a_line_opens_on_a_press_to_the_whole_card(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-done");
        let (ix, card) = card_titled(&view, "Chip takes the stale colour", cx);
        view.update(cx, |view, cx| view.flip_in_card(ix, &format!("{}:open", card.id), cx));
        redraw(cx);
        assert!(drawn(cx, Role::Article, "Ready to try: Chip takes the stale colour"), "the opened card carries no state word in its name");
    }

    #[gpui::test]
    fn looks_good_posts_the_branch_and_tip_and_the_card_folds_at_once(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit");
        let (ix, card) = card_titled(&view, FULL, cx);
        view.update_in(cx, |view, window, cx| view.answer_ready(ix, &card, ReadyAnswer::LooksGood, window, cx));
        redraw(cx);
        view.read_with(cx, |view, _| {
            let sent = view.mock_ready.last().expect("an answer went to Fleet");
            assert_eq!((sent.branch.as_str(), sent.tip.as_str()), (card.branch.as_str(), card.tip.as_str()));
            assert!(matches!(sent.answer, ReadyAnswer::LooksGood));
        });
        assert!(drawn(cx, Role::Article, &format!("Ready to try: {FULL}, Looks good")));
    }

    #[gpui::test]
    fn needs_rework_posts_his_words_and_folds_the_card(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-search");
        let (ix, card) = card_titled(&view, "Search finds a Job by its branch", cx);
        view.update_in(cx, |view, window, cx| view.answer_ready(ix, &card, ReadyAnswer::NeedsRework { words: "Match the tip too".into() }, window, cx));
        redraw(cx);
        view.read_with(cx, |view, _| match &view.mock_ready.last().expect("an answer went to Fleet").answer {
            ReadyAnswer::NeedsRework { words } => assert_eq!(words, "Match the tip too"),
            other => panic!("expected needs_rework, sent {other:?}"),
        });
        assert!(drawn(cx, Role::Article, "Ready to try: Search finds a Job by its branch, Needs rework"));
    }

    #[gpui::test]
    fn remind_me_posts_the_time_he_chose_and_the_card_says_when_it_returns(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit");
        let (ix, card) = card_titled(&view, FULL, cx);
        let until = Instant::carried((Local::now() + chrono::Duration::hours(3)).to_rfc3339());
        view.update_in(cx, |view, window, cx| view.answer_ready(ix, &card, ReadyAnswer::Remind { until: until.clone() }, window, cx));
        redraw(cx);
        view.read_with(cx, |view, _| match &view.mock_ready.last().expect("an answer went to Fleet").answer {
            ReadyAnswer::Remind { until: sent } => assert_eq!(sent.as_str(), until.as_str()),
            other => panic!("expected remind, sent {other:?}"),
        });
        assert!(!drawn_like(cx, Role::Article, &format!("Ready to try: {FULL}, Back ")).is_empty());
    }

    #[gpui::test]
    fn a_refused_answer_takes_the_card_back_and_shows_fleets_sentence_on_it(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit");
        let (ix, card) = card_titled(&view, FULL, cx);
        let past = Instant::carried((Local::now() - chrono::Duration::hours(1)).to_rfc3339());
        view.update_in(cx, |view, window, cx| view.answer_ready(ix, &card, ReadyAnswer::Remind { until: past }, window, cx));
        redraw(cx);
        let said = view.read_with(cx, |view, _| {
            let page = view.pages.get(view.shown.as_ref().expect("shown")).expect("its page");
            page.cards.said.get(&card.id).map(|why| why.to_string())
        });
        assert_eq!(said.as_deref(), Some("a time to remind him at that has not passed"));
        assert!(drawn(cx, Role::Status, "Refused"), "the sentence stands on the card's own row");
        assert!(drawn(cx, Role::Article, &format!("Ready to try: {FULL}")), "the card stays open");
        assert!(drawn(cx, Role::Button, "Looks good"));
    }

    #[gpui::test]
    fn a_press_in_needs_you_goes_to_the_card_gives_it_the_keys_and_pulses_it_once(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit");
        let (_, card) = card_titled(&view, FULL, cx);
        let held = view.update_in(cx, |view, window, cx| view.open_named(&card.id, window, cx));
        assert!(held, "the Job holds the card its item names");
        redraw(cx);
        let (pulsed, focused) = view.update_in(cx, |view, window, _| {
            let page = view.pages.get(view.shown.as_ref().expect("shown")).expect("its page");
            let spot = page.keys.spot_of(&format!("card:{}", card.id)).expect("the card holds a spot");
            (page.cards.pulse.as_ref().map(|(id, _)| id.clone()), spot.focus.is_focused(window))
        });
        assert_eq!(pulsed.as_deref(), Some(card.id.as_str()));
        assert!(focused, "the card has the keys");
        view.read_with(cx, |view, _| assert!(view.artifacts_of("tool-edit").iter().all(|one| one.id != card.id), "no reader opened for it"));
    }

    #[gpui::test]
    fn the_for_you_tab_reaches_the_card_the_same_way(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-search");
        let (_, card) = card_titled(&view, "Search finds a Job by its branch", cx);
        view.update_in(cx, |view, window, cx| view.open_artifact("tool-search", &card.id, window, cx));
        redraw(cx);
        let pulsed = view.read_with(cx, |view, _| view.pages.get(view.shown.as_ref().expect("shown")).and_then(|page| page.cards.pulse.as_ref().map(|(id, _)| id.clone())));
        assert_eq!(pulsed.as_deref(), Some(card.id.as_str()));
    }

    #[gpui::test]
    fn a_card_the_thread_no_longer_holds_opens_nothing(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx, "tool-edit");
        let held = view.update_in(cx, |view, window, cx| view.open_named("try:native/gone@0000", window, cx));
        assert!(!held);
    }
}
