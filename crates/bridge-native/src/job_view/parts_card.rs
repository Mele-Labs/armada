//! The card of an artifact that holds parts, in the Band look the owner approved on 14 Nov 2026 (take A, "each ask
//! answered on its own"): the band over a stack of parts in the order the Job added them, each with its caption and
//! a tag, and a footer that holds Dismiss.
//!
//! **Each part that asks is answered alone.** Words, a drawing and a page get Approve and Ask for changes. Takes are
//! a vote: a pick, with 1 to 6 choosing a take and Send pick sending it, and never an Approve, which Fleet refuses on
//! them. A buttons part acts when a button is pressed and asks nothing. The band reads Asking until every ask is in,
//! then the state the artifact settled in. A press shows its answer at once and a refusal takes it back, with Fleet's
//! sentence on that part's row (`answer_part`, `press_page_action`).

use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use gpui::{canvas, div, prelude::*, px, AnyElement, ClickEvent, Context, Entity, FontWeight, Focusable, KeyDownEvent, MouseButton, Role, SharedString, Window};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::text::TextView;
use ipc::{AnswerPart, ArtifactPart, ArtifactState, DismissWaiting, JobArtifact, PartAction, PartAnswer, PartContent, PartHow, PartKind, PartState, PartTake, PressDid, PressPageAction, SessionId, VoteForm};

use super::acts::{refusal, Answer, Mode};
use super::artifacts_tab::{Picture, PictureKey};
use super::band_card::{band, foot_box, frame, ghost, most, revision_said, shapes, well, Act, Do, Foot, Head, Stand};
use super::reader_sheet::artifact_in;
use super::review::{Ask, Item};
use super::rows::{glyph, tip, Size};
use super::sketch_card::{act, mixed, Kind};
use super::{Extra, JobView, Page};
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::scene::{Clock, Fit, Lens};

/// The route `answer_part` is posted to.
pub(super) const ANSWER_PART: &str = "/sessions/artifacts/part/answer";
/// The route a buttons part's press goes to, as a page's does (`press_page_action`).
pub(super) const PRESS: &str = crate::web::kit::PRESS;
/// What joins an artifact's id to a part's id in the key a part is kept under.
const JOIN: char = '\u{1f}';
/// The most takes one row holds.
const ACROSS: usize = 3;
/// The least a take is drawn wide side by side with another in the reading column, in points. Narrower, they stand one a row.
const SIDE_BY_SIDE: f32 = 520.;
/// How long Copy reads Copied after a press.
const COPIED_FOR: std::time::Duration = std::time::Duration::from_millis(1500);
/// How tall a picture may stand in the reading column, against the pane it is read in. In the review panel it is `reader_sheet::TALL_SHARE`.
const TALL_SHARE: f32 = 0.6;
/// How tall the take drawn large in the review sheet may stand, against the sheet.
const LARGE_SHARE: f32 = 0.55;

/// The order the sheet shows takes in: the Job's pick first, the rest in the order the Job gave them.
pub(super) fn sheet_order(takes: &[PartTake]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..takes.len()).collect();
    order.sort_by_key(|at| !pick_mark(&takes[*at].title).1);
    order
}

/// The take the sheet draws large: the one he last put large, else the one chosen, else the first in `order`.
pub(super) fn sheet_shown(order: &[usize], large: Option<usize>, chosen: Option<usize>) -> usize {
    large.or(chosen).filter(|at| order.contains(at)).or_else(|| order.first().copied()).unwrap_or(0)
}

/// The take ← (`by` -1) or → (`by` 1) puts large from `at`, walking `order` and wrapping at either end.
pub(super) fn sheet_step(order: &[usize], at: usize, by: isize) -> usize {
    let Some(place) = order.iter().position(|one| *one == at) else { return order.first().copied().unwrap_or(0) };
    let len = order.len() as isize;
    order[((place as isize + by).rem_euclid(len)) as usize]
}

/// How many takes one row holds in a body `room` points wide with `gap` between: as many as would each be `SIDE_BY_SIDE` wide, and
/// never more than `ACROSS`. Four stand two and two.
fn across_in(room: f32, gap: f32, takes: usize) -> usize {
    let fit = (((room + gap) / (SIDE_BY_SIDE + gap)).floor() as usize).clamp(1, ACROSS);
    let across = fit.min(takes.max(1));
    if takes == 4 && across > 2 {
        2
    } else {
        across
    }
}

/// The key a part is kept under in a Job's page: its artifact and its own id.
pub(super) fn part_key(artifact: &str, part: &str) -> String {
    format!("{artifact}{JOIN}{part}")
}

/// `element`, whose presses are its own: they do not reach the card under it, so a press on a control never also opens the card.
fn held_press(element: AnyElement) -> AnyElement {
    div().on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()).child(element).into_any_element()
}

pub(super) fn split_key(key: &str) -> Option<(&str, &str)> {
    key.split_once(JOIN)
}

/// What the cards of one Job's artifacts of parts remember.
#[derive(Default)]
pub(super) struct PartsState {
    /// The take chosen in a takes part and not yet sent, by part, as its place in the part.
    chosen: HashMap<String, usize>,
    /// The take drawn large, by part.
    large: HashMap<String, usize>,
    /// What he answered, by part, read at once and until Fleet's own part says it: the `drawn` it was answered on, the
    /// state it moved to, and his words.
    sent: HashMap<String, (String, PartState, PartAnswer)>,
    /// What a button said when pressed, by part and action, until Fleet's press says it.
    went: HashMap<String, String>,
    /// The one-line note on a pick, by takes part.
    notes: HashMap<String, Entity<InputState>>,
    typing: HashMap<String, gpui::Subscription>,
    /// Fleet's reason for the last act on a part it refused, by part.
    said: HashMap<String, SharedString>,
    /// Words parts he opened past their cut with Read more, by part.
    whole: std::collections::HashSet<String>,
    /// The words part whose Copy reads Copied, and the wait that turns it back.
    copied: Option<String>,
    _copying: Option<gpui::Task<()>>,
}

impl PartsState {
    /// The take he put large in takes part `key`, if he did.
    pub(super) fn large_of(&self, key: &str) -> Option<usize> {
        self.large.get(key).copied()
    }
}

/// The part as he reads it: his answer on it where Fleet's list has not caught up, on the revision he answered.
fn effective(part: &ArtifactPart, key: &str, state: &PartsState) -> ArtifactPart {
    match state.sent.get(key) {
        Some((drawn, now, answer)) if part.state == PartState::Open && *drawn == part.drawn => ArtifactPart { state: *now, answer: Some(answer.clone()), ..part.clone() },
        _ => part.clone(),
    }
}

/// The part as the card reads it, for the review panel's list: which artifacts still ask.
pub(super) fn effective_part(part: &ArtifactPart, key: &str, page: &Page) -> ArtifactPart {
    effective(part, key, &page.cards.parts)
}

/// Where the artifact stands as its card reads it: Asking while any part still asks, then Fleet's state, or the one his
/// answers settle it in where Fleet's list is behind them.
pub(super) fn stand_of(parts: &[ArtifactPart], fleet: ArtifactState) -> Stand {
    // His Dismiss leaves the parts open and Fleet reads the artifact Dismissed over them (`group_state`): it asks no more
    // (24 Nov: a dismissed vote came back each time he opened the sheet).
    if fleet == ArtifactState::Dismissed {
        return Stand::of(fleet);
    }
    if parts.iter().any(ArtifactPart::is_open_ask) {
        return Stand::Asking;
    }
    if fleet != ArtifactState::Asking {
        return Stand::of(fleet);
    }
    let answered: Vec<&ArtifactPart> = parts.iter().filter(|part| part.asks && matches!(part.state, PartState::Approved | PartState::SentBack | PartState::Picked)).collect();
    match (answered.is_empty(), answered.iter().any(|part| part.state == PartState::SentBack)) {
        (true, _) => Stand::Reading,
        (false, true) => Stand::SentBack,
        (false, false) => Stand::Approved,
    }
}

/// A take's title and whether the Job marked it as its pick, in the words it wrote: `B Rail (Job's pick)`.
pub(super) fn pick_mark(title: &str) -> (&str, bool) {
    match title.trim_end().strip_suffix("(Job's pick)") {
        Some(rest) => (rest.trim_end(), true),
        None => (title, false),
    }
}

/// The take a key chooses: 1 is the first. None where the key is no digit or the part holds no such take.
pub(super) fn take_of_key(key: &str, takes: usize) -> Option<usize> {
    let number = key.parse::<usize>().ok()?;
    (1..=takes.min(9)).contains(&number).then(|| number - 1)
}

/// The kinds that still ask, in the words the footer's line uses: `the words and the takes`.
fn asked_of(parts: &[ArtifactPart]) -> Option<String> {
    let mut kinds: Vec<&'static str> = Vec::new();
    for part in parts.iter().filter(|part| part.is_open_ask()) {
        let (_, name) = kind_look(part.content.kind());
        if !kinds.contains(&name) {
            kinds.push(name);
        }
    }
    (!kinds.is_empty()).then(|| kinds.iter().map(|name| format!("the {}", name.to_lowercase())).collect::<Vec<_>>().join(" and "))
}

/// The line in the footer while asks remain: what still has to be answered.
pub(super) fn settles_line(parts: &[ArtifactPart]) -> Option<String> {
    let answered = parts.iter().any(|part| part.asks && matches!(part.state, PartState::Approved | PartState::SentBack | PartState::Picked));
    let asked = asked_of(parts)?;
    // `the drawing` and `the page` are one thing; `the words`, `the takes` and any pair are several.
    let verb = if asked.contains(" and ") || asked.ends_with('s') { "are" } else { "is" };
    Some(format!("Settles when {asked} {verb} answered{}", if answered { " too" } else { "" }))
}

/// The glyph and the name of a part's kind, which its caption draws.
pub(super) fn kind_look(kind: PartKind) -> (&'static str, &'static str) {
    match kind {
        PartKind::Words => ("file-text", "Words"),
        PartKind::Takes => ("scale", "Takes"),
        PartKind::Drawing => ("git-branch", "Drawing"),
        PartKind::Buttons => ("zap", "Buttons"),
        PartKind::Page => ("scroll-text", "Page"),
        PartKind::Prototype => ("app-window", "Prototype"),
    }
}

/// How a part stands for its tag: informs, asks him, or acts when pressed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tag {
    Informs,
    Asks,
    /// It asked, and he has answered.
    Asked,
    Acts,
}

impl Tag {
    fn of(part: &ArtifactPart) -> Tag {
        match (part.content.kind(), part.asks, part.state) {
            (PartKind::Buttons, _, _) => Tag::Acts,
            (_, true, PartState::Open) => Tag::Asks,
            (_, true, _) => Tag::Asked,
            (_, false, _) => Tag::Informs,
        }
    }

    /// The word of a tag that is the same for every part. An answered part says how it stands instead (`stands`).
    fn word(self) -> &'static str {
        match self {
            Tag::Informs => "Informs",
            Tag::Asks => "Asks you",
            Tag::Acts => "Acts",
            Tag::Asked => "Answered",
        }
    }

    fn tip(self) -> &'static str {
        match self {
            Tag::Informs => "Shown to you, with no answer asked",
            Tag::Asks => "Waiting on your answer",
            Tag::Asked => "You answered it",
            Tag::Acts => "Each button acts when you press it",
        }
    }
}

/// How an answered part stands, in the glyph, hue, word and tooltip its one-line answer and its tag share.
fn stands(state: PartState) -> (&'static str, crate::look::Token, &'static str, &'static str) {
    match state {
        PartState::Approved => ("badge-check", token::STATUS_COMPLETED_SUCCESS, "Approved", "You approved it"),
        PartState::SentBack => ("reply", token::STATUS_REJECTED, "Sent back", "You sent it back with changes"),
        PartState::Picked => ("check", token::STATUS_COMPLETED_SUCCESS, "Picked", "You picked a take"),
        PartState::Withdrawn => ("ban", token::STATUS_KILLED, "Taken back", "The Job took it back"),
        PartState::Open => ("circle-dot", token::STATUS_AWAITING_REVIEW, "Asking", "Waiting on your answer"),
    }
}

/// The take a part's answer picked, by the words its tile carries.
pub(super) fn picked_take(part: &ArtifactPart) -> Option<String> {
    match (&part.content, part.answer.as_ref().and_then(|answer| answer.take.as_deref())) {
        (PartContent::Takes { takes, .. }, Some(id)) => takes.iter().find(|one| one.id == id).map(|one| pick_mark(&one.title).0.to_string()),
        _ => None,
    }
}

/// What a part's tag says and its hue: where it has been answered, how, in the one-line answer's words and colour.
fn tag_of(part: &ArtifactPart) -> (String, Option<crate::look::Token>) {
    let tag = Tag::of(part);
    if tag != Tag::Asked {
        return (tag.word().to_string(), None);
    }
    let (_, ink, word, _) = stands(part.state);
    let word = match (part.state, picked_take(part)) {
        (PartState::Picked, Some(take)) => format!("{word} {take}"),
        _ => word.to_string(),
    };
    (word, Some(ink))
}

/// What the card needs while it draws one part.
struct Ctx<'a> {
    ix: usize,
    id: &'a str,
    job: SharedString,
    page: &'a Page,
    look: &'a Look,
    /// Drawn in the artifact's own tab, where a drawing is large and nothing folds.
    tab: bool,
    /// Drawn in the review panel's detail: prose keeps its measure, pictures take the sheet's width, and no frame is round it.
    sheet: bool,
    /// The part the keys A and C answer, and the takes part 1 to 6 and Cmd-Enter answer: the first of each still asking.
    words: Option<String>,
    takes: Option<String>,
    /// Two or more votes still ask: each keeps its note box, and one Send picks in the foot sends them all.
    several: bool,
    /// The artifact holds a prototype: its words are captioned as the Job's own, apart from the build he approves.
    prototype: bool,
}

impl PartsState {
    /// The words box of a takes part, where it has been made.
    #[cfg(test)]
    pub(super) fn notes(&self, key: &str) -> Option<Entity<InputState>> {
        self.notes.get(key).cloned()
    }

    /// What he answered a part, as it was sent and not yet read back from Fleet.
    #[cfg(test)]
    pub(super) fn answer_sent(&self, key: &str) -> Option<&PartAnswer> {
        self.sent.get(key).map(|(_, _, answer)| answer)
    }
}

impl Ctx<'_> {
    fn state(&self) -> &PartsState {
        &self.page.cards.parts
    }
}

fn is_takes(part: &ArtifactPart) -> bool {
    matches!(part.content, PartContent::Takes { .. })
}

/// A part Approve and Ask for changes answer: words, a drawing or a page that asks.
fn is_words_like(part: &ArtifactPart) -> bool {
    part.is_open_ask() && !matches!(part.content, PartContent::Takes { .. } | PartContent::Buttons { .. })
}

/// A part a pick answers: a takes part of the `pick` form that asks.
fn is_pick(part: &ArtifactPart) -> bool {
    part.is_open_ask() && matches!(part.content, PartContent::Takes { form: VoteForm::Pick, .. })
}

/// Whether two or more votes of an artifact still ask, so their picks are sent together.
fn several_picks(parts: &[ArtifactPart]) -> bool {
    parts.iter().filter(|part| is_pick(part)).count() > 1
}

