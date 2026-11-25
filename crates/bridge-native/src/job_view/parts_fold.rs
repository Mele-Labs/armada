//! Where each revision of an artifact of parts stands in the thread, and the one line a card folds to once he has answered
//! it (owner's rulings, 15 Nov 2026).
//!
//! **Each revision's card sits at the time its revision was kept**, among the thread's items, so what the Job did since
//! flows below it. A card the thread has nothing after stays among the extras at its foot; one with items after it is drawn
//! under the item before it (`place`). **An answer folds the card at once**, on the answer itself, and Fleet's refusal
//! unfolds it. Only the newest revision asks: an older one is one line, with what he said of it where this Mac kept that.

use std::rc::Rc;

use chrono::{DateTime, FixedOffset};
use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString, Window};
use ipc::{ArtifactForm, ArtifactPart, Instant, PartState, ThreadItem};

use super::band_card::{ghost, kind_look, revision_chip, revision_said, Act, Do, Foot, Stand};
use super::parts_card::{picked_take, stand_of};
use super::rows::tip;
use super::{JobView, Page};
use crate::labels::labelled;
use crate::look::{token, Look};

/// The separator between the three fields a verdict is kept as: its state, the word it reads as, and his note.
const JOIN: char = '\u{1f}';
/// How many characters of his note the folded line holds before its ellipsis.
const NOTE_START: usize = 80;

/// One card of a revision, drawn under item `item` of the thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Anchor {
    pub item: usize,
    pub artifact: String,
    pub revision: u32,
}

/// When an item began, which is where a card kept at that time stands against it.
fn started(item: &ThreadItem) -> Option<&Instant> {
    match item {
        ThreadItem::Row { row } => Some(row.at()),
        ThreadItem::Calls { rows, .. } | ThreadItem::Checks { rows, .. } => rows.first().map(ipc::SessionRow::at),
        ThreadItem::Subagent { at, .. } | ThreadItem::File { at, .. } | ThreadItem::Change { at, .. } | ThreadItem::NotStarted { at, .. } | ThreadItem::Judged { at, .. } => Some(at),
        ThreadItem::FleetTurn { notice, .. } => Some(notice.at()),
        ThreadItem::Sketch(card) => Some(&card.at),
        ThreadItem::Document(card) => Some(&card.at),
        ThreadItem::ReadyToTry(card) => Some(&card.at),
        ThreadItem::Margin(card) => Some(&card.at),
        ThreadItem::Plan(_) => None,
    }
}

fn when(at: &Instant) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(at.as_str()).ok()
}

/// Each card, kept at `at`, placed: under the last item that began by then where later items stand, else at the foot of the
/// thread. A card older than every item stands under the first. The tail is what the extras draw.
pub(super) fn place(items: &[ThreadItem], cards: &[(String, u32, Instant)]) -> (Vec<Anchor>, Vec<(String, u32)>) {
    let mut ordered: Vec<&(String, u32, Instant)> = cards.iter().collect();
    ordered.sort_by(|a, b| (when(&a.2), &a.0, a.1).cmp(&(when(&b.2), &b.0, b.1)));
    let (mut anchors, mut tail) = (Vec::new(), Vec::new());
    for (artifact, revision, at) in ordered {
        let then = when(at);
        let mut last = None;
        let mut later = false;
        for (ix, item) in items.iter().enumerate() {
            let Some(start) = started(item).and_then(when) else { continue };
            if then.is_some_and(|then| start <= then) {
                last = Some(ix);
            } else {
                later = true;
                break;
            }
        }
        match (later, items.is_empty()) {
            (true, false) => anchors.push(Anchor { item: last.unwrap_or(0), artifact: artifact.clone(), revision: *revision }),
            _ => tail.push((artifact.clone(), *revision)),
        }
    }
    (anchors, tail)
}