/// A key that stops where a button takes Enter and Space as a press, so the card's own Enter does not open its page too.
fn presses_on_enter(event: &KeyDownEvent, cx: &mut gpui::App) {
    if !event.keystroke.modifiers.modified() && matches!(event.keystroke.key.as_str(), "enter" | "space") {
        cx.stop_propagation();
    }
}

impl JobView {
    /// The artifacts of the Job that hold parts and are not folded away, the oldest first.
    pub(super) fn parts_artifacts(&self, job: &str) -> Vec<&JobArtifact> {
        let mut held: Vec<&JobArtifact> = self.artifacts_of(job).into_iter().filter(|one| !one.parts.is_empty() && one.archived.is_none()).collect();
        held.sort_by(|a, b| a.parts[0].added.as_str().cmp(b.parts[0].added.as_str()));
        held
    }

    /// Whether a sketch, draft or page the thread holds a card for is an artifact of parts or one of its parts, which its card draws.
    pub(super) fn is_part(&self, job: &str, id: &str) -> bool {
        self.parts_artifacts(job).iter().any(|one| one.id == id || one.parts.iter().any(|part| part.id == id))
    }

    /// A part by its key, as he reads it, with the id of the artifact it belongs to.
    pub(super) fn part_of_key(&self, key: &str) -> Option<(String, ArtifactPart)> {
        self.part_of_key_in(self.shown.as_ref()?, key)
    }

    /// The same, of Job `job`'s artifact: the review sheet from the Jobs list holds other Jobs' asks.
    fn part_of_key_in(&self, job: &str, key: &str) -> Option<(String, ArtifactPart)> {
        let (artifact, part) = split_key(key)?;
        let page = self.pages.get(job)?;
        let held = self.artifact_of(job, artifact)?.parts.iter().find(|held| held.id == part)?;
        Some((artifact.to_string(), effective(held, key, &page.cards.parts)))
    }

    /// The takes of a takes part, whether a pick is still asked for, and the take he has chosen or picked, by its place.
    pub(super) fn takes_of_key(&self, key: &str) -> Option<(Vec<PartTake>, bool, Option<usize>)> {
        self.takes_of_key_in(self.shown.as_ref()?, key)
    }

    /// The same, of Job `job`'s artifact.
    pub(super) fn takes_of_key_in(&self, job: &str, key: &str) -> Option<(Vec<PartTake>, bool, Option<usize>)> {
        let (_, part) = self.part_of_key_in(job, key)?;
        let PartContent::Takes { takes, form, .. } = &part.content else { return None };
        let open = part.is_open_ask() && *form == VoteForm::Pick;
        let picked = part.answer.as_ref().and_then(|answer| answer.take.as_deref()).and_then(|id| takes.iter().position(|one| one.id == id));
        let chosen = if open { self.pages.get(job)?.cards.parts.chosen.get(key).copied().filter(|at| *at < takes.len()) } else { picked };
        Some((takes.clone(), open, chosen))
    }

    /// What stands under a picture of a part he is asked about: Send pick, with its note, under a vote that asks, and Approve and
    /// Ask for changes under a drawing that asks. `None` where the part asks nothing.
    pub(super) fn part_acts(&self, key: &str, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let (artifact, part) = self.part_of_key(key)?;
        let job = self.shown.clone()?;
        let page = self.pages.get(&job)?;
        let held = self.artifact_of(&job, &artifact)?;
        let parts: Vec<ArtifactPart> = held.parts.iter().map(|part| effective(part, &part_key(&artifact, &part.id), &page.cards.parts)).collect();
        let c = Ctx {
            ix: super::acts::SHEET,
            id: &artifact,
            job,
            page,
            look,
            tab: true,
            sheet: false,
            words: parts.iter().find(|part| is_words_like(part)).map(|part| part.id.clone()),
            takes: parts.iter().find(|part| is_pick(part)).map(|part| part.id.clone()),
            several: false,
            prototype: parts.iter().any(|part| matches!(part.content, PartContent::Prototype(_))),
        };
        if is_pick(&part) {
            let ready = self.takes_of_key(key).is_some_and(|(_, _, chosen)| chosen.is_some());
            return Some(self.pick_row(&c, key, ready, c.takes.as_deref() == Some(part.id.as_str()), cx));
        }
        (is_words_like(&part) && matches!(part.content, PartContent::Drawing { .. })).then(|| self.approve_row(&c, key, &part, cx))
    }

    /// The list place of the card of an artifact's newest revision: under the item it was kept after, or among the extras.
    pub(super) fn parts_ix(&self, artifact: &str) -> Option<usize> {
        let job = self.shown.as_ref()?;
        let page = self.pages.get(job)?;
        // The Job's first ask is answered on the first card, so its parts stand there.
        if let Some(at) = page.extras.iter().position(|extra| matches!(extra, Extra::First(key) if key == artifact)) {
            return Some(page.items + at);
        }
        let newest = self.artifact_of(job, artifact)?.revisions.last()?.number;
        if let Some(held) = page.anchors.iter().find(|one| one.artifact == artifact && one.revision == newest) {
            return Some(held.item);
        }
        page.extras.iter().position(|extra| matches!(extra, Extra::Parts(id, number) if id == artifact && *number == newest)).map(|at| page.items + at)
    }

    /// An artifact that still asks, and the sheet holds, is answered only there: in its own tab it is read only.
    pub(super) fn reads_only(&self, artifact: &str, cx: &gpui::App) -> bool {
        let Some(job) = self.shown.clone() else { return false };
        let (Some(page), Some(held)) = (self.pages.get(&job), self.artifact_of(&job, artifact)) else { return false };
        let parts: Vec<ArtifactPart> = held.parts.iter().map(|part| effective(part, &part_key(artifact, &part.id), &page.cards.parts)).collect();
        let asking = page.cards.answers.get(artifact) != Some(&Answer::Withdrawn) && stand_of(&parts, held.state) == Stand::Asking;
        asking && self.asking_item(&job, artifact, cx).and_then(|item| self.held_item(&job, &item, cx)).is_some()
    }

    pub(super) fn parts_changed(&mut self, artifact: &str, cx: &mut Context<Self>) {
        match self.parts_ix(artifact) {
            Some(ix) => self.card_changed(ix, cx),
            None => cx.notify(),
        }
    }

    /// Fleet's list of the Job's artifacts moved: the thread's cards are worked out again once this draw is over, since the
    /// list decides which artifacts of parts stand in it.
    pub(super) fn parts_arrived(&mut self, job: &SharedString, cx: &mut Context<Self>) {
        let (me, job) = (cx.entity().downgrade(), job.clone());
        cx.defer(move |cx| {
            let _ = me.update(cx, |this, cx| {
                this.sync(&job, cx);
                this.sync_prototypes(&job, cx);
                this.parts_redrawn(&job, cx);
            });
        });
    }

    /// Every card of parts of this Job is measured again: Fleet's list of its artifacts changed.
    pub(super) fn parts_redrawn(&mut self, job: &str, cx: &mut Context<Self>) {
        if let Some(page) = self.pages.get(job) {
            for (at, extra) in page.extras.iter().enumerate() {
                if matches!(extra, Extra::Parts(..)) {
                    page.remeasure(page.items + at);
                }
            }
            for held in &page.anchors {
                page.remeasure(held.item);
            }
        }
        cx.notify();
    }

    fn parts_state_mut(&mut self) -> Option<&mut PartsState> {
        self.cards_mut().map(|cards| &mut cards.parts)
    }

    /// The note field of each takes part that asks, made before the card draws since a field is an entity.
    pub(super) fn tend_parts(&mut self, job: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let wanted: Vec<String> = self.parts_artifacts(job).iter().flat_map(|one| one.parts.iter().filter(|part| is_pick(part)).map(|part| part_key(&one.id, &part.id))).collect();
        for key in wanted {
            if self.pages.get(job).is_none_or(|page| page.cards.parts.notes.contains_key(&key)) {
                continue;
            }
            let input = cx.new(|cx| InputState::new(window, cx).placeholder("Note"));
            let sent = key.clone();
            let typing = cx.subscribe_in(&input, window, move |this, _, event: &InputEvent, window, cx| {
                if let InputEvent::PressEnter { .. } = event {
                    match split_key(&sent).map(|(artifact, _)| artifact.to_string()) {
                        Some(artifact) if this.several_ask(&artifact) => this.send_picks(&artifact, window, cx),
                        _ => this.send_pick(&sent, window, cx),
                    }
                }
            });
            if let Some(page) = self.pages.get_mut(job) {
                page.cards.parts.notes.insert(key.clone(), input);
                page.cards.parts.typing.insert(key, typing);
            }
        }
        // The panel's page parts are pages, which are live entities too.
        self.sync_panel_pages(window, cx);
    }

    /// Whether he is typing in a part's note or in the box of Ask for changes: the keys are the box's own then.
    pub(super) fn parts_typing(&self, artifact: &str, window: &Window, cx: &gpui::App) -> bool {
        let Some(page) = self.shown.as_ref().and_then(|job| self.pages.get(job)) else { return false };
        let prefix = format!("{artifact}{JOIN}");
        page.cards.parts.notes.iter().any(|(key, input)| key.starts_with(&prefix) && input.read(cx).focus_handle(cx).is_focused(window)) || page.cards.modes.keys().any(|key| key.starts_with(&prefix))
    }

    /// Esc in a part's box: an open Ask for changes box closes, its words kept as drafts, and the card has the keys again.
    pub(super) fn close_parts_boxes(&mut self, artifact: &str, window: &mut Window, cx: &mut Context<Self>) {
        // A list open over the words goes first; the box shuts on the next Esc.
        if self.close_note_lists(cx) {
            return;
        }
        let prefix = format!("{artifact}{JOIN}");
        let open: Vec<String> = self.shown.as_ref().and_then(|job| self.pages.get(job)).map(|page| page.cards.modes.keys().filter(|key| key.starts_with(&prefix)).cloned().collect()).unwrap_or_default();
        // Parts read in the review panel stand on its own place, not a card's in the thread.
        for key in &open {
            self.close_box_kept(super::acts::SHEET, key, cx);
        }
        self.focus_card(artifact, window, cx);
    }

    // ---- what he does ------------------------------------------------------------------------------------------

    /// Answer a part: it reads the answer in this frame and Fleet is told. A refusal takes the answer back, and the part's
    /// row says why.
    #[allow(clippy::too_many_arguments)]
    fn answer_part(&mut self, artifact: &str, part: &ArtifactPart, how: PartHow, take: Option<String>, words: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        self.answer_part_with(artifact, part, how, take, words, None, window, cx);
    }

    /// [`Self::answer_part`] with `extras` in place of what the part's own box holds: the sheet's box sends them.
    #[allow(clippy::too_many_arguments)]
    fn answer_part_with(&mut self, artifact: &str, part: &ArtifactPart, how: PartHow, take: Option<String>, words: Option<String>, extras: Option<ipc::AnswerExtras>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let key = part_key(artifact, &part.id);
        // Read before the box goes.
        let extras = extras.unwrap_or_else(|| self.extras_of(&key, Mode::Note, cx));
        if let Some(ix) = self.parts_ix(artifact) {
            self.close_box(ix, &key, cx);
        }
        let now = match how {
            PartHow::Approved => PartState::Approved,
            PartHow::SentBack => PartState::SentBack,
            PartHow::Picked => PartState::Picked,
        };
        let answer = PartAnswer { words: words.clone(), take: take.clone(), notes: BTreeMap::new(), at: None };
        if let Some(state) = self.parts_state_mut() {
            state.sent.insert(key.clone(), (part.drawn.clone(), now, answer));
            state.said.remove(&key);
            state.large.remove(&key);
        }
        // Once this answer leaves no ask open, what he said of the revision is kept for the line it is when the Job draws the next.
        let settled = self.pages.get(&job).zip(self.artifact_of(&job, artifact)).map(|(page, held)| {
            (held.parts.iter().map(|one| effective(one, &part_key(artifact, &one.id), &page.cards.parts)).collect::<Vec<ArtifactPart>>(), held.words.clone())
        });
        if let Some((parts, words)) = settled {
            self.keep_settled(&job, artifact, &parts, words.as_deref(), cx);
        }
        self.parts_changed(artifact, cx);
        let said = AnswerPart { session_id: SessionId::carried(job.to_string()), artifact: artifact.to_string(), part: part.id.clone(), drawn: part.drawn.clone(), how, take, words, notes: BTreeMap::new(), extras };
        let body = ipc::encode(&said).map_err(|why| format!("{why:?}"));
        let back = job.clone();
        self.post_with(job, ANSWER_PART, body, move |this, why, cx| this.part_refused(&back, &key, why, cx), window, cx);
    }

    /// Fleet did not take it: the answer is taken back and its sentence stands on the part's row.
    pub(super) fn part_refused(&mut self, job: &SharedString, key: &str, why: SharedString, cx: &mut Context<Self>) {
        if let Some(page) = self.pages.get_mut(job) {
            page.cards.parts.sent.remove(key);
            page.cards.parts.went.retain(|went, _| !went.starts_with(key));
            page.cards.parts.said.insert(key.to_string(), why);
        }
        if let Some((artifact, _)) = split_key(key) {
            let artifact = artifact.to_string();
            if let Some(newest) = self.artifact_of(job, &artifact).and_then(|held| held.revisions.last()).map(|last| last.number) {
                crate::drafts::clear(&super::parts_fold::kept_key(job, &artifact, newest), cx);
            }
            self.parts_changed(&artifact, cx);
        }
    }

    /// D on the artifact in the review panel: Dismiss while it asks, Archive once it does not.
    pub(super) fn dismiss_artifact(&mut self, artifact: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let Some(held) = self.artifact_of(&job, artifact).cloned() else { return };
        let asking = self.pages.get(&job).is_some_and(|page| {
            let parts: Vec<ArtifactPart> = held.parts.iter().map(|part| effective(part, &part_key(artifact, &part.id), &page.cards.parts)).collect();
            page.cards.answers.get(artifact) != Some(&Answer::Withdrawn) && stand_of(&parts, held.state) == Stand::Asking
        });
        if asking {
            self.dismiss_parts(super::acts::SHEET, artifact, window, cx);
        } else {
            self.archive_card(super::acts::SHEET, artifact, artifact, window, cx);
        }
    }

    /// Dismiss the artifact: it reads Dismissed in this frame and Fleet is told. A refusal takes it back (`undo`).
    fn dismiss_parts(&mut self, ix: usize, artifact: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let Some(held) = self.artifact_of(&job, artifact).cloned() else { return };
        let item = held.item.clone().unwrap_or_else(|| held.id.clone());
        if let Some(newest) = held.revisions.last().map(|last| last.number) {
            let verdict = super::parts_fold::Verdict { stand: Stand::Dismissed, word: Stand::Dismissed.word().into(), note: None };
            self.keep_verdict(&job, artifact, newest, &verdict, cx);
        }
        let body = ipc::encode(&DismissWaiting { session_id: SessionId::carried(job.to_string()), item_id: item, reason: None, misread: false }).map_err(|why| format!("{why:?}"));
        self.settle(ix, artifact.to_string(), Answer::Withdrawn, "/sessions/waiting/dismiss", body, window, cx);
    }

    /// Send the pick: the take he chose, and the words in the note beside it.
    pub(super) fn send_pick(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some((artifact, part)) = split_key(key) else { return };
        let (artifact, part) = (artifact.to_string(), part.to_string());
        let Some(job) = self.shown.clone() else { return };
        let Some(held) = self.artifact_of(&job, &artifact).and_then(|one| one.parts.iter().find(|held| held.id == part)).cloned() else { return };
        let PartContent::Takes { takes, .. } = &held.content else { return };
        let Some(page) = self.pages.get(&job) else { return };
        if !effective(&held, key, &page.cards.parts).is_open_ask() {
            return;
        }
        let Some(take) = page.cards.parts.chosen.get(key).and_then(|chosen| takes.get(*chosen)).map(|take| take.id.clone()) else { return };
        let words = page.cards.parts.notes.get(key).map(|input| input.read(cx).value().trim().to_string()).filter(|words| !words.is_empty());
        self.answer_part(&artifact, &held, PartHow::Picked, Some(take), words, window, cx);
    }

    /// Send every pick he has made on an artifact's votes, one answer to each with its own note. A vote with no pick keeps asking.
    pub(super) fn send_picks(&mut self, artifact: &str, window: &mut Window, cx: &mut Context<Self>) {
        for key in self.picks_made(artifact) {
            self.send_pick(&key, window, cx);
        }
    }

    /// Whether two or more votes of an artifact still ask.
    fn several_ask(&self, artifact: &str) -> bool {
        let Some(job) = self.shown.as_ref() else { return false };
        let (Some(held), Some(page)) = (self.artifact_of(job, artifact), self.pages.get(job)) else { return false };
        let parts: Vec<ArtifactPart> = held.parts.iter().map(|part| effective(part, &part_key(artifact, &part.id), &page.cards.parts)).collect();
        several_picks(&parts)
    }

    /// The keys of the votes of an artifact that ask and have a take chosen.
    fn picks_made(&self, artifact: &str) -> Vec<String> {
        let Some(job) = self.shown.as_ref() else { return Vec::new() };
        let (Some(held), Some(page)) = (self.artifact_of(job, artifact), self.pages.get(job)) else { return Vec::new() };
        held.parts
            .iter()
            .filter(|part| is_pick(&effective(part, &part_key(artifact, &part.id), &page.cards.parts)))
            .map(|part| part_key(artifact, &part.id))
            .filter(|key| self.takes_of_key(key).is_some_and(|(_, _, chosen)| chosen.is_some()))
            .collect()
    }

    /// Choose a take of a takes part, as a press or a digit does. It stays a choice until Send pick.
    pub(super) fn choose_take(&mut self, key: &str, take: usize, cx: &mut Context<Self>) {
        if let Some(state) = self.parts_state_mut() {
            state.chosen.insert(key.to_string(), take);
        }
        if let Some((artifact, _)) = split_key(key) {
            let artifact = artifact.to_string();
            self.parts_changed(&artifact, cx);
        }
    }

    /// A take drawn large, or the takes again where `to` is none.
    pub(super) fn large_take(&mut self, key: &str, to: Option<usize>, cx: &mut Context<Self>) {
        if let Some(state) = self.parts_state_mut() {
            match to {
                Some(take) => state.large.insert(key.to_string(), take),
                None => state.large.remove(key),
            };
        }
        if let Some((artifact, _)) = split_key(key) {
            let artifact = artifact.to_string();
            self.parts_changed(&artifact, cx);
        }
    }

    /// Press a button of a buttons part: it reads sent at once, and Fleet runs the act (`press_page_action`).
    fn press_button(&mut self, artifact: &str, part: &ArtifactPart, action: &PartAction, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let key = part_key(artifact, &part.id);
        if let Some(state) = self.parts_state_mut() {
            state.went.insert(format!("{key}{JOIN}{}", action.id), format!("Sent to the Job: {}", action.label));
            state.said.remove(&key);
        }
        self.parts_changed(artifact, cx);
        let pressed = PressPageAction { session_id: SessionId::carried(job.to_string()), artifact: artifact.to_string(), revision: part.revision, action: action.id.clone(), words: None };
        let body = ipc::encode(&pressed).map_err(|why| format!("{why:?}"));
        let back = job.clone();
        self.post_with(job, PRESS, body, move |this, why, cx| this.part_refused(&back, &key, why, cx), window, cx);
    }

    /// Ask for changes on a part: its note box opens under it.
    pub(super) fn part_ask_for_changes(&mut self, artifact: &str, part: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.review_open().then_some(super::acts::SHEET).or_else(|| self.parts_ix(artifact)) else { return };
        self.ask_in_note(ix, &part_key(artifact, part), window, cx);
    }

    /// The keys go back to the card, once its box has shut: the card in the thread, or the reader's own in a tab.
    pub(super) fn focus_card(&mut self, artifact: &str, window: &mut Window, cx: &mut Context<Self>) {
        let page = self.shown.as_ref().and_then(|job| self.pages.get(job));
        let reader = page.and_then(|page| page.keys.spot_of(&format!("reader:{artifact}")));
        // With the sheet open the keys stay in it: the thread's card behind it would take Esc away from the sheet.
        let open = self.review_open();
        let spot = if open { reader } else { reader.or_else(|| page.and_then(|page| page.keys.spot_of(&format!("card:{artifact}")))) };
        match spot {
            Some(spot) => window.focus(&spot.focus, cx),
            None if open => window.focus(&self.review_focus.list, cx),
            None => {}
        }
    }

    /// The same, from a part's key: the pressed control is gone once the take opens or shuts, and the keys must not go with it.
    fn focus_card_of(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((artifact, _)) = split_key(key) {
            let artifact = artifact.to_string();
            self.focus_card(&artifact, window, cx);
        }
    }

    /// The keys on a card holding the keys: A and C answer the first words that ask, 1 to 6 choose a take and Cmd-Enter sends
    /// the pick, F draws a take large and Esc puts it back, D puts the artifact away and Enter reads it on its own.
    pub(super) fn parts_key(&mut self, artifact: &str, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        if self.parts_typing(artifact, window, cx) {
            return;
        }
        let Some(held) = self.artifact_of(&job, artifact).cloned() else { return };
        let Some(page) = self.pages.get(&job) else { return };
        let parts: Vec<ArtifactPart> = held.parts.iter().map(|part| effective(part, &part_key(artifact, &part.id), &page.cards.parts)).collect();
        let words = parts.iter().find(|part| is_words_like(part)).cloned();
        let takes = parts.iter().find(|part| is_pick(part)).cloned();
        let asking = parts.iter().any(ArtifactPart::is_open_ask);
        let modifiers = event.keystroke.modifiers;
        let pressed = event.keystroke.key.as_str();
        let take_key = takes.as_ref().map(|part| part_key(artifact, &part.id));
        let large = take_key.as_ref().and_then(|key| page.cards.parts.large.get(key).copied());
        let chosen = take_key.as_ref().and_then(|key| page.cards.parts.chosen.get(key).copied());
        let count = takes.as_ref().map_or(0, |part| match &part.content {
            PartContent::Takes { takes, .. } => takes.len(),
            _ => 0,
        });
        let plain = !modifiers.modified();
        let handled = if pressed == "enter" && modifiers.platform {
            if several_picks(&parts) {
                self.send_picks(artifact, window, cx);
            } else if let Some(key) = take_key.as_deref() {
                self.send_pick(key, window, cx);
            }
            true
        } else if plain && take_of_key(pressed, count).is_some() && take_key.is_some() {
            if let (Some(key), Some(take)) = (take_key.as_deref(), take_of_key(pressed, count)) {
                self.choose_take(key, take, cx);
            }
            true
        } else if plain && pressed == "a" && words.is_some() {
            if let Some(part) = words {
                self.answer_part(artifact, &part, PartHow::Approved, None, None, window, cx);
            }
            true
        } else if plain && pressed == "c" && words.is_some() {
            if let Some(part) = words {
                self.part_ask_for_changes(artifact, &part.id, window, cx);
            }
            true
        } else if plain && pressed == "f" && take_key.is_some() {
            if let Some(key) = take_key.as_deref() {
                self.large_take(key, if large.is_some() { None } else { Some(chosen.unwrap_or(0)) }, cx);
            }
            true
        } else if plain && pressed == "escape" && large.is_some() {
            if let Some(key) = take_key.as_deref() {
                self.large_take(key, None, cx);
            }
            true
        } else if plain && matches!(pressed, "left" | "right") && large.is_some() && count > 0 {
            if let (Some(key), Some(at)) = (take_key.as_deref(), large) {
                let to = if pressed == "left" { (at + count - 1) % count } else { (at + 1) % count };
                if to != at {
                    crate::ui_sounds::moved();
                }
                self.large_take(key, Some(to), cx);
            }
            true
        } else if plain && pressed == "d" {
            if let Some(ix) = self.parts_ix(artifact) {
                if asking {
                    self.dismiss_parts(ix, artifact, window, cx);
                } else {
                    self.archive_card(ix, artifact, artifact, window, cx);
                }
            }
            true
        } else if plain && pressed == "enter" {
            self.open_named(artifact, window, cx);
            true
        } else {
            false
        };
        if handled {
            cx.stop_propagation();
        }
    }

    // ---- the card ----------------------------------------------------------------------------------------------