/// The first item whose cards differ between two placings: the list measures again from there.
pub(super) fn first_difference(before: &[Anchor], now: &[Anchor]) -> Option<usize> {
    before.iter().filter(|one| !now.contains(one)).chain(now.iter().filter(|one| !before.contains(one))).map(|one| one.item).min()
}

/// What a folded card says: the state's colour, the words on its chip, and his note where he sent it back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Verdict {
    pub stand: Stand,
    pub word: String,
    pub note: Option<String>,
}

impl Verdict {
    /// What an older revision reads as where nothing was kept of how he answered it.
    pub(super) fn replaced() -> Self {
        Verdict { stand: Stand::Reading, word: "Replaced".into(), note: None }
    }

    /// How he settled the artifact: the state's word, or the take he picked where that was the one answer; his note on a send back.
    pub(super) fn of(stand: Stand, parts: &[ArtifactPart], words: Option<&str>) -> Self {
        let answered: Vec<&ArtifactPart> = parts.iter().filter(|part| part.asks && !matches!(part.state, PartState::Open | PartState::Withdrawn)).collect();
        let word = match answered.as_slice() {
            [only] if only.state == PartState::Picked && stand == Stand::Approved => picked_take(only).map_or_else(|| stand.word().to_string(), |take| format!("Picked {take}")),
            _ => stand.word().to_string(),
        };
        let note = (stand == Stand::SentBack)
            .then(|| parts.iter().filter(|part| part.state == PartState::SentBack).find_map(|part| part.answer.as_ref().and_then(|answer| answer.words.clone())).or_else(|| words.map(str::to_string)))
            .flatten();
        Verdict { stand, word, note }
    }

    fn kept(&self) -> String {
        format!("{}{JOIN}{}{JOIN}{}", self.stand.word(), self.word, self.note.as_deref().unwrap_or(""))
    }

    fn read(kept: &str) -> Option<Self> {
        let mut parts = kept.splitn(3, JOIN);
        let (stand, word, note) = (parts.next()?, parts.next()?, parts.next()?);
        let stand = [Stand::Approved, Stand::SentBack, Stand::Dismissed, Stand::Withdrawn, Stand::Reading].into_iter().find(|one| one.word() == stand)?;
        Some(Verdict { stand, word: word.to_string(), note: (!note.is_empty()).then(|| note.to_string()) })
    }
}

/// The key a revision's verdict is kept under, in the same file as his unsent words, so a restart finds it.
pub(super) fn kept_key(job: &str, artifact: &str, revision: u32) -> String {
    format!("fold:{job}:{artifact}:{revision}")
}

/// A note on one line, cut with an ellipsis where it goes on.
pub(super) fn note_start(words: &str) -> String {
    let line = words.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.chars().count() > NOTE_START {
        format!("{}\u{2026}", line.chars().take(NOTE_START).collect::<String>().trim_end())
    } else {
        line
    }
}

/// What a folded card draws.
pub(super) struct Fold<'a> {
    pub ix: usize,
    pub artifact: &'a str,
    pub title: &'a str,
    pub form: ArtifactForm,
    pub revision: u32,
    /// Whether the artifact has more than one revision, so the line says which this is.
    pub numbered: bool,
    pub verdict: Verdict,
    /// What the revision chip's tooltip says: who kept it and when.
    pub said: String,
}

impl JobView {
    /// His verdict on revision `revision`, as this Mac kept it when he gave it.
    pub(super) fn verdict_kept(&self, job: &str, artifact: &str, revision: u32, cx: &gpui::App) -> Option<Verdict> {
        crate::drafts::kept(&kept_key(job, artifact, revision), cx).and_then(|kept| Verdict::read(&kept))
    }

    /// Keep what he said of `revision`, for the line it is when the Job draws the next.
    pub(super) fn keep_verdict(&self, job: &str, artifact: &str, revision: u32, verdict: &Verdict, cx: &mut gpui::App) {
        crate::drafts::keep(&kept_key(job, artifact, revision), &verdict.kept(), cx);
    }