    /// The card of revision `revision` of artifact `id`: one line for an older revision, or for the newest once nothing is left to
    /// ask of him, else its band, its parts and its footer. In the artifact's own tab (`tab`) it is never folded.
    pub(super) fn parts_card(&self, ix: usize, id: &str, revision: u32, tab: bool, sheet: bool, page: &Page, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let job = self.shown.clone()?;
        let artifact = self.artifact_of(&job, id)?;
        let last = artifact.revisions.last();
        if !tab && last.is_some_and(|last| revision < last.number) {
            return self.past_card(ix, artifact, revision, page, look, cx);
        }
        let parts: Vec<ArtifactPart> = artifact.parts.iter().map(|part| effective(part, &part_key(id, &part.id), &page.cards.parts)).collect();
        let dismissed = page.cards.answers.get(id) == Some(&Answer::Withdrawn);
        let stand = if dismissed { Stand::Dismissed } else { stand_of(&parts, artifact.state) };
        let asking = stand == Stand::Asking;
        // An answer folds the card at once, on the answer itself; a refusal takes the answer back and the card stands whole again.
        if !tab && matches!(stand, Stand::Approved | Stand::SentBack | Stand::Dismissed | Stand::Withdrawn) {
            let verdict = super::parts_fold::Verdict::of(stand, &parts, artifact.words.as_deref());
            let said = last.map_or_else(String::new, |last| revision_said(last.number, last.by, last.at.as_str()));
            let fold = super::parts_fold::Fold { ix, artifact: id, title: &artifact.title, form: artifact.form, revision: last.map_or(1, |last| last.number), numbered: artifact.revisions.len() > 1, verdict, said };
            return Some(self.folded_parts(fold, page, look, cx));
        }
        // Only the Job's first open ask stands as a card, the thread's first card (`first_card`): while this one asks it is one line.
        if !tab && !sheet && asking && self.asking_item(&job, id, cx).is_some() {
            return self.parts_pointer("thread", id, look, cx);
        }
        // In its own tab it is read only, under the same line: no pick, verdict or Dismiss is drawn there.
        let pointer = if tab && !sheet && self.reads_only(id, cx) { self.parts_pointer("reader", id, look, cx) } else { None };
        let read_only = pointer.is_some();
        let parts: Vec<ArtifactPart> = if read_only {
            parts
                .into_iter()
                .map(|mut part| {
                    part.asks = false;
                    part
                })
                .collect()
        } else {
            parts
        };
        let revision = last.map(|last| (last.number, revision_said(last.number, last.by, last.at.as_str())));
        let head = band(
            look,
            Head { key: id, title: &artifact.title, form: artifact.form, stand, words: artifact.words.as_deref(), revision, yours: None, for_line: artifact.for_line.as_ref(), named: None },
        );
        let ctx = Ctx {
            ix,
            id,
            job,
            page,
            look,
            tab,
            sheet,
            words: parts.iter().find(|part| is_words_like(part)).map(|part| part.id.clone()),
            takes: parts.iter().find(|part| is_pick(part)).map(|part| part.id.clone()),
            several: several_picks(&parts),
            prototype: parts.iter().any(|part| matches!(part.content, PartContent::Prototype(_))),
        };
        let blocks: Vec<AnyElement> = parts.iter().enumerate().map(|(at, part)| self.part_block(&ctx, part, at == 0, cx)).collect();
        let spot = page.keys.spot(&format!("{}:{id}", if tab { "reader" } else { "card" }), cx);
        let artifact_id = id.to_string();
        let named = format!("{}: {}, Revision {}", if tab { "Reader" } else { "Artifact" }, artifact.title, last.map_or(1, |last| last.number));
        // In the sheet the parts stand on its own ground: the head, the foot and the frame are the sheet's.
        if sheet {
            let column = div()
                .id(SharedString::from(format!("artifact:{id}")))
                .track_focus(&spot.focus)
                .w_full()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_4))
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| this.parts_key(&artifact_id, event, window, cx)))
                .children(blocks);
            return Some(labelled(column, Role::Region, named).into_any_element());
        }
        let footer = (!read_only).then(|| held_press(self.parts_footer(ix, id, &parts, asking, None, page, look, cx)));
        // A press anywhere on the card that is not on one of its controls, and did not travel, grows it out over the app.
        let (opened, kept, store) = (id.to_string(), id.to_string(), self.pictures.cards.clone());
        let card = frame(look, self.band_narrow(look))
            .id(SharedString::from(format!("artifact:{id}")))
            .relative()
            .track_focus(&spot.focus)
            .in_focus(|style| style.border_color(look.color(token::ACCENT)))
            .cursor_pointer()
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                // Read only, Enter opens the sheet on it and no key answers it here.
                if !read_only {
                    this.parts_key(&artifact_id, event, window, cx);
                } else if !event.keystroke.modifiers.modified() && event.keystroke.key == "enter" {
                    cx.stop_propagation();
                    this.open_artifact_review(&artifact_id, window, cx);
                }
            }))
            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                if let ClickEvent::Mouse(press) = event {
                    let travel = press.up.position - press.down.position;
                    if travel.x.abs() > px(4.) || travel.y.abs() > px(4.) {
                        return;
                    }
                }
                this.open_card(&opened, window, cx);
            }))
            .child(canvas(move |bounds, _, _| { store.borrow_mut().insert(kept.clone(), bounds); }, |_, _, _, _| {}).absolute().size_full())
            .child(head)
            .children(blocks)
            .children(footer);
        let card = labelled(card, if tab { Role::Region } else { Role::Article }, named).into_any_element();
        Some(match pointer {
            Some(pointer) => div().w_full().flex().flex_col().gap(look.length(token::SPACE_2)).child(pointer).child(card).into_any_element(),
            None => card,
        })
    }

    /// The parts of artifact `id` and its footer as its card draws them under its head, for the thread's first card at list
    /// place `ix` while the artifact is the Job's first ask. None once it no longer asks.
    pub(super) fn parts_inside(&self, ix: usize, id: &str, page: &Page, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let job = self.shown.clone()?;
        let artifact = self.artifact_of(&job, id)?;
        let parts: Vec<ArtifactPart> = artifact.parts.iter().map(|part| effective(part, &part_key(id, &part.id), &page.cards.parts)).collect();
        let dismissed = page.cards.answers.get(id) == Some(&Answer::Withdrawn);
        if dismissed || stand_of(&parts, artifact.state) != Stand::Asking {
            return None;
        }
        let ctx = Ctx {
            ix,
            id,
            job,
            page,
            look,
            tab: false,
            sheet: false,
            words: parts.iter().find(|part| is_words_like(part)).map(|part| part.id.clone()),
            takes: parts.iter().find(|part| is_pick(part)).map(|part| part.id.clone()),
            several: several_picks(&parts),
            prototype: parts.iter().any(|part| matches!(part.content, PartContent::Prototype(_))),
        };
        let blocks: Vec<AnyElement> = parts.iter().enumerate().map(|(at, part)| self.part_block(&ctx, part, at == 0, cx)).collect();
        let footer = held_press(self.parts_footer(ix, id, &parts, true, None, page, look, cx));
        let artifact_id = id.to_string();
        Some(
            div()
                .id(SharedString::from(format!("first-parts:{id}")))
                .flex()
                .flex_col()
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| this.parts_key(&artifact_id, event, window, cx)))
                .children(blocks)
                .child(footer)
                .into_any_element(),
        )
    }

    /// The footer of the artifact in the review panel, pinned under its parts and read as the card would read it.
    pub(super) fn parts_pinned_footer(&self, ix: usize, id: &str, page: &Page, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let artifact = self.artifact_of(self.shown.as_ref()?, id)?;
        let parts: Vec<ArtifactPart> = artifact.parts.iter().map(|part| effective(part, &part_key(id, &part.id), &page.cards.parts)).collect();
        let dismissed = page.cards.answers.get(id) == Some(&Answer::Withdrawn);
        let asking = !dismissed && stand_of(&parts, artifact.state) == Stand::Asking;
        // A takes part's pick row stands over the footer. For an artifact of one drawing (drawn whole in the focus view, so its
        // own bar is not drawn), that drawing's Approve and Ask for changes stand in the footer's row beside Dismiss, and only
        // the note box Ask for changes opens takes a row of its own.
        let lone_drawing = match parts.as_slice() {
            [part] if is_words_like(part) && matches!(part.content, PartContent::Drawing { .. }) => Some(part),
            _ => None,
        };
        let lone_noting = lone_drawing.is_some_and(|part| page.cards.modes.get(&part_key(id, &part.id)) == Some(&Mode::Note));
        let inline = lone_drawing.filter(|_| !lone_noting).and_then(|part| self.part_acts(&part_key(id, &part.id), look, cx));
        let footer = self.parts_footer(ix, id, &parts, asking, inline, page, look, cx);
        let above = parts.iter().find(|part| is_pick(part)).filter(|_| !several_picks(&parts)).or(lone_drawing.filter(|_| lone_noting));
        let pick = above.and_then(|part| self.part_acts(&part_key(id, &part.id), look, cx));
        Some(match pick {
            Some(pick) => div()
                .flex()
                .flex_col()
                .child(div().px(look.length(token::SPACE_4)).pt(look.length(token::SPACE_3)).child(pick))
                .child(footer)
                .into_any_element(),
            None => footer,
        })
    }

    /// The card's footer: what settles it, the refusal he last had, `acts` where the footer carries a part's own, and Dismiss or
    /// Archive. In the focus view it is pinned below the reader instead of ending the card.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn parts_footer(&self, ix: usize, id: &str, parts: &[ArtifactPart], asking: bool, acts: Option<AnyElement>, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let run: Do = {
            let id = id.to_string();
            Rc::new(move |this, act, window, cx| match act {
                Act::Dismiss => this.dismiss_parts(ix, &id, window, cx),
                Act::Archive => this.archive_card(ix, &id, &id, window, cx),
                _ => {}
            })
        };
        let foot = Foot { ix, key: id.to_string(), asking, archivable: !asking, openable: false, editable: false, commit: false, route: "/sessions/waiting/dismiss", run };
        let line = settles_line(parts).filter(|_| asking);
        let picks = (asking && several_picks(parts)).then(|| self.send_picks_button(id, look, cx));
        let away = if asking {
            ghost(look, &foot, "dismiss", "Dismiss", "D", "Put it away with no answer", Act::Dismiss, cx)
        } else {
            ghost(look, &foot, "archive", "Archive", "D", "Fold it away from your list", Act::Archive, cx)
        };
        let footer = foot_box(look).children(page.cards.said.get(id).map(|why| refusal(look, id, why.clone()))).child(
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(
                    div()
                        .id(SharedString::from(format!("settles:{id}")))
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(look.length(token::TEXT_2XS))
                        .line_height(look.length(token::LEADING_2XS))
                        .text_color(look.color(token::FG_MUTED))
                        .when_some(line, |one, line| one.tooltip(tip(line.clone())).child(SharedString::from(line))),
                )
                .children(picks)
                .children(acts.map(|acts| div().flex_none().child(acts)))
                .child(div().flex_none().child(away)),
        );
        footer.into_any_element()
    }

    /// Send picks: one button for every vote of the artifact that has a pick. It waits until one does.
    fn send_picks_button(&self, id: &str, look: &Look, cx: &Context<Self>) -> AnyElement {
        let ready = !self.picks_made(id).is_empty();
        let artifact = id.to_string();
        super::acts::gated(
            div().id(SharedString::from(format!("send-picks:{id}"))).flex_none().child(labelled(
                act(look, format!("send-picks-button:{id}"), "Send picks", Kind::Primary, "\u{2318}\u{21b5}", ready).when(ready, |button| {
                    button.tooltip(tip("Send your picks to the Job")).on_click(cx.listener(move |this, _, window, cx| this.send_picks(&artifact, window, cx)))
                }),
                Role::Button,
                "Send picks",
            )),
            self.off_shown_to(ANSWER_PART, cx),
        )
        .into_any_element()
    }

    /// One part: its caption and tag, what it holds and, where it asks, how he answers it.
    fn part_block(&self, c: &Ctx, part: &ArtifactPart, first: bool, cx: &Context<Self>) -> AnyElement {
        let look = c.look;
        let key = part_key(c.id, &part.id);
        let (mark, name) = kind_look(part.content.kind());
        let body = match &part.content {
            PartContent::Words { text } => self.words_body(c, &key, text, part, cx),
            PartContent::Takes { question, form, takes, .. } => self.takes_body(c, &key, part, question.as_deref(), *form, takes, cx),
            PartContent::Drawing { scene } => self.drawing_body(c, &key, part, scene, cx),
            PartContent::Buttons { actions } => self.buttons_body(c, &key, part, actions, cx),
            PartContent::Page { path } => self.page_body(c, &key, part, path, cx),
            PartContent::Prototype(prototype) => self.prototype_line(c, &key, prototype),
        };
        // Takes draw their own pick row, since it is the vote's. The other parts that ask end in Approve or his answer; in the
        // sheet the bar at its foot answers every part at once, so a part draws no Approve of its own (24 Nov: two Approves).
        let answer = match (part.asks, part.state, is_takes(part)) {
            (true, PartState::Open, false) if !c.sheet => Some(self.approve_row(c, &key, part, cx)),
            (true, PartState::Open, false) => None,
            (true, state, false) if state != PartState::Open => Some(self.answered_line(c, &key, part)),
            _ => None,
        };
        let said = c.state().said.get(&key).map(|why| refusal(look, &key, why.clone()));
        // Words, buttons and a still of a page stand at the measure in the sheet; takes, a drawing and a live page take its width.
        let live = c.sheet && matches!(&part.content, PartContent::Page { path } if self.panel_page(&c.job, path, part.revision, cx).is_some());
        let body = if live || matches!(part.content, PartContent::Takes { .. } | PartContent::Drawing { .. } | PartContent::Prototype(_)) { body } else { self.measured(c, body) };
        labelled(
            div()
                .id(SharedString::from(format!("part:{key}")))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_3))
                .when(!c.sheet, |block| block.px(look.length(token::SPACE_4)).py(look.length(token::SPACE_4)))
                .when(c.sheet && !first, |block| block.pt(look.length(token::SPACE_4)))
                .when(!first, |block| block.border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
                .child(self.measured(c, self.caption(c, &key, mark, name, part, cx)))
                .child(if matches!(part.content, PartContent::Words { .. } | PartContent::Drawing { .. }) { body } else { held_press(body) })
                .children(answer.map(|answer| held_press(self.measured(c, answer))))
                .children(said.map(|said| self.measured(c, said))),
            Role::Group,
            name,
        )
        .into_any_element()
    }

    /// Prose in the sheet stands at its measure, at the left, where a picture beside it takes the width; elsewhere it is as it was.
    fn measured(&self, c: &Ctx, element: AnyElement) -> AnyElement {
        if c.sheet {
            div().w_full().max_w(super::reader_sheet::measure(c.look)).child(element).into_any_element()
        } else {
            element
        }
    }

    /// The small caps caption with the kind's glyph, and the tag at the right.
    fn caption(&self, c: &Ctx, key: &str, mark: &'static str, name: &'static str, part: &ArtifactPart, cx: &Context<Self>) -> AnyElement {
        let look = c.look;
        let tag = Tag::of(part);
        let (said, answered) = tag_of(part);
        let ink = match (tag, answered) {
            (_, Some(token)) => look.color(token),
            (Tag::Asks, _) => look.color(token::STATUS_AWAITING_REVIEW),
            (Tag::Acts, _) => look.color(token::KIND_SETUP),
            _ => look.color(token::FG_MUTED),
        };
        let pill = labelled(
            div()
                .id(SharedString::from(format!("tag:{key}")))
                .flex_none()
                .px(look.length(token::SPACE_2))
                .rounded_full()
                .border(look.length(token::BORDER_WIDTH))
                .border_color(if matches!(tag, Tag::Asks | Tag::Acts | Tag::Asked) { ink.opacity(0.5) } else { look.color(token::BORDER_DEFAULT) })
                .when(tag == Tag::Asks, |pill| pill.bg(ink.opacity(0.12)))
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_XS))
                .text_color(ink)
                .tooltip(tip(if answered.is_some() { stands(part.state).3 } else { tag.tip() }))
                .child(SharedString::from(said.clone())),
            Role::Label,
            said,
        );
        let caption = div()
            .id(SharedString::from(format!("caption:{key}")))
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .text_size(look.length(token::TEXT_2XS))
            .line_height(look.length(token::LEADING_2XS))
            .text_color(look.color(token::FG_SUBTLE))
            .child(glyph(look, mark, Size::Small, look.color(token::FG_SUBTLE)))
            .child(name.to_uppercase());
        let copy = matches!(part.content, PartContent::Words { .. }).then(|| self.copy_button(c, key, cx));
        div().flex().items_center().gap(look.length(token::SPACE_2)).child(labelled(caption, Role::Heading, name)).child(pill).children(copy).into_any_element()
    }

    /// Copy at the end of a words part's caption, which puts the words on the pasteboard with their formatting.
    fn copy_button(&self, c: &Ctx, key: &str, cx: &Context<Self>) -> AnyElement {
        let said: &'static str = if c.state().copied.as_deref() == Some(key) { "Copied" } else { "Copy" };
        let (job, held) = (c.job.clone(), key.to_string());
        labelled(
            act(c.look, format!("words-copy:{key}"), said, Kind::Ghost, "", true)
                .tooltip(tip("Copy with formatting"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    // The press is the button's alone, never the card's.
                    cx.stop_propagation();
                    this.copy_words(&job, &held, cx)
                })),
            Role::Button,
            said,
        )
        .into_any_element()
    }

    /// The words of a part put on the pasteboard as markdown and as HTML, and its Copy reading Copied for a moment.
    pub(super) fn copy_words(&mut self, job: &str, key: &str, cx: &mut Context<Self>) {
        let Some((artifact, id)) = split_key(key) else { return };
        let text = self.artifact_of(job, artifact).and_then(|one| one.parts.iter().find(|part| part.id == id)).and_then(|part| match &part.content {
            PartContent::Words { text } => Some(text.clone()),
            _ => None,
        });
        let Some(text) = text else { return };
        crate::pasteboard::copy_words(&text, cx);
        let (artifact, held) = (artifact.to_string(), key.to_string());
        let back = {
            let artifact = artifact.clone();
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(COPIED_FOR).await;
                let _ = this.update(cx, |this, cx| {
                    if let Some(state) = this.parts_state_mut().filter(|state| state.copied.as_deref() == Some(held.as_str())) {
                        state.copied = None;
                    }
                    this.parts_changed(&artifact, cx);
                });
            })
        };
        if let Some(state) = self.parts_state_mut() {
            state.copied = Some(key.to_string());
            state._copying = Some(back);
        }
        self.parts_changed(&artifact, cx);
    }

    // ---- words ---------------------------------------------------------------------------------------------------

    /// Words the Job wrote, as markdown. Long ones are cut as a page's first screen is.
    fn words_body(&self, c: &Ctx, key: &str, text: &str, part: &ArtifactPart, cx: &Context<Self>) -> AnyElement {
        let look = c.look;
        let words = TextView::markdown(gpui::ElementId::Name(format!("words:{}:{key}:{}", c.job, part.drawn).into()), text.to_string())
            .w_full()
            .text_size(look.length(token::TEXT_SM))
            .text_color(look.color(token::FG_DEFAULT));
        // The review panel and the artifact's own tab are where he reads it whole; only the thread's card cuts.
        let long = text.chars().count() > 600 || text.lines().count() > 8;
        if !long || c.tab || c.sheet {
            return div().w_full().child(words).into_any_element();
        }
        let open = c.state().whole.contains(key);
        let said: &'static str = if open { "Show less" } else { "Read more" };
        let held = key.to_string();
        let press = labelled(
            act(look, format!("words-whole:{key}"), said, Kind::Ghost, "", true)
                .tooltip(tip(if open { "Cut the words back to their first screen" } else { "Show the rest of the words here" }))
                .on_click(cx.listener(move |this, _, _, cx| {
                    // The press is the button's alone, never the card's, which would grow the whole card.
                    cx.stop_propagation();
                    this.flip_whole_words(&held, cx)
                })),
            Role::Button,
            said,
        );
        let body = if open {
            div().px(look.length(token::SPACE_4)).py(look.length(token::SPACE_3)).child(words).into_any_element()
        } else {
            well(look, true, words.into_any_element())
        };
        div().w_full().flex().flex_col().child(body).child(div().px(look.length(token::SPACE_4)).flex().child(press)).into_any_element()
    }

    /// Words opened past their cut, or cut back.
    pub(super) fn flip_whole_words(&mut self, key: &str, cx: &mut Context<Self>) {
        if let Some(state) = self.parts_state_mut() {
            if !state.whole.remove(key) {
                state.whole.insert(key.to_string());
            }
        }
        if let Some((artifact, _)) = split_key(key) {
            let artifact = artifact.to_string();
            self.parts_changed(&artifact, cx);
        }
    }

    /// Approve and Ask for changes under words, a drawing or a page that asks, or the note box in their place.
    fn approve_row(&self, c: &Ctx, key: &str, part: &ArtifactPart, cx: &Context<Self>) -> AnyElement {
        let look = c.look;
        let target = c.words.as_deref() == Some(part.id.as_str());
        if c.page.cards.modes.get(key) == Some(&Mode::Note) {
            let (ix, artifact, id, held) = (c.ix, c.id.to_string(), part.id.clone(), key.to_string());
            let run: Do = Rc::new(move |this, act, window, cx| match act {
                Act::Send => {
                    let note = this.typed_in(&held, Mode::Note, cx).trim().to_string();
                    let part = this.shown.clone().and_then(|job| this.artifact_of(&job, &artifact).and_then(|one| one.parts.iter().find(|one| one.id == id)).cloned());
                    if let (false, Some(part)) = (note.is_empty(), part) {
                        this.answer_part(&artifact, &part, PartHow::SentBack, None, Some(note), window, cx);
                        this.focus_card(&artifact, window, cx);
                    }
                }
                Act::Cancel => {
                    this.shut_band_note(ix, &held, window, cx);
                    this.focus_card(&artifact, window, cx);
                }
                _ => {}
            });
            let foot = Foot { ix: c.ix, key: key.to_string(), asking: true, archivable: false, openable: false, editable: false, commit: true, route: ANSWER_PART, run };
            return self.band_note(&foot, c.page, look, cx);
        }
        let (artifact, held) = (c.id.to_string(), part.clone());
        let (ask_artifact, ask_part) = (c.id.to_string(), part.id.clone());
        super::acts::gated(
            div()
                .id(SharedString::from(format!("answers:{key}")))
                .flex()
                .flex_wrap()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(labelled(
                    act(look, format!("approve:{key}"), "Approve", Kind::Approve, if target { "A" } else { "" }, true).tooltip(tip("Approve it as it stands")).on_click(cx.listener(move |this, _, window, cx| {
                        this.answer_part(&artifact, &held, PartHow::Approved, None, None, window, cx);
                    })),
                    Role::Button,
                    "Approve",
                ))
                .child(labelled(
                    act(look, format!("ask:{key}"), "Ask for changes", Kind::Secondary, if target { "C" } else { "" }, true).tooltip(tip("Send it back with a note")).on_click(cx.listener(
                        move |this, _, window, cx| {
                            this.part_ask_for_changes(&ask_artifact, &ask_part, window, cx);
                        },
                    )),
                    Role::Button,
                    "Ask for changes",
                )),
            self.off_shown_to(ANSWER_PART, cx),
        )
        .into_any_element()
    }

    /// What he answered, on one line where the buttons were: its glyph and word in the state's colour, and his words.
    fn answered_line(&self, c: &Ctx, key: &str, part: &ArtifactPart) -> AnyElement {
        let look = c.look;
        let (mark, ink, word, tipped) = stands(part.state);
        let take = picked_take(part);
        let words = part.answer.as_ref().and_then(|answer| answer.words.as_deref()).map(|words| words.split_whitespace().collect::<Vec<_>>().join(" ")).filter(|words| !words.is_empty());
        let said = [take, words].into_iter().flatten().collect::<Vec<_>>().join(": ");
        let (_, kind) = kind_look(part.content.kind());
        let ink = look.color(ink);
        labelled(
            div()
                .id(SharedString::from(format!("answered:{key}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .min_w_0()
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .tooltip(tip(tipped))
                .child(glyph(look, mark, Size::Large, ink))
                .child(div().flex_none().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(ink).child(word))
                .when(!said.is_empty(), |line| {
                    let full = SharedString::from(said.clone());
                    line.child(div().id(SharedString::from(format!("answered-words:{key}"))).flex_1().min_w_0().truncate().text_color(look.color(token::FG_MUTED)).tooltip(tip(full.clone())).child(full))
                }),
            Role::Status,
            format!("{kind} {}", word.to_lowercase()),
        )
        .into_any_element()
    }

    // ---- takes ---------------------------------------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    fn takes_body(&self, c: &Ctx, key: &str, part: &ArtifactPart, question: Option<&str>, form: VoteForm, takes: &[PartTake], cx: &Context<Self>) -> AnyElement {
        let look = c.look;
        let open = part.is_open_ask() && form == VoteForm::Pick;
        let picked = part.answer.as_ref().and_then(|answer| answer.take.as_deref()).and_then(|id| takes.iter().position(|one| one.id == id));
        let chosen = if open { c.state().chosen.get(key).copied().filter(|at| *at < takes.len()) } else { picked };
        let large = c.state().large.get(key).copied().filter(|at| *at < takes.len());
        let target = c.takes.as_deref() == Some(part.id.as_str());
        let question = question.map(|said| {
            div()
                .id(SharedString::from(format!("question:{key}")))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(look.color(token::FG_DEFAULT))
                .child(SharedString::from(said.to_string()))
        });
        let shown = match large {
            Some(at) => self.large_take_view(c, key, takes, at, chosen, open, cx),
            None => {
                // A narrow thread leaves a take too little room for its title: they stand one under another there.
                let across = across_in(f32::from(self.part_room(c)), f32::from(look.length(token::SPACE_3)), takes.len());
                let rows: Vec<AnyElement> = takes
                    .chunks(across)
                    .enumerate()
                    .map(|(row, chunk)| {
                        let tiles: Vec<AnyElement> = chunk.iter().enumerate().map(|(at, take)| self.take_tile(c, key, row * across + at, take, chosen == Some(row * across + at), open, target, cx)).collect();
                        let fill = (0..across - chunk.len()).map(|_| div().flex_1().min_w_0().into_any_element());
                        div().flex().gap(look.length(token::SPACE_3)).children(tiles).children(fill).into_any_element()
                    })
                    .collect();
                div().flex().flex_col().gap(look.length(token::SPACE_3)).children(rows).into_any_element()
            }
        };
        // In the review panel the words box and Send pick stand in its foot beside the answers, so the pick is never off screen.
        let send = if c.several { open.then(|| self.note_row(c, key)) } else { (open && !c.sheet).then(|| self.pick_row(c, key, chosen.is_some(), target, cx)) };
        let answered = (!part.is_open_ask() && part.asks).then(|| self.answered_line(c, key, part));
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .children(question.map(|question| self.measured(c, question.into_any_element())))
            .child(shown)
            .children(send.map(|send| self.measured(c, send)))
            .children(answered.map(|answered| self.measured(c, answered)))
            .into_any_element()
    }

    /// One take: its radio and title, the way to see it larger, and its picture. A press chooses it.
    #[allow(clippy::too_many_arguments)]
    fn take_tile(&self, c: &Ctx, key: &str, at: usize, take: &PartTake, on: bool, open: bool, target: bool, cx: &Context<Self>) -> AnyElement {
        let look = c.look;
        let (title, marked) = pick_mark(&take.title);
        // A take that cannot be picked here is named by its title alone, so nothing reads as a pick he can make.
        let name = if on {
            format!("Chosen: {}", take.title)
        } else if open {
            format!("Pick {}", take.title)
        } else {
            format!("Take {}", take.title)
        };
        let radio = div()
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .size(look.length(token::SPACE_4))
            .rounded_full()
            .border(look.length(token::BORDER_WIDTH) * 2.)
            .border_color(look.color(if on { token::ACCENT } else { token::BORDER_STRONG }))
            .when(on, |radio| radio.child(div().size(look.length(token::SPACE_2)).rounded_full().bg(look.color(token::ACCENT))));
        let expand_key = key.to_string();
        let expand = labelled(
            div()
                .id(SharedString::from(format!("expand:{key}:{at}")))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL_SM))
                .rounded(look.length(token::RADIUS_MD))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tab_index(0)
                .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
                .tooltip(tip(if target { "See it larger, F" } else { "See it larger" }))
                .child(glyph(look, "eye", Size::Small, look.color(token::FG_MUTED)))
                .on_key_down(|event: &KeyDownEvent, _, cx| presses_on_enter(event, cx))
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.large_take(&expand_key, Some(at), cx);
                    this.focus_card_of(&expand_key, window, cx);
                })),
            Role::Button,
            format!("See {title} larger"),
        );
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .child(radio)
            .child(
                div()
                    .id(SharedString::from(format!("take-title:{key}:{at}")))
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_DEFAULT))
                    .tooltip(tip(SharedString::from(take.title.clone())))
                    .child(div().min_w_0().truncate().child(SharedString::from(title.to_string())))
                    .when(marked, |line| line.child(div().flex_none().text_color(look.color(token::FG_MUTED)).child("(Job's pick)"))),
            )
            .child(expand);
        // In the sheet the picture fills its tile edge to edge; in a card it stands in the tile's own padding.
        let thumb = self.take_flat(c, key, at, take, cx).map(|picture| if c.sheet { div().child(picture) } else { div().px(look.length(token::SPACE_3)).pb(look.length(token::SPACE_3)).child(picture) });
        let (press_key, key_key) = (key.to_string(), key.to_string());
        let tile = div()
            .id(SharedString::from(format!("take:{key}:{at}")))
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(look.length(token::RADIUS_LG))
            .bg(look.color(token::BG_RAISED))
            .border(look.length(token::BORDER_WIDTH) * 2.)
            .border_color(if on { look.color(token::ACCENT) } else { look.color(token::BORDER_SUBTLE) })
            .when(on, |tile| tile.shadow(look.shadows(token::SHADOW_CARD)))
            .child(head)
            .children(thumb)
            .when(open, |tile| {
                tile.cursor_pointer()
                    .hover(|style| style.border_color(if on { look.color(token::ACCENT) } else { look.color(token::BORDER_STRONG) }))
                    .tab_index(0)
                    .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                        if !event.keystroke.modifiers.modified() && matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            cx.stop_propagation();
                            this.choose_take(&key_key, at, cx);
                        }
                    }))
                    .on_click(cx.listener(move |this, _, _, cx| this.choose_take(&press_key, at, cx)))
            });
        labelled(tile, Role::RadioButton, name).into_any_element()
    }

    /// What a part's body has to stand in, in the reader or a thread card: the card's width less the part's own padding.
    fn part_room(&self, c: &Ctx) -> gpui::Pixels {
        let look = c.look;
        if c.sheet {
            return (self.sheet.get().width - look.length(token::SPACE_6) * 2. - look.length(token::BORDER_WIDTH) * 2.).max(px(1.));
        }
        let held = if c.tab { self.body_scroll.bounds().size.width } else { self.pane.unwrap_or(px(0.)) };
        let card = if held > px(0.) { (held - look.length(token::SPACE_4) * 2.).min(self.column(look)) } else { self.column(look) };
        (card - look.length(token::SPACE_4) * 2. - look.length(token::BORDER_WIDTH) * 2.).max(px(1.))
    }

    /// The tallest a picture stands in the reading column: three fifths of the pane it is read in.
    fn tall_most(&self, c: &Ctx) -> gpui::Pixels {
        if c.sheet {
            return self.sheet.get().height * super::reader_sheet::TALL_SHARE;
        }
        let tab = c.tab;
        let pane = if tab { self.body_scroll.bounds().size.height } else { px(0.) };
        let pane = if pane > px(0.) { pane } else { self.pane_height.unwrap_or(px(600.)) };
        pane * TALL_SHARE
    }

    /// A take's picture filling the reading column: the width of its place, the height its own shape gives, no taller than
    /// `tall_most`, and no frame round it.
    fn take_flat(&self, c: &Ctx, key: &str, at: usize, take: &PartTake, cx: &Context<Self>) -> Option<AnyElement> {
        let text = take.scene.as_deref()?;
        let held = format!("part:{}:{key}:{at}:{}:column", c.job, text.len());
        let picture = self.scene_column(&held, text, self.tall_most(c), c.look, c.sheet)?;
        Some(self.take_press(c, key, at, take, picture, cx))
    }

    /// A take's picture made a button: a press reads it large over the app, and a press beside it still chooses the take. In the
    /// review panel the press chooses the take, as an ask never opens over the app.
    fn take_press(&self, c: &Ctx, key: &str, at: usize, take: &PartTake, picture: AnyElement, cx: &Context<Self>) -> AnyElement {
        let press = key.to_string();
        let sheet = c.sheet.then(|| c.job.to_string());
        labelled(
            div()
                .id(SharedString::from(format!("take-picture:{key}:{at}")))
                .w_full()
                .cursor_pointer()
                .child(picture)
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    match &sheet {
                        Some(job) => this.as_job(job, |this| this.choose_take(&press, at, cx)),
                        None => this.open_take(&press, at, window, cx),
                    }
                })),
            Role::Button,
            format!("Picture: {}", pick_mark(&take.title).0),
        )
        .into_any_element()
    }

    /// A scene's JSON drawn to fill its width: it stands as tall as its canvas's own shape asks, up to `tall`, and is centred
    /// where that leaves room at the sides.
    fn scene_column(&self, key: &str, text: &str, tall: gpui::Pixels, look: &Look, flat: bool) -> Option<AnyElement> {
        let raw: ipc::RawScene = serde_json::from_str(text).ok()?;
        let scene = crate::scene::read(key, &raw)?;
        // The canvas's shape is known once it has been drawn; the frame before that draws at a plain 16 to 10 and draws again.
        let canvas = crate::scene::canvas_size(key);
        let ratio = canvas.map_or(1.6, |(w, h)| (w / h).clamp(0.2, 8.));
        // No wider than the drawing stands at its most (its own size, or twice it), so a chip-sized mock is not given the column.
        let widest = canvas.map_or(f32::INFINITY, |(w, _)| w * Fit::Column.most_zoom(&scene));
        Some(
            div()
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .relative()
                        .w_full()
                        .max_w((tall * ratio).min(px(widest)))
                        .aspect_ratio(ratio)
                        .overflow_hidden()
                        .when(!flat, |picture| picture.rounded(look.length(token::RADIUS_MD)))
                        .child(crate::scene::view(SharedString::from(key.to_string()), scene, Fit::Column, Clock::default(), Lens::default())),
                )
                .into_any_element(),
        )
    }

    /// One take drawn large: the take before, this one, the take after and Back, then the picture.
    #[allow(clippy::too_many_arguments)]
    fn large_take_view(&self, c: &Ctx, key: &str, takes: &[PartTake], at: usize, chosen: Option<usize>, open: bool, cx: &Context<Self>) -> AnyElement {
        let look = c.look;
        let take = &takes[at];
        let (title, marked) = pick_mark(&take.title);
        let (prev, next) = ((at + takes.len() - 1) % takes.len(), (at + 1) % takes.len());
        // A neighbour is named by its own name, in a fixed column that the title never enters.
        let step = |to: usize, arrow: &'static str, cap: &'static str, before: bool| {
            let (held, name) = (key.to_string(), pick_mark(&takes[to].title).0.to_string());
            let said = if before { format!("{arrow} {name}") } else { format!("{name} {arrow}") };
            labelled(
                div()
                    .id(SharedString::from(format!("large-step:{key}:{to}")))
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .when(!before, |one| one.justify_end())
                    .h(look.length(token::H_CONTROL))
                    .px(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                    .tooltip(tip(SharedString::from(name.clone())))
                    .child(div().min_w_0().truncate().child(SharedString::from(said)))
                    .child(super::acts::kbd(look, cap))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.large_take(&held, Some(to), cx);
                        this.focus_card_of(&held, window, cx);
                    })),
                Role::Button,
                name,
            )
        };
        let (back_key, pick_key) = (key.to_string(), key.to_string());
        let on = chosen == Some(at);
        let whole = SharedString::from(title.to_string());
        let heading = div()
            .id(SharedString::from(format!("large-title:{key}")))
            .w_full()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
            .text_color(look.color(token::FG_DEFAULT))
            .tooltip(tip(whole.clone()))
            .when(on, |line| line.child(glyph(look, "check", Size::Small, look.color(token::ACCENT))))
            .child(div().min_w_0().line_clamp(2).child(whole))
            .when(marked, |line| line.child(div().flex_none().text_color(look.color(token::FG_MUTED)).child("(Job's pick)")));
        let heading = labelled(heading, Role::Heading, title.to_string());
        let bar = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .child(heading)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .children((takes.len() > 1).then(|| step(prev, "\u{2039}", "\u{2190}", true)))
                    .children((takes.len() > 1).then(|| step(next, "\u{203a}", "\u{2192}", false)))
                    .child(labelled(
                        act(look, format!("large-back:{key}"), "Back", Kind::Ghost, "Esc", true).tooltip(tip("Back to the takes")).on_click(cx.listener(move |this, _, window, cx| {
                            this.large_take(&back_key, None, cx);
                            this.focus_card_of(&back_key, window, cx);
                        })),
                        Role::Button,
                        "Back",
                    )),
            );
        let picture = self.take_flat(c, key, at, take, cx);
        let choose = open.then(|| {
            labelled(
                act(look, format!("large-pick:{key}"), if on { "Chosen" } else { "Choose this take" }, Kind::Secondary, "", true)
                    .tooltip(tip("Choose this take, then Send pick"))
                    .on_click(cx.listener(move |this, _, _, cx| this.choose_take(&pick_key, at, cx))),
                Role::Button,
                format!("Choose {title}"),
            )
        });
        div().flex().flex_col().gap(look.length(token::SPACE_3)).child(bar).children(picture).children(choose).into_any_element()
    }

    /// The note that goes with a pick, in a box of its own: under each vote where several ask.
    fn note_row(&self, c: &Ctx, key: &str) -> AnyElement {
        div().id(SharedString::from(format!("pick-note:{key}"))).flex().items_center().children(self.note_field(c, key)).into_any_element()
    }

    fn note_field(&self, c: &Ctx, key: &str) -> Option<AnyElement> {
        let look = c.look;
        c.state().notes.get(key).map(|input| {
            labelled(
                div()
                    .id(SharedString::from(format!("note:{key}")))
                    .flex_1()
                    .min_w_0()
                    .h(look.length(token::H_CONTROL))
                    .flex()
                    .items_center()
                    .px(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_SUNKEN))
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(Input::new(input).appearance(false).cleanable(false)),
                Role::TextInput,
                "Note on your pick",
            )
            .into_any_element()
        })
    }

    /// The note beside the pick and Send pick, which sends it: the vote's one act.
    fn pick_row(&self, c: &Ctx, key: &str, ready: bool, target: bool, cx: &Context<Self>) -> AnyElement {
        let look = c.look;
        let field = self.note_field(c, key);
        let send_key = key.to_string();
        super::acts::gated(
            div().id(SharedString::from(format!("pick:{key}"))).flex().items_center().gap(look.length(token::SPACE_2)).children(field).child(labelled(
                act(look, format!("send-pick:{key}"), "Send pick", Kind::Primary, if target { "\u{2318}\u{21b5}" } else { "" }, ready).when(ready, |button| {
                    button.tooltip(tip("Send your pick to the Job")).on_click(cx.listener(move |this, _, window, cx| this.send_pick(&send_key, window, cx)))
                }),
                Role::Button,
                "Send pick",
            )),
            self.off_shown_to(ANSWER_PART, cx),
        )
        .into_any_element()
    }

    // ---- drawing, buttons and page -----------------------------------------------------------------------------

    /// A drawing: its scene, in a sunken box. A press on it, or its eye, reads it large in the artifact's own tab, where it zooms.
    fn drawing_body(&self, c: &Ctx, key: &str, part: &ArtifactPart, scene: &str, cx: &Context<Self>) -> AnyElement {
        let look = c.look;
        let held = format!("part:{}:{key}:{}", c.job, part.drawn);
        if c.tab {
            return self.drawing_large(c, key, &held, scene, cx);
        }
        let Some(drawn) = self.scene_column(&format!("{held}:column"), scene, self.tall_most(c), look, c.sheet) else { return div().into_any_element() };
        let (artifact, press) = (c.id.to_string(), key.to_string());
        let grows = !c.sheet && !c.tab;
        let name = format!("See {} larger", kind_look(part.content.kind()).1.to_lowercase());
        let eye = labelled(
            div()
                .id(SharedString::from(format!("expand:{key}")))
                .absolute()
                .top(look.length(token::SPACE_2))
                .right(look.length(token::SPACE_2))
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL_SM))
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(token::BG_RAISED).opacity(0.85))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tab_index(0)
                .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                .tooltip(tip("See it larger"))
                .child(glyph(look, "eye", Size::Small, look.color(token::FG_MUTED)))
                .on_key_down(|event: &KeyDownEvent, _, cx| presses_on_enter(event, cx))
                .on_click(cx.listener({
                    let artifact = artifact.clone();
                    move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.open_named(&artifact, window, cx);
                    }
                })),
            Role::Button,
            name,
        );
        labelled(
            div()
                .id(SharedString::from(format!("drawing:{key}")))
                .relative()
                .w_full()
                .cursor_pointer()
                .child(drawn)
                .child(eye)
                .on_click(cx.listener(move |this, _, window, cx| {
                    // In the thread the drawing grows out of its card, as a press anywhere on the card does.
                    cx.stop_propagation();
                    if grows {
                        this.grow_drawing(&press, window, cx);
                    } else {
                        this.open_drawing(&press, window, cx);
                    }
                })),
            Role::Button,
            "Drawing",
        )
        .into_any_element()
    }

    /// The drawing read large in the artifact's tab: the width of the column, as tall as its own shape asks.
    fn drawing_large(&self, c: &Ctx, key: &str, held: &str, scene: &str, cx: &Context<Self>) -> AnyElement {
        // In the review panel one drawing zooms where it stands, the first until another is pressed; a press on another moves the
        // zoom to it, as an ask never opens over the app. Outside the panel a press reads it over the app.
        if c.sheet && self.in_place_drawing(&c.job, c.id).as_deref() == Some(key) {
            if let Some(drawn) = self.drawing_in_place(key, &format!("{held}:column"), scene, self.part_room(c), self.tall_most(c), c.look, cx) {
                return drawn;
            }
        }
        let Some(drawn) = self.scene_column(&format!("{held}:column"), scene, self.tall_most(c), c.look, c.sheet) else { return div().into_any_element() };
        let (press, sheet) = (key.to_string(), c.sheet);
        labelled(
            div().id(SharedString::from(format!("drawing:{key}"))).w_full().cursor_pointer().child(drawn).on_click(cx.listener(move |this, _, window, cx| {
                cx.stop_propagation();
                if sheet {
                    this.zoom_drawing_in_place(&press, cx);
                } else {
                    this.open_drawing(&press, window, cx);
                }
            })),
            Role::Button,
            "Drawing",
        )
        .into_any_element()
    }

    /// Buttons in the Kept look: a row each, its words at the left and the button in a column of its own at the right. A
    /// pressed one reads what was sent, on the row's green edge.
    fn buttons_body(&self, c: &Ctx, key: &str, part: &ArtifactPart, actions: &[PartAction], cx: &Context<Self>) -> AnyElement {
        let look = c.look;
        let held = self.artifact_of(&c.job, c.id);
        let green = look.color(token::STATUS_COMPLETED_SUCCESS);
        let rows: Vec<AnyElement> = actions
            .iter()
            .map(|action| {
                let sent = c.state().went.get(&format!("{key}{JOIN}{}", action.id)).cloned().or_else(|| {
                    held.and_then(|one| one.presses.iter().rev().find(|press| press.action == action.id && press.revision == part.revision)).map(|press| match &press.did {
                        PressDid::Sent => format!("Sent to the Job: {}", action.label),
                        PressDid::Started { .. } => format!("{}: started a Job", action.label),
                        PressDid::Answered { .. } => format!("{}: answered its ask", action.label),
                        PressDid::Closed { .. } => format!("{}: closed in its tracker", action.label),
                    })
                });
                let (artifact, owner, pressed) = (c.id.to_string(), part.clone(), action.clone());
                let button = labelled(
                    div()
                        .id(SharedString::from(format!("press:{key}:{}", action.id)))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .h(look.length(token::H_CONTROL))
                        .px(look.length(token::SPACE_3))
                        .rounded(look.length(token::RADIUS_MD))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_STRONG))
                        .bg(look.color(token::BG_OVERLAY))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                        .text_color(look.color(token::FG_DEFAULT))
                        .cursor_pointer()
                        .hover(|style| style.bg(look.color(token::BG_HOVER)))
                        .tab_index(0)
                        .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
                        .tooltip(tip(match action.act {
                            ipc::PageAct::Message => "Sends it to the Job",
                            ipc::PageAct::Start => "Starts a Job",
                            ipc::PageAct::Answer => "Answers the Job's ask",
                            ipc::PageAct::Close => "Closes the issues it lists in its tracker",
                        }))
                        .child(SharedString::from(action.label.clone()))
                        .on_key_down(|event: &KeyDownEvent, _, cx| presses_on_enter(event, cx))
                        .on_click(cx.listener(move |this, _, window, cx| this.press_button(&artifact, &owner, &pressed, window, cx))),
                    Role::Button,
                    action.label.clone(),
                );
                let words = div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_1))
                    .child(div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(action.item.clone().unwrap_or_default())))
                    .children(sent.clone().map(|said| {
                        labelled(
                            div()
                                .id(SharedString::from(format!("sent:{key}:{}", action.id)))
                                .flex()
                                .items_center()
                                .gap(look.length(token::SPACE_1))
                                .text_size(look.length(token::TEXT_2XS))
                                .line_height(look.length(token::LEADING_2XS))
                                .text_color(green)
                                .tooltip(tip("The Job has it"))
                                .child(glyph(look, "check", Size::Small, green))
                                .child(SharedString::from(said.clone())),
                            Role::Status,
                            said,
                        )
                    }));
                div()
                    .id(SharedString::from(format!("action:{key}:{}", action.id)))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_MD))
                    .when(sent.is_some(), |row| row.bg(mixed(look, token::STATUS_COMPLETED_SUCCESS, 0.13)).border_l(look.length(token::BORDER_WIDTH) * 3.).border_color(green))
                    .child(words)
                    .child(super::acts::gated(
                        div().id(SharedString::from(format!("button-column:{key}:{}", action.id))).flex_none().w(look.length(token::SPACE_12) * 2.).flex().justify_end().child(button),
                        self.off_shown_to(PRESS, cx),
                    ))
                    .into_any_element()
            })
            .collect();
        div().flex().flex_col().gap(look.length(token::SPACE_1)).children(rows).into_any_element()
    }

    /// A page: in the review panel the live page itself, the detail's width and as tall as a picture stands there, which scrolls
    /// inside; elsewhere its first screen in a well, which a press opens in its own tab.
    /// A prototype: in the review sheet the scene itself, drawn as the Job's Prototype tab draws it, with its states, Open tab and Pop out;
    /// elsewhere its scene's file name.
    fn prototype_line(&self, c: &Ctx, key: &str, prototype: &ipc::PrototypePart) -> AnyElement {
        let look = c.look;
        let scene = c.sheet.then(|| self.row_tabs.get(&c.job).and_then(|tabs| tabs.sheets.get(c.id))).flatten();
        if let Some((view, _)) = scene {
            return div()
                .id(SharedString::from(format!("prototype:{key}")))
                .w_full()
                .h(self.tall_most(c))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .child(view.clone())
                .into_any_element();
        }
        div()
            .id(SharedString::from(format!("prototype:{key}")))
            .font_family(look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default())
            .text_size(look.length(token::TEXT_XS))
            .text_color(look.color(token::FG_MUTED))
            .child(SharedString::from(prototype.scene.clone()))
            .into_any_element()
    }

    fn page_body(&self, c: &Ctx, key: &str, part: &ArtifactPart, path: &str, cx: &Context<Self>) -> AnyElement {
        let look = c.look;
        if let Some(live) = c.sheet.then(|| self.panel_page(&c.job, path, part.revision, cx)).flatten() {
            return div()
                .id(SharedString::from(format!("page:{key}")))
                .w_full()
                .h(self.tall_most(c))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .child(live)
                .into_any_element();
        }
        let width = self.first_screen_width(look);
        let want = PictureKey { job: c.job.to_string(), page: path.to_string(), revision: part.revision, width, dark: look.tone() == crate::look::Tone::Dark };
        let screen = match self.picture_of(&want) {
            Some(Picture::Ready(image, height)) => {
                let (image, height) = (image.clone(), *height);
                Some(well(look, height > f32::from(most(look)) - f32::from(look.length(token::SPACE_3)) * 2., gpui::img(image).w(px(width as f32)).h(px(height)).rounded(look.length(token::RADIUS_SM)).into_any_element()))
            }
            Some(Picture::Failed) => None,
            _ => Some(self.asking_picture(&want, well(look, true, shapes(look)), cx)),
        };
        let page = path.to_string();
        labelled(
            div().id(SharedString::from(format!("page:{key}"))).w_full().cursor_pointer().children(screen).on_click(cx.listener(move |this, _, window, cx| this.open_document(&page, window, cx))),
            Role::Button,
            format!("Open page {path}"),
        )
        .into_any_element()
    }
}

// ---- the review sheet's ask ------------------------------------------------------------------------------------

impl JobView {
    /// The vote the review sheet answers for an artifact ask: its first takes part still asking for a pick, by key. None for an
    /// ask that is no artifact.
    pub(super) fn sheet_vote(&self, item: &Item) -> Option<String> {
        let artifact = artifact_in(item)?;
        let page = self.pages.get(item.job.as_str())?;
        let held = self.artifact_of(&item.job, artifact)?;
        held.parts.iter().find(|part| is_pick(&effective(part, &part_key(artifact, &part.id), &page.cards.parts))).map(|part| part_key(artifact, &part.id))
    }

    /// What an artifact ask reads as on the sheet: a vote where any part is takes, else a drawing, a page or words.
    pub(super) fn artifact_ask(&self, item: &Item) -> Ask {
        let Some(held) = artifact_in(item).and_then(|artifact| self.artifact_of(&item.job, artifact)) else { return Ask::Draft };
        let any = |is: fn(&PartContent) -> bool| held.parts.iter().any(|part| is(&part.content));
        if any(|content| matches!(content, PartContent::Takes { .. })) {
            Ask::Takes
        } else if any(|content| matches!(content, PartContent::Drawing { .. })) {
            Ask::Drawing
        } else if any(|content| matches!(content, PartContent::Page { .. })) {
            Ask::Page
        } else {
            Ask::Draft
        }
    }