    /// What the newest revision's answers settle it as, kept under its number. Nothing is kept while an ask is open.
    pub(super) fn keep_settled(&self, job: &str, artifact: &str, parts: &[ArtifactPart], words: Option<&str>, cx: &mut gpui::App) {
        let Some(newest) = self.artifact_of(job, artifact).and_then(|one| one.revisions.last()).map(|last| last.number) else { return };
        let stand = stand_of(parts, ipc::ArtifactState::Asking);
        if matches!(stand, Stand::Approved | Stand::SentBack) {
            self.keep_verdict(job, artifact, newest, &Verdict::of(stand, parts, words), cx);
        }
    }

    /// One line for a card that has nothing left to ask: its kind's glyph, the title, the revision, where it stands, his note
    /// where he sent it back, and Open. A press anywhere on it opens the artifact; Enter and Space do where it holds the keys.
    pub(super) fn folded_parts(&self, fold: Fold, page: &Page, look: &Look, cx: &Context<Self>) -> AnyElement {
        let Fold { ix, artifact, title, form, revision, numbered, verdict, said } = fold;
        let key = format!("{artifact}:r{revision}");
        let ink = look.color(verdict.stand.ink());
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
                .tooltip(tip(verdict.stand.tip()))
                .child(gpui::svg().path(crate::assets::icon_path("circle-dot")).flex_none().size(look.length(token::TEXT_2XS)).text_color(ink))
                .child(SharedString::from(verdict.word.clone())),
            Role::Status,
            format!("{title}: {}", verdict.word),
        )
        .into_any_element();
        let narrow = self.band_narrow(look);
        let note = verdict.note.as_deref().map(note_start).filter(|note| !note.is_empty());
        let run: Do = {
            let named = artifact.to_string();
            Rc::new(move |this, act, window, cx| {
                if act == Act::Open {
                    this.open_named(&named, window, cx);
                }
            })
        };
        let foot = Foot { ix, key: key.clone(), asking: false, archivable: false, openable: true, editable: false, commit: false, route: "", run: run.clone() };
        let open = div().flex_none().ml_auto().child(ghost(look, &foot, "open", "Open", "\u{21b5}", "Read it on its own", Act::Open, cx)).into_any_element();
        let mut open = Some(open);
        let parts = vec![
            div()
                .id(SharedString::from(format!("band-title:{key}")))
                .flex_none()
                .max_w(gpui::relative(0.45))
                .truncate()
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .tooltip(tip(title.to_string()))
                .child(SharedString::from(title.to_string()))
                .into_any_element(),
        ]
        .into_iter()
        .chain(numbered.then(|| revision_chip(look, format!("revision:{key}"), revision, said)))
        .chain([chip])
        .chain(open.take_if(|_| narrow))
        .chain([
            div()
                .id(SharedString::from(format!("band-words:{key}")))
                .when(narrow, |one| one.w_full())
                .when(!narrow, |one| one.flex_1().min_w_0())
                .truncate()
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(token::FG_MUTED))
                .when_some(verdict.note.clone().filter(|_| note.is_some()), |one, full| one.tooltip(tip(full)))
                .children(note.map(SharedString::from))
                .into_any_element(),
        ])
        .collect::<Vec<_>>();
        let spot = page.keys.spot(&format!("card:{key}"), cx);
        let (pressed, keyed) = (run.clone(), run);
        let line = self
            .quiet_base(&format!("artifact:{key}"), kind_look(form).0, parts, look)
            .track_focus(&spot.focus)
            .in_focus(|style| style.border_color(look.color(token::ACCENT)))
            .cursor_pointer()
            .hover(|style| style.border_color(look.color(token::BORDER_STRONG)))
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.modifiers.modified() || !matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    return;
                }
                cx.stop_propagation();
                keyed(this, Act::Open, window, cx);
            }))
            .on_click(cx.listener(move |this, _, window: &mut Window, cx| pressed(this, Act::Open, window, cx)))
            .when(narrow, |line| line.flex_wrap())
            .children(open);
        labelled(line, Role::Article, format!("Artifact: {title}, Revision {revision}")).into_any_element()
    }

    /// The card of an older revision: one line, with what he said of it where that was kept.
    pub(super) fn past_card(&self, ix: usize, artifact: &ipc::JobArtifact, revision: u32, page: &Page, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let job = self.shown.clone()?;
        let kept = artifact.revisions.iter().find(|one| one.number == revision)?;
        let verdict = self.verdict_kept(&job, &artifact.id, revision, cx).unwrap_or_else(Verdict::replaced);
        let said = revision_said(kept.number, kept.by, kept.at.as_str());
        Some(self.folded_parts(Fold { ix, artifact: &artifact.id, title: &artifact.title, form: artifact.form, revision, numbered: true, verdict, said }, page, look, cx))
    }
}