    /// The sheet's answer to an artifact ask: every part still asking approved, and each vote sent with the take chosen; or, with
    /// `words`, every part still asking sent back with them. False where nothing was sent: a vote with no take chosen holds the
    /// whole answer back. The parts' acts answer the shown Job, so another Job's ask is answered under `as_job`.
    pub(super) fn sheet_answer_parts(&mut self, item: &Item, words: Option<(String, ipc::AnswerExtras)>, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(artifact) = artifact_in(item).map(str::to_string) else { return false };
        if self.shown.as_deref() != Some(item.job.as_str()) {
            return false;
        }
        let (Some(held), Some(page)) = (self.artifact_of(&item.job, &artifact), self.pages.get(item.job.as_str())) else { return false };
        let asking: Vec<ArtifactPart> = held.parts.iter().map(|part| effective(part, &part_key(&artifact, &part.id), &page.cards.parts)).filter(ArtifactPart::is_open_ask).collect();
        if asking.is_empty() {
            return false;
        }
        match words {
            Some((words, extras)) => {
                // What he attached goes once, with the first part sent back: the Job reads one message for each part.
                let mut extras = Some(extras);
                for part in &asking {
                    self.answer_part_with(&artifact, part, PartHow::SentBack, None, Some(words.clone()), extras.take(), window, cx);
                }
            }
            None => {
                let unchosen = asking.iter().any(|part| is_pick(part) && self.takes_of_key(&part_key(&artifact, &part.id)).is_none_or(|(_, _, chosen)| chosen.is_none()));
                if unchosen {
                    return false;
                }
                for part in &asking {
                    if is_pick(part) {
                        self.send_pick(&part_key(&artifact, &part.id), window, cx);
                    } else if is_words_like(part) {
                        self.answer_part(&artifact, part, PartHow::Approved, None, None, window, cx);
                    }
                }
            }
        }
        true
    }