/// The folds' Job on screen in a window tall enough to draw its whole thread, its thread read and drawn.
#[cfg(test)]
pub(super) fn folds_job<'a>(cx: &'a mut gpui::TestAppContext, tune: impl FnOnce(&mut Vec<ipc::SessionRecord>)) -> (gpui::Entity<JobView>, &'a mut gpui::VisualTestContext) {
    rooted_job(cx, "artifact-folds", crate::mock_parts::JOB, gpui::size(gpui::px(1400.), gpui::px(6000.)), tune)
}

/// A window's one view, drawn under the label root as the app draws it: the book of what was drawn is cleared each frame,
/// so a test can ask that something is not drawn.
#[cfg(test)]
pub(super) struct Rooted(gpui::Entity<JobView>);

#[cfg(test)]
impl Rooted {
    pub(super) fn of(view: gpui::Entity<JobView>) -> Self {
        Rooted(view)
    }
}

#[cfg(test)]
impl gpui::Render for Rooted {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        crate::labels::label_root(div().size_full().child(self.0.clone()))
    }
}

/// A scenario's Job on screen at `size` under the label root, its thread read and drawn.
#[cfg(test)]
pub(super) fn rooted_job<'a>(
    cx: &'a mut gpui::TestAppContext,
    scenario: &str,
    job: &str,
    size: gpui::Size<gpui::Pixels>,
    tune: impl FnOnce(&mut Vec<ipc::SessionRecord>),
) -> (gpui::Entity<JobView>, &'a mut gpui::VisualTestContext) {
    rooted_job_with(cx, scenario, job, size, tune, |_| {})
}

/// The same, with the scenario's threads changed by `threads` before the Job opens.
#[cfg(test)]
pub(super) fn rooted_job_with<'a>(
    cx: &'a mut gpui::TestAppContext,
    scenario: &str,
    job: &str,
    size: gpui::Size<gpui::Pixels>,
    tune: impl FnOnce(&mut Vec<ipc::SessionRecord>),
    tune_threads: impl FnOnce(&mut std::collections::HashMap<String, ipc::JobThread>),
) -> (gpui::Entity<JobView>, &'a mut gpui::VisualTestContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::look::init(cx);
    });
    let mut jobs = crate::mock::jobs(scenario).expect("the fixture decodes");
    tune(&mut jobs);
    let mut threads = crate::mock::threads(scenario).expect("the threads decode");
    tune_threads(&mut threads);
    let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
    let (root, cx) = cx.add_window_view({
        let list = list.clone();
        move |window, cx| Rooted(cx.new(|cx| JobView::new(list, Some(threads), window, cx)))
    });
    let view = root.read_with(cx, |root, _| root.0.clone());
    cx.simulate_resize(size);
    list.update(cx, |list, cx| list.open(job.into(), cx));
    redraw(cx);
    (view, cx)
}

/// Every turn of the loop the window needs to settle: parked work, then a frame, three times.
#[cfg(test)]
pub(super) fn redraw(cx: &mut gpui::VisualTestContext) {
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
}