    /// The takes of an artifact ask for the review sheet: a strip of small cards across the top, the Job's pick first, and the
    /// take he is on drawn large under it to fill the sheet (his pick on "Zoom for takes", 24 Nov: three side by side were too
    /// small to read). A press on a card, a digit, or ← and → puts that take large and, while the vote is open, chooses it; the
    /// bar's answer sends it. Fleet's sentence stands under them where it refused the last answer.
    pub(super) fn sheet_takes(&self, item: &Item, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let artifact = artifact_in(item)?;
        let job = SharedString::from(item.job.clone());
        let page = self.pages.get(&job)?;
        let held = self.artifact_of(&job, artifact)?;
        let part = held.parts.iter().map(|part| effective(part, &part_key(artifact, &part.id), &page.cards.parts)).find(is_takes)?;
        let PartContent::Takes { takes, form, .. } = &part.content else { return None };
        if takes.is_empty() {
            return None;
        }
        let key = part_key(artifact, &part.id);
        let c = Ctx { ix: super::acts::SHEET, id: artifact, job: job.clone(), page, look, tab: false, sheet: true, words: None, takes: Some(part.id.clone()), several: false, prototype: false };
        let open = part.is_open_ask() && *form == VoteForm::Pick;
        let picked = part.answer.as_ref().and_then(|answer| answer.take.as_deref()).and_then(|id| takes.iter().position(|one| one.id == id));
        let chosen = if open { c.state().chosen.get(&key).copied().filter(|at| *at < takes.len()) } else { picked };
        let order = sheet_order(takes);
        let shown = sheet_shown(&order, c.state().large.get(&key).copied(), chosen);
        let gap = look.length(token::SPACE_3);
        let wide = (self.part_room(&c) - gap * (order.len() as f32 - 1.)) / order.len() as f32;
        let cards: Vec<AnyElement> = order.iter().map(|at| self.sheet_take(&c, &key, *at, &takes[*at], chosen == Some(*at), *at == shown, open, wide, cx)).collect();
        let strip = labelled(
            div().id(SharedString::from(format!("sheet-strip:{key}"))).w_full().flex().items_stretch().gap(gap).children(cards),
            if open { Role::RadioGroup } else { Role::Group },
            item.name.clone(),
        );
        let place = order.iter().position(|at| *at == shown).unwrap_or(0);
        let large = self.sheet_large(&c, &key, shown, &takes[shown], place, takes.len());
        let said = c.state().said.get(&key).map(|why| refusal(look, &key, why.clone()));
        // A settled vote is read, not answered: the take he picked stands over the takes, which carry no radios.
        let settled = (!open).then_some(picked).flatten().map(|at| {
            let line = format!("Picked: {}", pick_mark(&takes[at].title).0);
            labelled(
                div()
                    .id(SharedString::from(format!("sheet-picked:{key}")))
                    .text_size(look.length(token::TEXT_SM))
                    .line_height(look.length(token::LEADING_SM))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(SharedString::from(line.clone())),
                Role::Status,
                line,
            )
        });
        Some(div().id(SharedString::from(format!("sheet-takes:{key}"))).w_full().flex().flex_col().gap(gap).children(settled).child(strip).child(large).children(said).into_any_element())
    }

    /// The take he is on, drawn as large as the sheet's width and height let it, with which of how many it is, its title and,
    /// where there are others, the keys that switch.
    fn sheet_large(&self, c: &Ctx, key: &str, at: usize, take: &PartTake, place: usize, count: usize) -> AnyElement {
        let look = c.look;
        let (title, marked) = pick_mark(&take.title);
        let held = format!("part:{}:{key}:{at}:{}:large", c.job, take.scene.as_deref().map_or(0, str::len));
        let tall = self.sheet.get().height * LARGE_SHARE;
        let picture = take.scene.as_deref().and_then(|text| self.scene_column(&held, text, tall, look, false));
        let well = div()
            .id(SharedString::from(format!("sheet-large:{key}")))
            .w_full()
            .p(look.length(token::SPACE_4))
            .flex()
            .items_center()
            .justify_center()
            .rounded(look.length(token::RADIUS_LG))
            .bg(look.color(token::SURFACE_WELL))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .children(picture);
        let name = format!("{} of {count} \u{b7} {title}", place + 1);
        let chip = marked.then(|| self.sheet_pick_chip(look, &format!("sheet-large-pick:{key}")));
        let caption = div()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_3))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_2))
                    .child(
                        div()
                            .min_w_0()
                            .text_size(look.length(token::TEXT_SM))
                            .line_height(look.length(token::LEADING_SM))
                            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                            .text_color(look.color(token::FG_DEFAULT))
                            .child(SharedString::from(name.clone())),
                    )
                    .children(chip),
            )
            .when(count > 1, |caption| {
                caption.child(
                    div()
                        .flex_none()
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_SM))
                        .text_color(look.color(token::FG_MUTED))
                        .child("\u{2190} \u{2192} to switch"),
                )
            });
        labelled(div().id(SharedString::from(format!("sheet-large-take:{key}"))).w_full().flex().flex_col().gap(look.length(token::SPACE_2)).child(well).child(caption), Role::Group, name)
            .into_any_element()
    }

    /// The chip that marks the take the Job would pick.
    fn sheet_pick_chip(&self, look: &Look, id: &str) -> AnyElement {
        labelled(
            div()
                .id(SharedString::from(id.to_string()))
                .flex_none()
                .px(look.length(token::SPACE_2))
                .rounded_full()
                .bg(look.color(token::ACCENT).opacity(0.14))
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(look.color(token::ACCENT))
                .tooltip(tip("The take the Job would pick"))
                .child("Job's pick"),
            Role::Status,
            "Job's pick",
        )
        .into_any_element()
    }

    /// One take in the strip: a small picture in a well, then its radio, its title wrapped whole and, on the Job's pick, a chip.
    /// The one drawn large wears the accent outline; a press puts it large and, while the vote is open, chooses it.
    #[allow(clippy::too_many_arguments)]
    fn sheet_take(&self, c: &Ctx, key: &str, at: usize, take: &PartTake, on: bool, shown: bool, open: bool, wide: gpui::Pixels, cx: &Context<Self>) -> AnyElement {
        let look = c.look;
        let (title, marked) = pick_mark(&take.title);
        let name = match (open, on) {
            (true, true) => format!("Chosen: {title}"),
            (true, false) => format!("Pick {title}"),
            (false, true) => format!("Picked: {title}"),
            (false, false) => title.to_string(),
        };
        let radio = div()
            .flex_none()
            .mt(look.length(token::SPACE_1) / 2.)
            .flex()
            .items_center()
            .justify_center()
            .size(look.length(token::SPACE_4))
            .rounded_full()
            .border(look.length(token::BORDER_WIDTH) * 2.)
            .border_color(look.color(if on { token::ACCENT } else { token::BORDER_STRONG }))
            .when(on, |radio| radio.child(div().size(look.length(token::SPACE_2)).rounded_full().bg(look.color(token::ACCENT))));
        let chip = marked.then(|| self.sheet_pick_chip(look, &format!("sheet-take-pick:{key}:{at}")));
        let held = format!("part:{}:{key}:{at}:{}:sheet", c.job, take.scene.as_deref().map_or(0, str::len));
        let small = (wide * 0.4).min(look.length(token::SPACE_6) * 4.);
        let picture = take.scene.as_deref().and_then(|text| self.scene_column(&held, text, small, look, false));
        // Every well is one height, the picture centred in it, so the cards stand level whatever their shapes.
        let well = picture.map(|picture| {
            div()
                .m(look.length(token::SPACE_2))
                .mb_0()
                .p(look.length(token::SPACE_1))
                .h(small + look.length(token::SPACE_1) * 2.)
                .flex()
                .items_center()
                .justify_center()
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(token::SURFACE_WELL))
                .child(picture)
        });
        let head = div()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .p(look.length(token::SPACE_2))
            .when(open, |head| head.child(radio))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(SharedString::from(title.to_string())),
            )
            .children(chip);
        let (press_key, key_key) = (key.to_string(), key.to_string());
        // A take of another Job's ask is chosen on that Job's page.
        let (press_job, key_job) = (c.job.to_string(), c.job.to_string());
        let hovered = look.color(if shown { token::ACCENT } else { token::BORDER_STRONG });
        let card = div()
            .id(SharedString::from(format!("sheet-take:{key}:{at}")))
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(look.length(token::RADIUS_LG))
            .bg(look.color(token::SURFACE_CARD))
            .border(look.length(token::BORDER_WIDTH) * if shown { 2. } else { 1. })
            .border_color(look.color(if shown { token::ACCENT } else { token::BORDER_SUBTLE }))
            .when(shown, |card| card.shadow(look.shadows(token::SHADOW_CARD)))
            .children(well)
            .child(head)
            .cursor_pointer()
            .hover(move |style| style.border_color(hovered))
            .tab_index(0)
            .focus_visible(|style| style.border_color(look.color(token::ACCENT)))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                if !event.keystroke.modifiers.modified() && matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    cx.stop_propagation();
                    this.as_job(&key_job, |this| this.sheet_view_take(&key_key, at, open, cx));
                }
            }))
            .on_click(cx.listener(move |this, _, _, cx| this.as_job(&press_job, |this| this.sheet_view_take(&press_key, at, open, cx))));
        labelled(card, if open { Role::RadioButton } else { Role::Button }, name).into_any_element()
    }

    /// Put a take large in the sheet and, while its vote is open, choose it.
    pub(super) fn sheet_view_take(&mut self, key: &str, at: usize, open: bool, cx: &mut Context<Self>) {
        if open {
            if let Some(state) = self.parts_state_mut() {
                state.chosen.insert(key.to_string(), at);
            }
        }
        self.large_take(key, Some(at), cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(id: &str, content: PartContent, asks: bool, state: PartState) -> ArtifactPart {
        ArtifactPart { id: id.into(), content, asks, state, revision: 1, drawn: format!("{id}-1"), added: ipc::Instant::carried("2026-11-14T10:00:00Z"), answer: None, question: None }
    }

    fn words(id: &str, asks: bool, state: PartState) -> ArtifactPart {
        part(id, PartContent::Words { text: "words".into() }, asks, state)
    }

    fn takes(state: PartState) -> ArtifactPart {
        let take = |id: &str| PartTake { id: id.into(), title: id.into(), scene: None };
        part("vote", PartContent::Takes { question: None, form: VoteForm::Pick, takes: vec![take("a"), take("b")], parts: Vec::new() }, true, state)
    }

    #[test]
    fn the_band_reads_asking_until_every_ask_is_answered_then_the_state_it_settled_in() {
        let open = [words("w", true, PartState::Open), takes(PartState::Open)];
        assert_eq!(stand_of(&open, ArtifactState::Asking), Stand::Asking);
        let one_in = [words("w", true, PartState::Approved), takes(PartState::Open)];
        assert_eq!(stand_of(&one_in, ArtifactState::Asking), Stand::Asking, "the takes still ask");
        let all_in = [words("w", true, PartState::Approved), takes(PartState::Picked)];
        assert_eq!(stand_of(&all_in, ArtifactState::Asking), Stand::Approved, "his answers settle it before Fleet's list says so");
        assert_eq!(stand_of(&all_in, ArtifactState::Approved), Stand::Approved);
        let sent_back = [words("w", true, PartState::SentBack), takes(PartState::Picked)];
        assert_eq!(stand_of(&sent_back, ArtifactState::Asking), Stand::SentBack);
        assert_eq!(stand_of(&all_in, ArtifactState::Dismissed), Stand::Dismissed, "a state Fleet settled stands as it is");
    }

    #[test]
    fn a_vote_he_dismissed_asks_no_more_though_its_takes_are_still_open() {
        let open = [takes(PartState::Open)];
        assert_eq!(stand_of(&open, ArtifactState::Dismissed), Stand::Dismissed, "Fleet keeps the parts open and reads the artifact Dismissed");
    }

    #[test]
    fn the_strip_puts_the_jobs_pick_first_and_arrows_walk_it_wrapping() {
        let take = |title: &str| PartTake { id: title.into(), title: title.into(), scene: None };
        let takes = [take("A"), take("B (Job's pick)"), take("C")];
        let order = sheet_order(&takes);
        assert_eq!(order, [1, 0, 2], "the Job's pick leads, the rest keep their order");
        assert_eq!(sheet_shown(&order, None, None), 1, "with nothing chosen the pick is drawn large");
        assert_eq!(sheet_shown(&order, None, Some(2)), 2, "the chosen take is drawn large");
        assert_eq!(sheet_shown(&order, Some(0), Some(2)), 0, "the one he put large stands over the chosen");
        assert_eq!(sheet_shown(&order, Some(9), None), 1, "a place past the end falls back to the first");
        assert_eq!(sheet_step(&order, 1, 1), 0);
        assert_eq!(sheet_step(&order, 2, 1), 1, "→ from the last wraps to the first");
        assert_eq!(sheet_step(&order, 1, -1), 2, "← from the first wraps to the last");
    }

    #[test]
    fn a_part_that_only_informs_never_holds_the_band_asking() {
        let parts = [words("w", false, PartState::Open), part("flow", PartContent::Drawing { scene: "{}".into() }, false, PartState::Open)];
        assert_eq!(stand_of(&parts, ArtifactState::Reading), Stand::Reading);
    }

    #[test]
    fn his_answer_reads_on_the_revision_he_answered_and_not_on_one_the_job_drew_since() {
        let mut state = PartsState::default();
        let held = words("w", true, PartState::Open);
        state.sent.insert(part_key("a", "w"), (held.drawn.clone(), PartState::Approved, PartAnswer::default()));
        assert_eq!(effective(&held, &part_key("a", "w"), &state).state, PartState::Approved);
        let redrawn = ArtifactPart { drawn: "w-2".into(), ..held.clone() };
        assert_eq!(effective(&redrawn, &part_key("a", "w"), &state).state, PartState::Open, "the Job put it again: it asks again");
        let answered = ArtifactPart { state: PartState::SentBack, ..held };
        assert_eq!(effective(&answered, &part_key("a", "w"), &state).state, PartState::SentBack, "Fleet's own answer wins");
    }

    #[test]
    fn a_key_chooses_the_take_it_names_and_none_past_the_last() {
        assert_eq!(take_of_key("1", 3), Some(0));
        assert_eq!(take_of_key("3", 3), Some(2));
        assert_eq!(take_of_key("4", 3), None);
        assert_eq!(take_of_key("0", 3), None);
        assert_eq!(take_of_key("a", 3), None);
        assert_eq!(take_of_key("1", 0), None, "a card with no takes asking has nothing to choose");
    }

    #[test]
    fn the_jobs_pick_is_the_words_it_wrote_in_the_title() {
        assert_eq!(pick_mark("B Rail (Job's pick)"), ("B Rail", true));
        assert_eq!(pick_mark("A Bar"), ("A Bar", false));
    }

    #[test]
    fn takes_stand_one_a_row_until_each_would_be_five_hundred_and_twenty_wide_side_by_side() {
        let gap = 12.;
        assert_eq!(across_in(700., gap, 3), 1, "a column of seven hundred has no room for two");
        assert_eq!(across_in(1051., gap, 3), 1, "one short of two takes of 520 and the gap between");
        assert_eq!(across_in(1052., gap, 3), 2);
        assert_eq!(across_in(1600., gap, 3), 3);
        assert_eq!(across_in(2600., gap, 2), 2, "never more across than there are takes");
        assert_eq!(across_in(1600., gap, 4), 2, "four stand two and two");
        assert_eq!(across_in(1052., gap, 4), 2);
        assert_eq!(across_in(700., gap, 4), 1);
    }

    #[test]
    fn the_footer_names_what_still_asks_and_says_too_once_something_was_answered() {
        let open = [words("w", true, PartState::Open), takes(PartState::Open)];
        assert_eq!(settles_line(&open).as_deref(), Some("Settles when the words and the takes are answered"));
        let one_in = [words("w", true, PartState::Approved), takes(PartState::Open)];
        assert_eq!(settles_line(&one_in).as_deref(), Some("Settles when the takes are answered too"));
        let done = [words("w", true, PartState::Approved), takes(PartState::Picked)];
        assert_eq!(settles_line(&done), None);
    }

    #[test]
    fn a_part_is_tagged_as_it_informs_asks_or_acts() {
        assert_eq!(Tag::of(&words("w", false, PartState::Open)), Tag::Informs);
        assert_eq!(Tag::of(&words("w", true, PartState::Open)), Tag::Asks);
        assert_eq!(Tag::of(&words("w", true, PartState::Approved)), Tag::Asked, "answered, it is no longer the amber ask");
        assert_eq!(Tag::of(&part("b", PartContent::Buttons { actions: Vec::new() }, false, PartState::Open)), Tag::Acts);
        assert_eq!([Tag::Informs, Tag::Asks, Tag::Acts].map(Tag::word), ["Informs", "Asks you", "Acts"]);
    }

    #[test]
    fn an_answered_part_tags_how_it_stands_and_never_asks_you() {
        let answered = crate::mock_parts::artifact(true);
        let tags: Vec<(String, String)> = answered.parts.iter().filter(|part| part.asks).map(|part| (part.id.clone(), tag_of(part).0)).collect();
        assert_eq!(tags, [("intro".to_string(), "Approved".to_string()), ("bar".to_string(), "Picked B \u{b7} Rail".to_string()), ("flow".to_string(), "Approved".to_string())]);
        assert!(answered.parts.iter().all(|part| tag_of(part).0 != "Asks you"), "no answered part is still the amber ask");
        let (_, green) = tag_of(&answered.parts[0]);
        assert_eq!(green, Some(token::STATUS_COMPLETED_SUCCESS), "approved in the one-line answer's green");
        assert_eq!(tag_of(&words("w", true, PartState::SentBack)), ("Sent back".to_string(), Some(token::STATUS_REJECTED)));
        assert_eq!(tag_of(&words("w", true, PartState::Open)), ("Asks you".to_string(), None), "one still asking keeps its amber tag");
    }

    #[test]
    fn approve_answers_words_and_never_takes() {
        assert!(is_words_like(&words("w", true, PartState::Open)));
        assert!(!is_words_like(&words("w", false, PartState::Open)), "words that only inform have nothing to answer");
        assert!(!is_words_like(&takes(PartState::Open)), "takes are a vote: Fleet refuses an Approve on them");
        assert!(is_pick(&takes(PartState::Open)));
        assert!(!is_pick(&takes(PartState::Picked)));
    }

    #[test]
    fn a_part_is_kept_under_its_artifact_and_its_own_id() {
        let key = part_key("sketch:Step bar", "bar");
        assert_eq!(split_key(&key), Some(("sketch:Step bar", "bar")));
        assert_ne!(part_key("a", "b/c"), part_key("a/b", "c"));
    }

    use super::super::parts_fold::{drawn, drawn_like, folds_job, redraw};
    use crate::mock_parts::JOB;

    fn held_part(view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext, artifact: &str, part: &str) -> ArtifactPart {
        view.read_with(cx, |view, _| view.artifact_of(JOB, artifact).and_then(|one| one.parts.iter().find(|held| held.id == part)).cloned().expect("the artifact holds the part"))
    }

    #[gpui::test]
    fn long_words_on_the_thread_card_open_whole_with_read_more_and_cut_back_with_show_less(cx: &mut gpui::TestAppContext) {
        let (view, cx) = folds_job(cx, |_| {});
        assert!(!drawn(cx, gpui::Role::Button, "Read more"), "short words stand whole with no Read more");
        view.update(cx, |view, cx| {
            let part = view.artifact_mut(JOB, "draft:Commit words").and_then(|one| one.parts.first_mut()).expect("the card holds its words");
            part.content = PartContent::Words { text: (1..=14).map(|line| format!("Line {line} of a long reply.")).collect::<Vec<_>>().join("\n\n") };
            view.parts_changed("draft:Commit words", cx);
        });
        redraw(cx);
        assert!(drawn(cx, gpui::Role::Button, "Read more"), "long words are cut and say so");
        let key = part_key("draft:Commit words", "commit");
        view.update(cx, |view, cx| view.flip_whole_words(&key, cx));
        redraw(cx);
        assert!(drawn(cx, gpui::Role::Button, "Show less"), "opened, the words stand whole");
        assert!(!drawn(cx, gpui::Role::Button, "Read more"));
        view.update(cx, |view, cx| view.flip_whole_words(&key, cx));
        redraw(cx);
        assert!(drawn(cx, gpui::Role::Button, "Read more"), "Show less cuts them back");
    }

    #[gpui::test]
    fn copy_on_words_puts_them_on_the_clipboard_and_reads_copied_for_a_moment(cx: &mut gpui::TestAppContext) {
        let (view, cx) = folds_job(cx, |_| {});
        assert!(drawn(cx, gpui::Role::Button, "Copy"), "words carry Copy");
        assert!(!drawn(cx, gpui::Role::Button, "Copied"));
        let key = part_key("draft:Commit words", "commit");
        view.update(cx, |view, cx| view.copy_words(JOB, &key, cx));
        redraw(cx);
        assert!(drawn(cx, gpui::Role::Button, "Copied"), "the press says it took");
        let text = match held_part(&view, cx, "draft:Commit words", "commit").content {
            PartContent::Words { text } => text,
            _ => unreachable!("the commit part is words"),
        };
        assert_eq!(cx.read_from_clipboard().and_then(|item| item.text()), Some(text), "the markdown as written");
        cx.executor().advance_clock(COPIED_FOR);
        redraw(cx);
        assert!(drawn(cx, gpui::Role::Button, "Copy"), "and goes back to Copy");
        assert!(!drawn(cx, gpui::Role::Button, "Copied"));
    }

    #[gpui::test]
    fn approve_folds_the_card_to_one_line_with_its_state_word_and_open(cx: &mut gpui::TestAppContext) {
        let (view, cx) = folds_job(cx, |_| {});
        assert!(!drawn(cx, gpui::Role::Status, "Commit words: Approved"), "asking, the card stands whole");
        let part = held_part(&view, cx, "draft:Commit words", "commit");
        view.update_in(cx, |view, window, cx| view.answer_part("draft:Commit words", &part, PartHow::Approved, None, None, window, cx));
        redraw(cx);
        assert!(drawn(cx, gpui::Role::Status, "Commit words: Approved"));
        assert!(drawn(cx, gpui::Role::Article, "Artifact: Commit words, Revision 1"));
        assert!(drawn(cx, gpui::Role::Button, "Open"), "Open is on the line");
    }

    #[gpui::test]
    fn send_back_folds_the_card_with_his_note_and_the_job_drawing_it_again_is_a_new_card(cx: &mut gpui::TestAppContext) {
        let (view, cx) = folds_job(cx, |_| {});
        let part = held_part(&view, cx, "draft:Page title", "title");
        view.update_in(cx, |view, window, cx| view.answer_part("draft:Page title", &part, PartHow::SentBack, None, Some("Name it for the question.".into()), window, cx));
        redraw(cx);
        assert!(drawn(cx, gpui::Role::Status, "Page title: Sent back"), "revision 1 stands as one line");
        assert!(view.read_with(cx, |view, cx| view.asking_item(JOB, "draft:Page title", cx).is_some()), "the Job's next drawing asks again");
        // Only the Job's first ask stands whole; a later one is a line in the thread until its turn.
        let first = view.read_with(cx, |view, cx| view.first_ask(JOB, cx).map(|item| super::super::first_card::first_key(&item)));
        assert_eq!(drawn(cx, gpui::Role::Article, "Artifact: Page title, Revision 2"), first.as_deref() == Some("draft:Page title"), "revision 2 is a card of its own only where it is first: {first:?}");
        view.read_with(cx, |view, cx| {
            let verdict = view.verdict_kept(JOB, "draft:Page title", 1, cx).expect("his words were kept under revision 1");
            assert_eq!(verdict.note.as_deref(), Some("Name it for the question."));
        });
    }

    #[gpui::test]
    fn send_pick_folds_the_card_naming_the_take(cx: &mut gpui::TestAppContext) {
        let (view, cx) = folds_job(cx, |_| {});
        let key = part_key("sketch:Rollout order", "order");
        view.update_in(cx, |view, _, cx| view.choose_take(&key, 1, cx));
        view.update_in(cx, |view, window, cx| view.send_pick(&key, window, cx));
        redraw(cx);
        let mut said = drawn_like(cx, gpui::Role::Status, "Rollout order: Picked");
        said.dedup();
        assert_eq!(said, ["Rollout order: Picked B \u{b7} Votes first"], "the fold names the take he picked");
    }

    #[gpui::test]
    fn a_vote_is_picked_on_its_thread_card_and_leaves_the_sheet(cx: &mut gpui::TestAppContext) {
        // The Job put the vote first, so the first card holds it.
        let (view, cx) = folds_job(cx, |jobs| {
            let job = jobs.iter_mut().find(|job| job.id.as_str() == JOB).expect("the Job");
            let wire = br#"{"id":"agent-rollout","text":"Pick the rollout order","since":"2026-11-14T10:05:00Z","source":"agent","artifact":"sketch:Rollout order","order":1,"why":"The rollout waits on your pick"}"#;
            job.waiting_for.insert(0, ipc::decode::<ipc::WaitingItem>("vote item", wire).expect("the vote item decodes"));
        });
        let held = |view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext| view.read_with(cx, |view, _| view.artifact_items(JOB, None).iter().any(|one| one.artifact.as_deref() == Some("sketch:Rollout order")));
        assert!(held(&view, cx), "the sheet holds the vote while it waits");
        let card = labelled_bounds(cx, gpui::Role::Article, "Artifact: Rollout order");
        // The take's head, where its radio stands: the picture under it opens the take larger.
        let take = labelled_in(cx, gpui::Role::RadioButton, "Pick B", card);
        cx.simulate_click(take.origin + gpui::point(px(16.), px(16.)), gpui::Modifiers::none());
        redraw(cx);
        let card = labelled_bounds(cx, gpui::Role::Article, "Artifact: Rollout order");
        let send = labelled_in(cx, gpui::Role::Button, "Send pick", card);
        cx.simulate_click(send.center(), gpui::Modifiers::none());
        redraw(cx);
        assert!(!view.read_with(cx, |view, _| view.review_open()), "it answers in place, with no sheet");
        let mut said = drawn_like(cx, gpui::Role::Status, "Rollout order: Picked");
        said.dedup();
        assert_eq!(said, ["Rollout order: Picked B \u{b7} Votes first"], "the card folds naming the take he picked");
        assert!(!held(&view, cx), "picked on its card, the vote leaves the sheet");
    }

    /// The bounds of the first `role` whose name starts `prefix`.
    fn labelled_bounds(cx: &mut gpui::VisualTestContext, role: gpui::Role, prefix: &str) -> gpui::Bounds<gpui::Pixels> {
        cx.update(|window, cx| cx.global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().find(|one| one.role == role && one.name.starts_with(prefix)).map(|one| one.bounds))
            .unwrap_or_else(|| panic!("{prefix} was not drawn"))
    }

    /// The bounds of the `role` whose name starts `prefix`, drawn inside `card`.
    fn labelled_in(cx: &mut gpui::VisualTestContext, role: gpui::Role, prefix: &str, card: gpui::Bounds<gpui::Pixels>) -> gpui::Bounds<gpui::Pixels> {
        cx.update(|window, cx| {
            cx.global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().filter(|one| one.role == role && one.name.starts_with(prefix)).map(|one| one.bounds).find(|one| card.contains(&one.center()))
        })
        .unwrap_or_else(|| panic!("{prefix} was not drawn in its card"))
    }

    #[gpui::test]
    fn dismiss_folds_the_card_reading_dismissed(cx: &mut gpui::TestAppContext) {
        let (view, cx) = folds_job(cx, |_| {});
        let ix = view.read_with(cx, |view, _| view.parts_ix("sketch:Step flow")).expect("the card has a place");
        view.update_in(cx, |view, window, cx| view.dismiss_parts(ix, "sketch:Step flow", window, cx));
        redraw(cx);
        assert!(drawn(cx, gpui::Role::Status, "Step flow: Dismissed"));
    }

    #[gpui::test]
    fn a_refusal_unfolds_the_card_and_keeps_fleets_sentence(cx: &mut gpui::TestAppContext) {
        let (view, cx) = folds_job(cx, |_| {});
        // Sent back with no note: the mock Fleet refuses it, as Fleet does.
        let part = held_part(&view, cx, "draft:Commit words", "commit");
        view.update_in(cx, |view, window, cx| view.answer_part("draft:Commit words", &part, PartHow::SentBack, None, None, window, cx));
        redraw(cx);
        assert!(!drawn(cx, gpui::Role::Status, "Commit words: Sent back"), "the card stands whole again");
        view.read_with(cx, |view, _| {
            let page = view.pages.get(JOB).expect("the page");
            assert!(page.cards.parts.said.get(&part_key("draft:Commit words", "commit")).is_some_and(|why| why.contains("note")), "Fleet's own sentence stands on the part");
        });
    }
}