/// Whether the window drew an element with this role and name in its last frame.
#[cfg(test)]
pub(super) fn drawn(cx: &mut gpui::VisualTestContext, role: Role, name: &str) -> bool {
    cx.update(|window, cx| cx.try_global::<crate::labels::LabelBook>().is_some_and(|book| book.find(window.window_handle().window_id(), role, name).is_some()))
}

/// The names of what the window drew with this role in its last frame that start with `prefix`.
#[cfg(test)]
pub(super) fn drawn_like(cx: &mut gpui::VisualTestContext, role: Role, prefix: &str) -> Vec<String> {
    cx.update(|window, cx| {
        cx.try_global::<crate::labels::LabelBook>()
            .map(|book| book.drawn(window.window_handle().window_id()).iter().filter(|one| one.role == role && one.name.starts_with(prefix)).map(|one| one.name.to_string()).collect())
            .unwrap_or_default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(at: &str) -> ThreadItem {
        let wire = format!(r#"{{"kind":"row","row":{{"kind":"message","id":"m-{at}","at":"{at}","from":{{"kind":"you"}},"text":"x"}}}}"#);
        ipc::decode::<ThreadItem>("item", wire.as_bytes()).expect("a row decodes")
    }

    fn card(artifact: &str, revision: u32, at: &str) -> (String, u32, Instant) {
        (artifact.into(), revision, Instant::carried(at))
    }

    #[test]
    fn a_card_stands_under_the_last_item_that_began_by_its_time_where_later_items_stand() {
        let items = [row("2026-11-14T10:00:00Z"), row("2026-11-14T10:10:00Z"), row("2026-11-14T10:30:00Z")];
        let (anchors, tail) = place(&items, &[card("a", 1, "2026-11-14T10:05:00Z"), card("b", 1, "2026-11-14T10:20:00Z")]);
        assert_eq!(anchors.iter().map(|one| (one.item, one.artifact.as_str())).collect::<Vec<_>>(), [(0, "a"), (1, "b")]);
        assert!(tail.is_empty());
    }

    #[test]
    fn a_card_with_nothing_after_it_stands_at_the_foot_with_the_extras() {
        let items = [row("2026-11-14T10:00:00Z"), row("2026-11-14T10:10:00Z")];
        let (anchors, tail) = place(&items, &[card("a", 1, "2026-11-14T10:10:00Z"), card("a", 2, "2026-11-14T10:40:00Z")]);
        assert!(anchors.is_empty());
        assert_eq!(tail, [("a".to_string(), 1), ("a".to_string(), 2)]);
    }

    #[test]
    fn revisions_keep_the_order_they_were_kept_in_and_a_thread_with_no_items_holds_them_all_at_the_foot() {
        let items = [row("2026-11-14T10:00:00Z"), row("2026-11-14T10:50:00Z")];
        let (anchors, _) = place(&items, &[card("a", 2, "2026-11-14T10:30:00Z"), card("a", 1, "2026-11-14T10:20:00Z")]);
        assert_eq!(anchors.iter().map(|one| one.revision).collect::<Vec<_>>(), [1, 2]);
        let (anchors, tail) = place(&[], &[card("a", 1, "2026-11-14T10:20:00Z")]);
        assert!(anchors.is_empty() && tail.len() == 1);
    }

    #[test]
    fn a_card_older_than_every_item_stands_under_the_first() {
        let items = [row("2026-11-14T10:30:00Z"), row("2026-11-14T10:50:00Z")];
        let (anchors, _) = place(&items, &[card("a", 1, "2026-11-14T09:00:00Z")]);
        assert_eq!(anchors[0].item, 0);
    }

    #[test]
    fn the_list_measures_again_from_the_first_item_whose_cards_changed() {
        let a = |item| Anchor { item, artifact: "a".into(), revision: 1 };
        assert_eq!(first_difference(&[a(2), a(5)], &[a(2), a(5)]), None);
        assert_eq!(first_difference(&[a(2)], &[a(2), a(5)]), Some(5));
        assert_eq!(first_difference(&[a(4)], &[a(2)]), Some(2));
    }

    #[test]
    fn a_verdict_is_kept_and_read_back_with_its_note() {
        let sent = Verdict { stand: Stand::SentBack, word: "Sent back".into(), note: Some("Shorter, and name the PR.".into()) };
        assert_eq!(Verdict::read(&sent.kept()), Some(sent));
        let picked = Verdict { stand: Stand::Approved, word: "Picked B Rail".into(), note: None };
        assert_eq!(Verdict::read(&picked.kept()), Some(picked));
        assert_eq!(Verdict::read("nonsense"), None);
    }

    #[test]
    fn a_note_is_cut_to_one_line_with_an_ellipsis_where_it_goes_on() {
        assert_eq!(note_start("Shorter,\nand name the PR."), "Shorter, and name the PR.");
        let long = "word ".repeat(40);
        assert!(note_start(&long).ends_with('\u{2026}') && note_start(&long).chars().count() <= NOTE_START + 1);
    }

    use super::super::row_tabs::RowKind;
    use super::super::Extra;
    use crate::mock_parts::JOB;

    #[gpui::test]
    fn each_revision_stands_where_it_was_kept_and_the_older_one_above_the_newer(cx: &mut gpui::TestAppContext) {
        let (view, cx) = folds_job(cx, |_| {});
        view.read_with(cx, |view, _| {
            let page = view.pages.get(JOB).expect("the Job's page is held");
            let at = |artifact: &str, revision: u32| {
                page.anchors
                    .iter()
                    .find(|one| one.artifact == artifact && one.revision == revision)
                    .map(|one| one.item)
                    .or_else(|| page.extras.iter().position(|extra| matches!(extra, Extra::Parts(id, number) if id == artifact && *number == revision)).map(|at| page.items + at))
            };
            let (first, second) = (at("draft:Merge line wording", 1).expect("revision 1 is placed"), at("draft:Merge line wording", 2).expect("revision 2 is placed"));
            assert!(first < second, "revision 1 stands above revision 2");
            assert!(page.anchors.iter().any(|one| one.artifact == "draft:Merge line wording" && one.revision == 1), "revision 1 stands under an item of the thread, not at its foot");
        });
        assert!(drawn(cx, Role::Article, "Artifact: Merge line wording, Revision 1"));
        assert!(drawn(cx, Role::Article, "Artifact: Merge line wording, Revision 2"), "the asking revision is a card answered in the thread");
    }

    #[gpui::test]
    fn an_older_revision_with_no_verdict_kept_reads_replaced_and_one_he_sent_back_reads_it_with_his_note(cx: &mut gpui::TestAppContext) {
        let (view, cx) = folds_job(cx, |_| {});
        assert_eq!(drawn_like(cx, Role::Status, "Merge line wording: ").first().map(String::as_str), Some("Merge line wording: Sent back"));
        view.read_with(cx, |view, cx| {
            let verdict = view.verdict_kept(JOB, "draft:Merge line wording", 1, cx).expect("the mock kept what he said of revision 1");
            assert_eq!(verdict.word, "Sent back");
            assert!(verdict.note.as_deref().is_some_and(|note| note.starts_with("Say which branch")));
        });
    }

    #[gpui::test]
    fn a_sketch_redrawn_under_a_new_title_draws_its_artifact_card_and_no_pinned_card_beside_it(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let jobs = crate::mock::jobs("artifact-folds").expect("the fixture decodes");
        let mut threads = crate::mock::threads("artifact-folds").expect("the threads decode");
        // Fleet pins the newest card of the sketch: its ledger id is the first title's, its title the new one.
        let wire = r#"{"id":"sk-2","at":"2026-11-14T10:46:00Z","sketch":"sketch:Step flow","title":"Step flow, with the gate","drawing":{"nodes":[],"edges":[]},"asked":true}"#;
        threads.get_mut(JOB).expect("the Job's thread").head.sketch = Some(ipc::decode::<ipc::ThreadSketch>("a pin", wire.as_bytes()).expect("a pin decodes"));
        let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
        let (root, cx) = cx.add_window_view({
            let list = list.clone();
            move |window, cx| Rooted(cx.new(|cx| JobView::new(list, Some(threads), window, cx)))
        });
        let view = root.read_with(cx, |root, _| root.0.clone());
        cx.simulate_resize(gpui::size(gpui::px(1400.), gpui::px(6000.)));
        list.update(cx, |list, cx| list.open(JOB.into(), cx));
        redraw(cx);
        view.read_with(cx, |view, _| {
            let page = view.pages.get(JOB).expect("the Job's page is held");
            assert!(!page.extras.iter().any(|extra| matches!(extra, Extra::Sketch(_))), "no pinned card: the artifact's card draws the sketch");
            let placed = page.anchors.iter().any(|one| one.artifact == "sketch:Step flow") || page.extras.iter().any(|extra| matches!(extra, Extra::Parts(id, _) if id == "sketch:Step flow"));
            assert!(placed, "the artifact's card stands");
        });
    }

    #[gpui::test]
    fn open_reads_a_parts_artifact_whole_in_the_review_panel_one_with_no_drawing_too(cx: &mut gpui::TestAppContext) {
        let (view, cx) = folds_job(cx, |_| {});
        for id in ["sketch:Rollout order", "sketch:Step flow", "draft:Merge line wording"] {
            let opened = view.update_in(cx, |view, window, cx| view.open_named(id, window, cx));
            assert!(opened, "{id} is held");
            redraw(cx);
            view.read_with(cx, |view, _| {
                assert!(view.review_open(), "{id} opens the review panel");
                assert_eq!(view.review_chosen().and_then(|chosen| chosen.artifact.clone()).as_deref(), Some(id), "{id} is the one chosen");
                assert!(view.row_tab_of(JOB, RowKind::Artifact, id).is_none(), "{id} opens no tab of its own");
            });
        }
        assert!(drawn(cx, Role::Region, "Reader: Merge line wording, Revision 2"), "the reader holds the newest revision whole");
    }

    #[gpui::test]
    fn open_reads_a_page_in_the_review_panel_a_draft_in_its_own_tab_and_a_sketch_large(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let jobs = crate::mock::jobs("reader").expect("the fixture decodes");
        let threads = crate::mock::threads("reader").expect("the threads decode");
        let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
        let (view, cx) = cx.add_window_view({
            let list = list.clone();
            move |window, cx| JobView::new(list, Some(threads), window, cx)
        });
        list.update(cx, |list, cx| list.open("draft-job".into(), cx));
        redraw(cx);
        let held: Vec<(String, ArtifactForm, bool)> = view.read_with(cx, |view, _| view.artifacts_of("draft-job").iter().map(|one| (one.id.clone(), one.form, one.parts.is_empty())).collect());
        for form in [ArtifactForm::Page, ArtifactForm::Draft] {
            let (id, _, _) = held.iter().find(|(_, held, bare)| *held == form && *bare).cloned().unwrap_or_else(|| panic!("the Job holds a {form:?} with no parts"));
            assert!(view.update_in(cx, |view, window, cx| view.open_named(&id, window, cx)));
            redraw(cx);
            if form == ArtifactForm::Page {
                view.read_with(cx, |view, _| {
                    assert!(view.review_open(), "a page opens the review panel");
                    assert!(!view.row_tab_list("draft-job").iter().any(|tab| tab.kind == RowKind::Document), "and no tab of its own");
                });
                view.update_in(cx, |view, window, cx| view.close_review(window, cx));
            } else {
                view.read_with(cx, |view, _| assert!(view.row_tab_list("draft-job").iter().any(|tab| tab.kind == RowKind::Draft), "a {form:?} opens in its own tab"));
            }
        }
    }
}
