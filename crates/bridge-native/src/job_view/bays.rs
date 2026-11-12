//! A Job's Bays tab (`BaysLayer`): its write clashes, then each bay it holds as the cockpit's Berth
//! draws it, a press opening the bay's panel (`BayPanel`) with its acts, their confirms, the Scout's
//! Finding and the bay's history.
//!
//! **The acts act on his real slots**, so each carries Classic's confirm exactly where Classic has one
//! (`PoolSlots/ReleaseConfirm.tsx`, `SlotFinding.tsx`'s `ScrapConfirm`) and none where it has none.
//! The pool is read from `GET /worktrees` as the tab shows and again after each act; the clashes are
//! the ones `write_clashes.changed` last carried.

use std::cell::Cell;
use std::collections::HashSet;

use chrono::{DateTime, Utc};
use gpui::{div, prelude::*, px, AnyElement, Context, FontWeight, Role, ScrollHandle, SharedString, Window};
use ipc::{
    BayHistory, BayHistoryEntry, ChangeSlotPool, ClashFile, ClashHolder, ClashSide, CommitHome, ManifestId, RescueAct, RescueSlot, SlotFindingState, SlotHolding, SlotLastHolder, SlotPoolChanged,
    SlotRescued, SlotVerdict, WorktreeSlot, WorktreesHeld,
};

use super::pr_panel::{small_button, stand_ins};
use super::rows::{glyph, mono, tip, Size};
use super::JobView;
use crate::diff;
use crate::labels::labelled;
use crate::look::{token, Look, Token};

/// Behind its base by this many commits, a bay is wrong (`BEHIND_FROM`).
const BEHIND_FROM: u32 = 20;
/// A clash's files fold past this many, to the first three (`ClashPaths`).
const SHOWN: usize = 3;

#[derive(Default)]
pub(super) struct Bays {
    /// The pool as Fleet last answered.
    pool: Option<Vec<WorktreeSlot>>,
    /// Clashes whose files are unfolded, by the other holder's id.
    unfolded: HashSet<String>,
}

/// What a held bay is wrong with, where it is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Wrong {
    Stranded,
    Behind,
}

/// A bay the Job holds, or held last before it stranded.
#[derive(Clone)]
pub(super) struct HeldBay {
    slot: WorktreeSlot,
    wrong: Option<Wrong>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BayState {
    Stranded,
    Leased,
    Closed,
    Busy,
    Unmade,
    NotACheckout,
    Free,
}

/// A bay as the deck draws it (`bayViewOf`, live).
struct BayView {
    state: BayState,
    word: &'static str,
    said: Option<String>,
    /// The holder's id and title.
    holder: Option<(String, Option<String>)>,
    branch: Option<String>,
    last: Option<SlotLastHolder>,
}

fn view_of(slot: &WorktreeSlot) -> BayView {
    let base = |state, word, said: Option<String>| BayView { state, word, said, holder: None, branch: None, last: None };
    match &slot.held {
        SlotHolding::Stranded { why } => BayView {
            branch: slot.branch.clone(),
            last: slot.stranded.as_ref().and_then(|stranded| stranded.last_holder.clone()),
            ..base(BayState::Stranded, "Stranded", Some(format!("Stranded: {why}")))
        },
        SlotHolding::Session { holder, session_id, session_title } => BayView {
            holder: Some((session_id.clone().unwrap_or_else(|| holder.clone()), session_title.clone())),
            branch: slot.branch.clone(),
            ..base(BayState::Leased, "Held", None)
        },
        SlotHolding::HostedSession { session_id, session_title } => BayView {
            holder: Some((session_id.clone(), session_title.clone())),
            branch: slot.branch.clone(),
            ..base(BayState::Leased, "Held", None)
        },
        _ if slot.closed => base(BayState::Closed, "Closed", None),
        SlotHolding::Busy => base(BayState::Busy, "Busy", Some("Being taken or given back".into())),
        SlotHolding::Unmade => base(BayState::Unmade, "Not made yet", None),
        SlotHolding::NotACheckout => base(BayState::NotACheckout, "Not a checkout", None),
        SlotHolding::Free => base(BayState::Free, "Free", None),
    }
}

/// The state's hue (`--slot-*`).
fn hue_of(state: BayState) -> Token {
    match state {
        BayState::Leased | BayState::Busy => token::SLOT_HELD,
        BayState::Free => token::SLOT_FREE,
        BayState::Stranded => token::SLOT_STRANDED,
        BayState::Closed => token::SLOT_CLOSED,
        BayState::Unmade | BayState::NotACheckout => token::FG_SUBTLE,
    }
}

/// The panel's badge: its hue and glyph (`BAY`).
fn badge_of(state: BayState) -> (Token, &'static str) {
    match state {
        BayState::Leased | BayState::Busy => (token::STATUS_RUNNING, "key-round"),
        BayState::Stranded => (token::STATUS_AWAITING_REVIEW, "ghost"),
        BayState::Closed => (token::STATUS_NOT_STARTED, "door-closed-locked"),
        _ => (token::STATUS_COMPLETED_SUCCESS, "door-open"),
    }
}

/// An id cut to what a person tells it by (`shortOf`).
fn short_of(id: &str) -> String {
    let chars: Vec<char> = id.chars().collect();
    if chars.len() > 10 {
        format!("{}\u{2026}{}", chars[..4].iter().collect::<String>(), chars[chars.len() - 4..].iter().collect::<String>())
    } else {
        id.to_string()
    }
}

/// `48m`, `2h 05m` (`spanOf`).
fn span_of(ms: i64) -> String {
    let minutes = ((ms as f64) / 60_000.).round().max(0.) as i64;
    if minutes < 60 {
        format!("{minutes}m")
    } else {
        format!("{}h {:02}m", minutes / 60, minutes % 60)
    }
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn short_sha(sha: &str) -> &str {
    &sha[..sha.len().min(7)]
}

/// The slot numbers a Job names: its slot attachments carry the number (`namedSlots`). A branch
/// takes its slot from those, so it names none of its own.
fn named_slots(record: &ipc::SessionRecord) -> HashSet<u32> {
    record.attachments.iter().filter(|one| one.kind == "slot").filter_map(|one| one.target.trim().parse().ok()).filter(|slot| *slot != 0).collect()
}

/// The pool's slots a Job holds, or held last before they stranded, in slot order (`baysOf`).
fn bays_of(record: &ipc::SessionRecord, slots: &[WorktreeSlot]) -> Vec<HeldBay> {
    let named = named_slots(record);
    let mine = |id: &str| id == record.id.as_str();
    let mut bays: Vec<HeldBay> = slots
        .iter()
        .filter(|slot| record.manifest_id.as_ref().is_none_or(|manifest| *manifest == slot.manifest_id))
        .filter(|slot| {
            named.contains(&slot.slot)
                || match &slot.held {
                    SlotHolding::Session { session_id: Some(id), .. } | SlotHolding::HostedSession { session_id: id, .. } => mine(id),
                    SlotHolding::Stranded { .. } => slot.stranded.as_ref().and_then(|stranded| stranded.last_holder.as_ref()).is_some_and(|last| last.kind == ipc::SlotHolderKind::Session && mine(&last.id)),
                    _ => false,
                }
        })
        .map(|slot| {
            let wrong = if matches!(slot.held, SlotHolding::Stranded { .. }) {
                Some(Wrong::Stranded)
            } else if slot.behind.is_some_and(|behind| behind >= BEHIND_FROM) {
                Some(Wrong::Behind)
            } else {
                None
            };
            HeldBay { slot: slot.clone(), wrong }
        })
        .collect();
    bays.sort_by_key(|bay| bay.slot.slot);
    bays
}

/// A Job's write clashes, each as the other holder and the files.
fn clashes_of(all: &ipc::AllWriteClashes, id: &str) -> Vec<(ClashHolder, Vec<String>)> {
    all.repositories
        .iter()
        .flat_map(|repository| repository.clashes.iter())
        .filter_map(|clash| {
            let mine = |holder: &ClashHolder| holder.kind == ClashSide::Session && holder.id == id;
            if mine(&clash.one) {
                Some((clash.other.clone(), clash.paths.clone()))
            } else if mine(&clash.other) {
                Some((clash.one.clone(), clash.paths.clone()))
            } else {
                None
            }
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum BayTab {
    Facts,
    History,
}

/// A confirm open on the panel.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Confirm {
    Release,
    Scrap,
}

/// One lease of the bay's history: how long, by whom, on what branch.
#[derive(Clone)]
struct HistoryRow {
    span: String,
    who: String,
    branch: Option<String>,
}

/// A bay's panel: which bay, its tab, its history, the confirm open, and what the last act said.
pub(super) struct BayPanel {
    pub manifest: ManifestId,
    pub slot: u32,
    pub tab: BayTab,
    history: Option<Vec<HistoryRow>>,
    confirm: Option<Confirm>,
    acting: bool,
    said: Option<String>,
    refused: Option<String>,
    /// The panel's scroller, and whether the next draw brings a just-opened confirm, or what an act
    /// answered, into view.
    scroll: ScrollHandle,
    reveal: Cell<bool>,
}

/// A clash's file as each side changed it.
pub(super) struct ClashFilePanel {
    pub with: ClashHolder,
    pub path: String,
    pub read: Option<Result<ClashFile, String>>,
}

/// Leases from Fleet's history, newest first: the release that ends one is the nearest above it.
fn history_rows(history: &BayHistory, now: DateTime<Utc>) -> Vec<HistoryRow> {
    let at = |instant: &ipc::Instant| DateTime::parse_from_rfc3339(instant.as_str()).ok().map(|at| at.with_timezone(&Utc));
    history
        .entries
        .iter()
        .enumerate()
        .filter_map(|(index, one)| {
            let BayHistoryEntry::Leased { at: leased, holder, branch } = one else { return None };
            let freed = history.entries[..index].iter().rev().find_map(|later| match later {
                BayHistoryEntry::Released { at, holder: freed } if freed.id == holder.id => Some(at),
                _ => None,
            });
            let end = freed.and_then(at).unwrap_or(now);
            let span = at(leased).map(|start| span_of((end - start).num_milliseconds())).unwrap_or_default();
            Some(HistoryRow { span, who: holder.title.clone().unwrap_or_else(|| short_of(&holder.id)), branch: branch.clone() })
        })
        .collect()
}

/// The slot's commits on no remote and not on its base, as `sha7 subject`.
fn only_here(slot: &WorktreeSlot) -> Vec<String> {
    slot.stranded
        .as_ref()
        .map(|stranded| stranded.commits.iter().filter(|one| one.home == CommitHome::OnlyHere).map(|one| format!("{} {}", short_sha(&one.sha), one.subject)).collect())
        .unwrap_or_default()
}

/// `nvim (pid 44698)` as the command and its process id.
fn holder_parts(holder: &str) -> (String, Option<String>) {
    if let Some(open) = holder.rfind(" (pid ") {
        if let Some(pid) = holder[open + 6..].strip_suffix(')') {
            if !pid.is_empty() && pid.chars().all(|c| c.is_ascii_digit()) {
                return (holder[..open].to_string(), Some(pid.to_string()));
            }
        }
    }
    (holder.to_string(), None)
}

impl JobView {
    /// `GET /worktrees`: the pool, read as a tab that draws bays shows, and after each act.
    pub(super) fn read_pool(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.fleet.is_none() {
            if self.panes.bays.pool.is_none() {
                self.panes.bays.pool = crate::mock::pool();
                if let Some(clashes) = crate::mock::bay_clashes() {
                    crate::clash_mark::hold(clashes, cx);
                }
            }
            return;
        }
        self.get(
            "/worktrees".into(),
            window,
            cx,
            Box::new(|this, answer, _, cx| {
                if let Some(held) = answer.ok().and_then(|bytes| ipc::decode::<WorktreesHeld>("WorktreesHeld", &bytes).ok()) {
                    this.panes.bays.pool = Some(held.slots);
                    cx.notify();
                }
            }),
        );
    }

    /// The bays the Job holds, from the pool last read.
    pub(super) fn held_bays(&self, id: &str, cx: &gpui::App) -> Vec<HeldBay> {
        let (Some(record), Some(pool)) = (self.record(id, cx), self.panes.bays.pool.as_ref()) else { return Vec::new() };
        bays_of(record, pool)
    }

    /// Bays: the write clashes, then a Berth for each bay. Neither, and nothing is drawn.
    pub(super) fn pane_bays(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let clashes = crate::clash_mark::held(cx).map(|all| clashes_of(all, id)).unwrap_or_default();
        let bays = self.held_bays(id, cx);
        if clashes.is_empty() && bays.is_empty() {
            return None;
        }
        let table = (!clashes.is_empty()).then(|| {
            let head = |said: &'static str, wide: bool| {
                div()
                    .when(wide, |cell| cell.flex_1())
                    .when(!wide, |cell| cell.flex_none().w(look.length(token::SPACE_12) * 3.))
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_2XS))
                    .text_color(look.color(token::FG_SUBTLE))
                    .child(said)
            };
            let rows: Vec<AnyElement> = clashes.into_iter().map(|(with, paths)| self.clash_row(with, paths, look, cx)).collect();
            super::system::section(
                look,
                "Write clashes",
                None,
                None,
                labelled(
                    div()
                        .id("clash-table")
                        .flex()
                        .flex_col()
                        .rounded(look.length(token::RADIUS_MD))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_SUBTLE))
                        .child(
                            div()
                                .flex()
                                .gap(look.length(token::SPACE_3))
                                .px(look.length(token::SPACE_2))
                                .py(look.length(token::SPACE_1))
                                .border_b(look.length(token::BORDER_WIDTH))
                                .border_color(look.color(token::BORDER_SUBTLE))
                                .child(head("Clashes with", false))
                                .child(head("Files", true)),
                        )
                        .children(rows),
                    Role::Table,
                    "Write clashes",
                )
                .into_any_element(),
            )
        });
        let berths: Vec<AnyElement> = bays.iter().map(|bay| self.berth_row(bay, "bays", look, cx)).collect();
        Some(
            div()
                .id("pane-bays")
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_4))
                .py(look.length(token::SPACE_2))
                .children(table)
                .child(div().flex().flex_col().gap(look.length(token::SPACE_2)).children(berths))
                .into_any_element(),
        )
    }

    /// One clash: the other holder, a link unless it is main, and its files, folded past four.
    fn clash_row(&self, with: ClashHolder, paths: Vec<String>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let holder: AnyElement = if with.kind == ClashSide::Main {
            div().child(SharedString::from(with.label.clone())).into_any_element()
        } else {
            let pressed = with.clone();
            labelled(
                div()
                    .id(SharedString::from(format!("clash-with:{}", with.id)))
                    .truncate()
                    .text_color(look.color(token::ACCENT))
                    .cursor_pointer()
                    .hover(|style| style.underline())
                    .child(SharedString::from(with.label.clone()))
                    .on_click(cx.listener(move |_, _, window, cx| {
                        if let Some(open) = cx.try_global::<crate::clash_mark::OpenHolder>().cloned() {
                            (open.0)(&pressed, window, cx);
                        }
                    })),
                Role::Link,
                format!("Open {}", with.label),
            )
            .into_any_element()
        };
        let unfolded = self.panes.bays.unfolded.contains(&with.id);
        let folds = paths.len() > SHOWN + 1;
        let shown: Vec<String> = if folds && !unfolded { paths.iter().take(SHOWN).cloned().collect() } else { paths.clone() };
        let chips: Vec<AnyElement> = shown
            .into_iter()
            .map(|path| {
                let (dir, name) = match path.rfind('/') {
                    Some(at) => (path[..=at].to_string(), path[at + 1..].to_string()),
                    None => (String::new(), path.clone()),
                };
                let (pressed_with, pressed_path) = (with.clone(), path.clone());
                let ground = look.color(token::BG_HOVER);
                let chip = div()
                    .id(SharedString::from(format!("clash-path:{}:{path}", with.id)))
                    .flex()
                    .max_w_full()
                    .px(look.length(token::SPACE_1))
                    .rounded(look.length(token::RADIUS_SM))
                    .bg(look.color(token::BG_SUNKEN))
                    .cursor_pointer()
                    .hover(move |style| style.bg(ground))
                    .tooltip(tip(path.clone()))
                    .child(div().text_color(look.color(token::FG_SUBTLE)).truncate().child(SharedString::from(dir)))
                    .child(div().flex_none().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(name)))
                    .on_click(cx.listener(move |this, _, window, cx| this.open_clash_file(pressed_with.clone(), pressed_path.clone(), window, cx)));
                labelled(chip, Role::Button, format!("Open {path}")).into_any_element()
            })
            .collect();
        let toggle = folds.then(|| {
            let key = with.id.clone();
            let said = if unfolded { "Fewer".to_string() } else { format!("{} more", paths.len() - SHOWN) };
            let strong = look.color(token::FG_DEFAULT);
            labelled(
                div()
                    .id(SharedString::from(format!("clash-more:{}", with.id)))
                    .px(look.length(token::SPACE_1))
                    .rounded(look.length(token::RADIUS_SM))
                    .text_color(look.color(token::FG_MUTED))
                    .cursor_pointer()
                    .hover(move |style| style.text_color(strong))
                    .child(SharedString::from(said.clone()))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.panes.bays.unfolded.remove(&key) {
                            this.panes.bays.unfolded.insert(key.clone());
                        }
                        cx.notify();
                    })),
                Role::Button,
                said,
            )
            .into_any_element()
        });
        div()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_1))
            .text_size(look.length(token::TEXT_XS))
            .child(div().flex_none().w(look.length(token::SPACE_12) * 3.).min_w_0().child(holder))
            .child(labelled(
                div()
                    .id(SharedString::from(format!("clash-files:{}", with.id)))
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_wrap()
                    .gap(look.length(token::SPACE_1))
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_2XS))
                    .children(chips)
                    .children(toggle),
                Role::List,
                format!("Conflicting files with {}", with.label),
            ))
            .into_any_element()
    }

    /// One bay as the Berth draws it: a door in the state's hue with its number, then the holder,
    /// the branch and the foot. A press opens its panel.
    pub(super) fn berth_row(&self, bay: &HeldBay, place: &'static str, look: &Look, cx: &Context<Self>) -> AnyElement {
        let slot = &bay.slot;
        let view = view_of(slot);
        let hue = look.color(hue_of(view.state));
        let name = format!("slot-{}", slot.slot);
        let free = view.state == BayState::Free;
        let edge = look.color(token::BORDER_SUBTLE).blend(hue.opacity(0.55));
        let (ground, border) = match view.state {
            BayState::Free => (look.color(token::BG_SUNKEN), edge),
            BayState::Stranded => (look.color(token::BG_RAISED).blend(look.color(token::SLOT_STRANDED_BG)), hue),
            BayState::Closed => (look.color(token::BG_SUNKEN).blend(look.color(token::SLOT_CLOSED_BG)), look.color(token::BORDER_SUBTLE).blend(hue.opacity(0.25))),
            _ => (look.color(token::BG_RAISED), edge),
        };
        let lamp = div()
            .absolute()
            .top(look.length(token::SPACE_2))
            .left(look.length(token::SPACE_2))
            .size(px(6.))
            .rounded_full()
            .when(free, |lamp| lamp.border(look.length(token::BORDER_WIDTH)).border_color(hue))
            .when(!free, |lamp| lamp.bg(hue));
        let door = div()
            .id(SharedString::from(format!("{place}-door:{}", slot.slot)))
            .relative()
            .flex_none()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .w(look.length(token::SPACE_12) + look.length(token::SPACE_2))
            .border_r(look.length(token::BORDER_WIDTH) * if free { 2. } else { 1. })
            .border_color(if free { edge } else { look.color(token::BORDER_SUBTLE) })
            .when(free, |door| door.border_dashed())
            .tooltip(tip(view.said.clone().unwrap_or_else(|| view.word.to_string())))
            .child(lamp)
            .child(div().font_family(mono(look)).text_size(look.length(token::TEXT_XL)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(hue).child(SharedString::from(slot.slot.to_string())));
        let lead = match (view.state, &view.holder, &view.last) {
            (BayState::Leased, Some((id, title)), _) => Some((title.clone().unwrap_or_else(|| short_of(id)), title.clone().unwrap_or_else(|| id.clone()), false)),
            (BayState::Stranded, _, Some(last)) => Some((last.title.clone().unwrap_or_else(|| short_of(&last.id)), last.title.clone().unwrap_or_else(|| last.id.clone()), false)),
            (BayState::Stranded, _, None) => Some(("holder gone".to_string(), "holder gone".to_string(), true)),
            _ => None,
        };
        let lead = lead.map(|(shown, whole, gone)| {
            div()
                .id(SharedString::from(format!("{place}-who:{}", slot.slot)))
                .truncate()
                .text_color(look.color(if gone { token::FG_MUTED } else { token::FG_DEFAULT }))
                .tooltip(tip(whole))
                .child(SharedString::from(shown))
        });
        let branch = view.branch.clone().filter(|_| view.state == BayState::Leased).map(|branch| {
            div()
                .id(SharedString::from(format!("{place}-branch:{}", slot.slot)))
                .truncate()
                .font_family(mono(look))
                .text_size(look.length(token::TEXT_2XS))
                .text_color(look.color(token::FG_MUTED))
                .tooltip(tip(branch.clone()))
                .child(SharedString::from(branch))
        });
        let mark = |key: String, icon: &'static str, said: String, ink: Token| -> AnyElement {
            labelled(div().id(SharedString::from(key)).flex().items_center().tooltip(tip(said.clone())).child(glyph(look, icon, Size::Small, look.color(ink))), Role::Image, said).into_any_element()
        };
        let mut foot: Vec<AnyElement> = Vec::new();
        if matches!(view.state, BayState::Free | BayState::Closed) {
            foot.push(if slot.warm {
                mark(format!("{place}-warm:{}", slot.slot), "flame", "Warm".into(), token::SLOT_WARM)
            } else {
                mark(format!("{place}-cold:{}", slot.slot), "snowflake", "Cold".into(), token::SLOT_COLD)
            });
        }
        if let Some(stranded) = slot.stranded.as_ref().filter(|_| view.state == BayState::Stranded) {
            if !stranded.uncommitted.is_empty() {
                foot.push(mark(format!("{place}-dirty:{}", slot.slot), "file-pen-line", format!("{} uncommitted", plural(stranded.uncommitted.len(), "file", "files")), token::FG_SUBTLE));
            }
            if stranded.unpushed > 0 {
                foot.push(mark(format!("{place}-unpushed:{}", slot.slot), "arrow-up-from-line", format!("{} not pushed", plural(stranded.unpushed as usize, "commit", "commits")), token::FG_SUBTLE));
            }
            if let Some(told) = stranded.told.as_ref() {
                let at = DateTime::parse_from_rfc3339(told.as_str()).map(|at| at.with_timezone(&chrono::Local).format("%H:%M").to_string()).unwrap_or_default();
                foot.push(mark(format!("{place}-told:{}", slot.slot), "mail", format!("Holder told at {at}"), token::FG_SUBTLE));
            }
            if stranded.last_holder.as_ref().is_some_and(|last| !last.open) {
                foot.push(mark(format!("{place}-gone:{}", slot.slot), "unplug", "Job closed".into(), token::FG_SUBTLE));
            }
        }
        if let Some(behind) = slot.behind.filter(|behind| *behind > 0) {
            let ink = look.color(if bay.wrong == Some(Wrong::Behind) { token::STATUS_AWAITING_REVIEW } else { token::FG_MUTED });
            foot.push(
                labelled(
                    div()
                        .id(SharedString::from(format!("{place}-behind:{}", slot.slot)))
                        .flex()
                        .items_center()
                        .gap(px(2.))
                        .text_color(ink)
                        .tooltip(tip(format!("{} behind {}", plural(behind as usize, "commit", "commits"), slot.base)))
                        .child(glyph(look, "arrow-down", Size::Small, ink))
                        .child(SharedString::from(behind.to_string())),
                    Role::Image,
                    format!("{behind} behind {}", slot.base),
                )
                .into_any_element(),
            );
        }
        let body = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .justify_center()
            .gap(px(2.))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .children(lead)
            .children(branch)
            .when(!foot.is_empty(), |body| body.child(div().flex().items_center().gap(look.length(token::SPACE_2)).font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).children(foot)));
        let (manifest, number) = (slot.manifest_id.clone(), slot.slot);
        let hover_ground = look.color(token::BG_HOVER);
        labelled(
            div()
                .id(SharedString::from(format!("{place}-bay:{}", slot.slot)))
                .flex()
                .w_full()
                .min_h(look.length(token::H_ROW_UNIT) * 2.)
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(border)
                .when(free, |bay| bay.border_dashed())
                .bg(ground)
                .overflow_hidden()
                .text_size(look.length(token::TEXT_XS))
                .cursor_pointer()
                .hover(move |style| style.border_color(hue).bg(hover_ground))
                .tooltip(tip("Show in Bays"))
                .child(door)
                .child(body)
                .on_click(cx.listener(move |this, _, window, cx| this.open_bay(manifest.clone(), number, window, cx))),
            Role::Button,
            format!("Show {name} in Bays"),
        )
        .into_any_element()
    }

    fn open_bay(&mut self, manifest: ManifestId, slot: u32, window: &mut Window, cx: &mut Context<Self>) {
        let panel = BayPanel { manifest: manifest.clone(), slot, tab: BayTab::Facts, history: None, confirm: None, acting: false, said: None, refused: None, scroll: ScrollHandle::new(), reveal: Cell::new(false) };
        self.open_bay_panel(panel, window, cx);
        self.read_bay_history(manifest, slot, window, cx);
    }

    /// `GET /slot_history`: who held the bay, newest first. A refusal reads as none.
    fn read_bay_history(&mut self, manifest: ManifestId, slot: u32, window: &mut Window, cx: &mut Context<Self>) {
        let take = move |this: &mut JobView, history: Option<BayHistory>, cx: &mut Context<JobView>| {
            let rows = history.map(|history| history_rows(&history, Utc::now())).unwrap_or_default();
            if let Some(panel) = this.bay_panel_mut().filter(|panel| panel.slot == slot) {
                panel.history = Some(rows);
            }
            cx.notify();
        };
        if self.fleet.is_none() {
            take(self, crate::mock::bay_history(manifest.as_str(), slot), cx);
            return;
        }
        let path = format!("/slot_history?manifest_id={}&slot={slot}", super::wire::encoded(manifest.as_str()));
        self.get(path, window, cx, Box::new(move |this, answer, _, cx| take(this, answer.ok().and_then(|bytes| ipc::decode::<BayHistory>("BayHistory", &bytes).ok()), cx)));
    }

    fn bay_tab(&mut self, tab: BayTab, cx: &mut Context<Self>) {
        if let Some(panel) = self.bay_panel_mut() {
            panel.tab = tab;
        }
        cx.notify();
    }

    fn bay_confirm(&mut self, confirm: Option<Confirm>, cx: &mut Context<Self>) {
        if let Some(panel) = self.bay_panel_mut() {
            panel.confirm = confirm;
            panel.reveal.set(confirm.is_some());
        }
        cx.notify();
    }

    /// Closes any confirm, clears what the last act said and marks one out; the bay it acts on.
    fn begin_act(&mut self) -> Option<(ManifestId, u32)> {
        let panel = self.bay_panel_mut()?;
        panel.confirm = None;
        panel.acting = true;
        panel.said = None;
        panel.refused = None;
        Some((panel.manifest.clone(), panel.slot))
    }

    /// An act's answer on the panel: a receipt, or Fleet's refusal under its lead; then the pool again.
    fn end_act(&mut self, slot: u32, said: Result<Option<String>, String>, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(panel) = self.bay_panel_mut().filter(|panel| panel.slot == slot) {
            panel.acting = false;
            match said {
                Ok(said) => panel.said = said,
                Err(why) => panel.refused = Some(why),
            }
            panel.reveal.set(panel.said.is_some() || panel.refused.is_some());
        }
        self.read_pool(window, cx);
        cx.notify();
    }

    /// A pool act (`POST /worktrees/slots`): Close, Reopen, Remove, or Release once confirmed.
    fn pool_act(&mut self, act: ipc::SlotAct, window: &mut Window, cx: &mut Context<Self>) {
        let Some((manifest, slot)) = self.begin_act() else { return };
        let holder = self.panes.bays.pool.as_ref().and_then(|pool| pool.iter().find(|one| one.slot == slot && one.manifest_id == manifest)).and_then(|one| match &one.held {
            SlotHolding::Session { holder, .. } => Some(holder.clone()),
            SlotHolding::HostedSession { session_id, .. } => Some(session_id.clone()),
            _ => None,
        });
        let lead = match act {
            ipc::SlotAct::Add => "Slot not added",
            ipc::SlotAct::Remove => "Slot not removed",
            ipc::SlotAct::Close => "Slot not closed",
            ipc::SlotAct::Open => "Slot not reopened",
            ipc::SlotAct::Release => "Slot not released",
        };
        let receipt = move |changed: Option<SlotPoolChanged>| {
            (act == ipc::SlotAct::Release).then(|| match changed.and_then(|changed| changed.released) {
                Some(released) if released.saved.is_some() => format!("Committed to {}, slot released", released.branch),
                _ => "Slot released".to_string(),
            })
        };
        if self.fleet.is_none() {
            let answer = crate::mock::pool_act(self.panes.bays.pool.get_or_insert_with(Vec::new), manifest.as_str(), slot, act);
            self.end_act(slot, answer.map(receipt).map_err(|why| format!("{lead}: {why}")), window, cx);
            return;
        }
        let body = ipc::encode(&ChangeSlotPool { act, slot: Some(slot), holder: holder.filter(|_| act == ipc::SlotAct::Release) });
        let path = format!("/worktrees/slots?manifest_id={}", super::wire::encoded(manifest.as_str()));
        self.post(
            path,
            body,
            window,
            cx,
            Box::new(move |this, answer, window, cx| {
                let said = answer.map(|bytes| receipt(ipc::decode::<SlotPoolChanged>("SlotPoolChanged", &bytes).ok())).map_err(|why| format!("{lead}: {why}"));
                this.end_act(slot, said, window, cx);
            }),
        );
    }

    /// A rescue act (`POST /worktrees/slots/rescue`): start or stop the Scout, or Pick up, Stash, or
    /// Scrap once confirmed.
    fn rescue_act(&mut self, act: RescueAct, window: &mut Window, cx: &mut Context<Self>) {
        let Some((manifest, slot)) = self.begin_act() else { return };
        let lead = match act {
            RescueAct::Start => "Scout not started",
            RescueAct::Stop => "Scout not stopped",
            RescueAct::Scrap => "Slot not scrapped",
            RescueAct::Stash => "Changes not stashed",
            RescueAct::PickUp => "Work not picked up",
        };
        let receipt = move |got: Option<SlotRescued>| {
            let got = got?;
            match act {
                RescueAct::Scrap if got.branch_kept => got.branch.map(|branch| format!("Branch {branch} kept")),
                RescueAct::Stash | RescueAct::PickUp => got.committed.map(|commit| format!("Committed {}{}", short_sha(&commit), got.branch.map(|branch| format!(" on {branch}")).unwrap_or_default())),
                _ => None,
            }
        };
        if self.fleet.is_none() {
            let answer = crate::mock::rescue_act(self.panes.bays.pool.get_or_insert_with(Vec::new), manifest.as_str(), slot, act);
            self.end_act(slot, answer.map(receipt).map_err(|why| format!("{lead}: {why}")), window, cx);
            return;
        }
        let body = ipc::encode(&RescueSlot { act, slot });
        let path = format!("/worktrees/slots/rescue?manifest_id={}", super::wire::encoded(manifest.as_str()));
        self.post(
            path,
            body,
            window,
            cx,
            Box::new(move |this, answer, window, cx| {
                let said = answer.map(|bytes| receipt(ipc::decode::<SlotRescued>("SlotRescued", &bytes).ok())).map_err(|why| format!("{lead}: {why}"));
                this.end_act(slot, said, window, cx);
            }),
        );
    }

    /// The bay panel's body: the heading and badge, its tabs, then Facts (the acts, what the last act
    /// said, a confirm, the Finding and the facts) or History.
    pub(super) fn bay_body(&self, panel: &BayPanel, look: &Look, cx: &Context<Self>) -> AnyElement {
        let Some(slot) = self.panes.bays.pool.as_ref().and_then(|pool| pool.iter().find(|one| one.slot == panel.slot && one.manifest_id == panel.manifest)) else {
            return div().p(look.length(token::SPACE_6)).child(stand_ins(look, "bay-reading", &[0.6, 0.8, 0.5])).into_any_element();
        };
        let view = view_of(slot);
        let (badge_hue, badge_icon) = badge_of(view.state);
        let hue = look.color(badge_hue);
        let badge = div()
            .id("bay-badge")
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .px(look.length(token::SPACE_2))
            .py(px(2.))
            .rounded_full()
            .border(look.length(token::BORDER_WIDTH))
            .border_color(hue.opacity(0.6))
            .bg(hue.opacity(0.12))
            .text_color(hue)
            .text_size(look.length(token::TEXT_XS))
            .tooltip(tip(view.said.clone().unwrap_or_else(|| view.word.to_string())))
            .child(glyph(look, badge_icon, Size::Small, hue))
            .child(view.word);
        let heading = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .child(div().text_size(look.length(token::TEXT_LG)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(format!("slot-{}", slot.slot))))
            .child(div().flex().child(badge));
        let tab = |id: &'static str, said: &'static str, which: BayTab| {
            let front = panel.tab == which;
            labelled(
                div()
                    .id(id)
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .border_b(look.length(token::BORDER_WIDTH) * 2.)
                    .border_color(if front { look.color(token::ACCENT) } else { gpui::transparent_black() })
                    .text_color(look.color(if front { token::FG_DEFAULT } else { token::FG_MUTED }))
                    .cursor_pointer()
                    .child(said)
                    .on_click(cx.listener(move |this, _, _, cx| this.bay_tab(which, cx))),
                Role::Tab,
                said,
            )
        };
        let tabs = div().flex().border_b(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).child(tab("bay-tab-facts", "Facts", BayTab::Facts)).child(tab("bay-tab-history", "History", BayTab::History));
        let (facts, confirm_at) = self.bay_facts(panel, slot, look, cx);
        let content = match panel.tab {
            BayTab::Facts => facts,
            BayTab::History => vec![bay_history(panel, look)],
        };
        // The heading and the tabs come first among the scroller's children.
        if let Some(at) = confirm_at.filter(|_| panel.reveal.take()) {
            panel.scroll.scroll_to_item(2 + at);
        }
        div()
            .id("bay-body")
            .track_scroll(&panel.scroll)
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_4))
            .px(look.length(token::SPACE_6))
            .py(look.length(token::SPACE_4))
            .child(heading)
            .child(tabs)
            .children(content)
            .into_any_element()
    }

    fn bay_facts(&self, panel: &BayPanel, slot: &WorktreeSlot, look: &Look, cx: &Context<Self>) -> (Vec<AnyElement>, Option<usize>) {
        type Press = Box<dyn Fn(&mut JobView, &mut Window, &mut Context<JobView>)>;
        let name = format!("slot-{}", slot.slot);
        let waiting = panel.acting || panel.confirm.is_some();
        let stranded = matches!(slot.held, SlotHolding::Stranded { .. });
        let rescue = slot.rescue.as_ref().filter(|_| stranded);
        let reading = rescue.is_some_and(|rescue| rescue.state == SlotFindingState::Reading);
        let releasable = matches!(slot.held, SlotHolding::Session { .. } | SlotHolding::HostedSession { .. });
        let removable = matches!(slot.held, SlotHolding::Free | SlotHolding::Unmade);
        let branch = slot.branch.clone().unwrap_or_else(|| "its branch".into());
        let act = |id: &'static str, label: &'static str, icon: &'static str, said: String, tone: Option<Token>, press: Press| -> AnyElement {
            let ink = look.color(tone.unwrap_or(token::FG_DEFAULT));
            let ground = look.color(token::BG_HOVER);
            labelled(
                div()
                    .id(id)
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .h(look.length(token::H_ROW))
                    .px(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(tone.map_or(look.color(token::BORDER_SUBTLE), |_| ink.opacity(0.5)))
                    .text_color(ink)
                    .text_size(look.length(token::TEXT_XS))
                    .tooltip(tip(said))
                    .child(glyph(look, icon, Size::Small, ink))
                    .child(label)
                    .when(waiting, |button| button.opacity(0.5))
                    .when(!waiting, move |button| button.cursor_pointer().hover(move |style| style.bg(ground)).on_click(cx.listener(move |this, _, window, cx| press(this, window, cx)))),
                Role::Button,
                label,
            )
            .into_any_element()
        };
        let mut acts: Vec<AnyElement> = Vec::new();
        if stranded && slot.rescue.is_none() {
            let said = format!("Starts a Scout that reads the uncommitted changes and the commits not on {} in {name}. It changes nothing.", slot.base);
            acts.push(act("bay-rescue", "Rescue", "life-buoy", said, Some(token::SLOT_STRANDED), Box::new(|this, window, cx| this.rescue_act(RescueAct::Start, window, cx))));
        }
        if reading {
            acts.push(act("bay-stop", "Stop", "power", format!("Stops the Scout reading {name}."), None, Box::new(|this, window, cx| this.rescue_act(RescueAct::Stop, window, cx))));
        }
        if releasable {
            let said = format!("Commits any uncommitted files in {name} to {branch} as a WIP commit, then releases the slot. Keeps the branch.");
            acts.push(act("bay-release", "Release", "unplug", said, None, Box::new(|this, _, cx| this.bay_confirm(Some(Confirm::Release), cx))));
        }
        if slot.closed {
            acts.push(act("bay-open", "Reopen slot", "door-open", format!("Reopens {name}: a new lease can take it again."), None, Box::new(|this, window, cx| this.pool_act(ipc::SlotAct::Open, window, cx))));
        } else {
            let said = format!("Closes {name}: no new lease will take it. Its holder keeps it until its lease ends.");
            acts.push(act("bay-close", "Close slot", "door-closed-locked", said, None, Box::new(|this, window, cx| this.pool_act(ipc::SlotAct::Close, window, cx))));
        }
        if removable {
            let said = format!("Removes {name} from the pool with git worktree remove on .armada/slots/{name}. Refused while it holds uncommitted changes.");
            acts.push(act("bay-remove", "Remove slot", "trash", said, Some(token::ERROR), Box::new(|this, window, cx| this.pool_act(ipc::SlotAct::Remove, window, cx))));
        }
        let acts_row = labelled(div().id("bay-acts").flex().flex_wrap().gap(look.length(token::SPACE_2)).children(acts), Role::Group, "Acts");
        let refused = panel.refused.clone().map(|why| labelled(div().id("bay-refused").text_color(look.color(token::ERROR)).child(SharedString::from(why.clone())), Role::Alert, why));
        let said = panel.said.clone().map(|said| labelled(div().id("bay-said").text_color(look.color(token::FG_MUTED)).child(SharedString::from(said.clone())), Role::Status, said));
        let releasing = (panel.confirm == Some(Confirm::Release)).then(|| self.release_confirm(slot, look, cx));
        let scrapping = (panel.confirm == Some(Confirm::Scrap)).then(|| self.scrap_confirm(slot, look, cx));
        let finding = rescue.map(|_| finding_of(slot, look));
        // `BayManage.tsx:39-46`: the foot follows the body, and is left out while the Scout reads (Stop is among the acts then).
        let foot = rescue.filter(|_| panel.confirm.is_none() && !reading).map(|rescue| {
            let stashed = format!("Commits the uncommitted files in {name} to branch {branch} as a WIP commit, pushes {branch} to origin and releases the slot");
            let on = !panel.acting;
            let plain = |id: &'static str, said: &'static str, hint: String, press: Press| {
                let button = small_button(look, id, said, false, on)
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .tooltip(tip(hint))
                    .when(on, |button| button.on_click(cx.listener(move |this, _, window, cx| press(this, window, cx))));
                labelled(button, Role::Button, said).into_any_element()
            };
            let mut buttons: Vec<AnyElement> = Vec::new();
            let carries_on = rescue.verdict == Some(SlotVerdict::Unfinished) || (rescue.verdict.is_none() && (rescue.summary.is_some() || !rescue.items.is_empty()));
            if carries_on {
                buttons.push(plain("bay-pick-up", "Pick up", format!("{stashed}, then carries on from {branch}."), Box::new(|this, window, cx| this.rescue_act(RescueAct::PickUp, window, cx))));
            }
            buttons.push(plain("bay-stash", "Stash", format!("{stashed}."), Box::new(|this, window, cx| this.rescue_act(RescueAct::Stash, window, cx))));
            let scrap_tip = format!("Discards the uncommitted and untracked files in {name} and releases the slot. Deletes branch {branch} only if {} has all its commits, otherwise keeps it.", slot.base);
            let scrap = danger_button(look, "bay-scrap", "Scrap", on).tooltip(tip(scrap_tip)).when(on, |button| button.on_click(cx.listener(|this, _, _, cx| this.bay_confirm(Some(Confirm::Scrap), cx))));
            buttons.push(labelled(scrap, Role::Button, "Scrap").into_any_element());
            div().flex().gap(look.length(token::SPACE_2)).children(buttons).into_any_element()
        });
        let mut body: Vec<AnyElement> = vec![acts_row.into_any_element()];
        // What an act answered stands under the acts; the acts row comes with it into view.
        let answered_at = (refused.is_some() || said.is_some()).then_some(0);
        body.extend(refused.map(IntoElement::into_any_element));
        body.extend(said.map(IntoElement::into_any_element));
        let mut confirm_at = releasing.as_ref().map(|_| body.len());
        body.extend(releasing);
        body.extend(finding);
        confirm_at = confirm_at.or(scrapping.as_ref().map(|_| body.len()));
        body.extend(scrapping);
        body.extend(foot);
        body.push(self.bay_fact_list(slot, look, cx));
        (body, confirm_at.or(answered_at))
    }

    /// Release's confirm (`ReleaseConfirm`): who holds it, that the process keeps running, then what
    /// happens to the files and the branch; Release and Cancel.
    fn release_confirm(&self, slot: &WorktreeSlot, look: &Look, cx: &Context<Self>) -> AnyElement {
        let name = format!("slot-{}", slot.slot);
        let branch = slot.branch.clone().unwrap_or_else(|| "its branch".into());
        let files = slot.stranded.as_ref().map(|stranded| stranded.uncommitted.clone()).unwrap_or_default();
        let (command, pid) = match &slot.held {
            SlotHolding::Session { holder, .. } => holder_parts(holder),
            SlotHolding::HostedSession { session_title, .. } => (session_title.clone().unwrap_or_else(|| "a Job".into()), None),
            _ => return div().into_any_element(),
        };
        let mut lines = vec![
            confirm_line(look, "key-round", "The agent job that took the slot", format!("Held by {command}, on branch {branch}"), pid.map(|pid| format!("pid {pid}")), None),
            confirm_line(look, "unplug", "The pool's record changes. The process keeps running", "Takes the checkout from that job".into(), None, None),
        ];
        if files.is_empty() {
            lines.push(confirm_line(look, "folder", "The pool takes the slot back for its next lease", format!("Releases {name}"), None, None));
            lines.push(confirm_line(look, "git-branch", "Nothing on the branch changes", format!("Keeps branch {branch}"), None, None));
        } else {
            let said = format!("Commits the uncommitted files to branch {branch} as a WIP commit");
            lines.push(confirm_line(look, "file-pen-line", "git add --all, then git commit. Ignored files are left out and nothing is pushed", said, None, Some(names(look, "Uncommitted files", &files))));
            lines.push(confirm_line(look, "folder", "The pool takes the slot back for its next lease", format!("Releases {name}"), None, None));
            lines.push(confirm_line(look, "git-branch", "The WIP commit stays on the branch", format!("Keeps branch {branch}"), None, None));
        }
        let choice = div()
            .flex()
            .gap(look.length(token::SPACE_2))
            .child(labelled(small_button(look, "release-send", "Release", false, true).border(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).on_click(cx.listener(|this, _, window, cx| this.pool_act(ipc::SlotAct::Release, window, cx))), Role::Button, "Release"))
            .child(labelled(small_button(look, "release-cancel", "Cancel", false, true).on_click(cx.listener(|this, _, _, cx| this.bay_confirm(None, cx))), Role::Button, "Cancel"));
        labelled(confirm_frame(look, "release-confirm").children(lines).child(choice), Role::Group, format!("Release {name}")).into_any_element()
    }

    /// The Scrap's confirm (`ScrapConfirm`): every uncommitted path and the commits only here; Scrap
    /// and Cancel.
    fn scrap_confirm(&self, slot: &WorktreeSlot, look: &Look, cx: &Context<Self>) -> AnyElement {
        let uncommitted = slot.stranded.as_ref().map(|stranded| stranded.uncommitted.clone()).unwrap_or_default();
        let unpushed = only_here(slot);
        let lists = div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .when(!uncommitted.is_empty(), |lists| lists.child(group(look, "Uncommitted", names(look, "Uncommitted", &uncommitted))))
            .when(!unpushed.is_empty(), |lists| lists.child(group(look, "Unpushed", names(look, "Unpushed", &unpushed))));
        let choice = div()
            .flex()
            .gap(look.length(token::SPACE_2))
            .child(labelled(danger_button(look, "scrap-send", "Scrap", true).on_click(cx.listener(|this, _, window, cx| this.rescue_act(RescueAct::Scrap, window, cx))), Role::Button, "Scrap"))
            .child(labelled(small_button(look, "scrap-cancel", "Cancel", false, true).on_click(cx.listener(|this, _, _, cx| this.bay_confirm(None, cx))), Role::Button, "Cancel"));
        labelled(confirm_frame(look, "scrap-confirm").border_color(look.color(token::ERROR).opacity(0.5)).child(lists).child(choice), Role::Group, format!("Scrap slot-{}", slot.slot)).into_any_element()
    }

    /// The facts (`side-facts`), each left out where it is empty.
    fn bay_fact_list(&self, slot: &WorktreeSlot, look: &Look, cx: &Context<Self>) -> AnyElement {
        let view = view_of(slot);
        let held_for = slot
            .since
            .as_ref()
            .filter(|_| view.state == BayState::Leased)
            .and_then(|since| DateTime::parse_from_rfc3339(since.as_str()).ok())
            .map(|since| span_of((Utc::now() - since.with_timezone(&Utc)).num_milliseconds()));
        let uncommitted = slot.stranded.as_ref().map(|stranded| stranded.uncommitted.clone()).unwrap_or_default();
        let unpushed = only_here(slot);
        let row = |term: &'static str, value: AnyElement| {
            div()
                .flex()
                .gap(look.length(token::SPACE_4))
                .py(look.length(token::SPACE_1))
                .child(div().flex_none().w(look.length(token::SPACE_12) * 2.).text_color(look.color(token::FG_MUTED)).child(term))
                .child(div().flex_1().min_w_0().child(value))
        };
        let text = |value: String, is_mono: bool| div().when(is_mono, |cell| cell.font_family(mono(look)).text_size(look.length(token::TEXT_XS))).child(SharedString::from(value)).into_any_element();
        let lines = |items: &[String]| div().flex().flex_col().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).children(items.iter().map(|one| div().truncate().child(SharedString::from(one.clone())))).into_any_element();
        let mut rows = Vec::new();
        if let Some((id, title)) = view.holder.clone() {
            let label = title.unwrap_or_else(|| short_of(&id));
            let holder = ClashHolder { kind: ClashSide::Session, id, label: label.clone() };
            let ground = look.color(token::BG_HOVER);
            rows.push(row(
                "Holder",
                labelled(
                    div()
                        .id("bay-holder")
                        .rounded(look.length(token::RADIUS_SM))
                        .text_color(look.color(token::ACCENT))
                        .cursor_pointer()
                        .hover(move |style| style.bg(ground))
                        .child(SharedString::from(label.clone()))
                        .on_click(cx.listener(move |_, _, window, cx| {
                            if let Some(open) = cx.try_global::<crate::clash_mark::OpenHolder>().cloned() {
                                (open.0)(&holder, window, cx);
                            }
                        })),
                    Role::Button,
                    format!("Job: {label}"),
                )
                .into_any_element(),
            ));
        }
        if let Some(branch) = slot.branch.clone() {
            rows.push(row("Branch", text(branch, true)));
        }
        if let Some(held_for) = held_for {
            rows.push(row("Held for", text(held_for, false)));
        }
        rows.push(row("Base", text(slot.base.clone(), true)));
        if let Some(behind) = slot.behind.filter(|behind| *behind > 0) {
            rows.push(row("Behind", text(plural(behind as usize, "commit", "commits"), false)));
        }
        rows.push(row("Checkout", text(if slot.warm { "Warm".into() } else { "Cold".into() }, false)));
        if !uncommitted.is_empty() {
            rows.push(row("Uncommitted", lines(&uncommitted)));
        }
        if !unpushed.is_empty() {
            rows.push(row("Unpushed", lines(&unpushed)));
        }
        rows.push(row("Path", text(slot.path.clone(), true)));
        labelled(div().id("bay-facts").flex().flex_col().children(rows), Role::List, "Facts").into_any_element()
    }

    /// `GET /clashes/file`: one file of a clash, as each side changed it.
    fn open_clash_file(&mut self, with: ClashHolder, path: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        if self.fleet.is_none() {
            let read = crate::mock::clash_file(&with.id, &path).ok_or_else(|| "Fleet could not read this file".to_string());
            self.open_clash_panel(ClashFilePanel { with, path, read: Some(read) }, window, cx);
            return;
        }
        let kind = if with.kind == ClashSide::Main { "main" } else { "session" };
        let query = format!("/clashes/file?session_id={}&other_kind={kind}&other_id={}&path={}", super::wire::encoded(&job), super::wire::encoded(&with.id), super::wire::encoded(&path));
        let (key_with, key_path) = (with.id.clone(), path.clone());
        self.open_clash_panel(ClashFilePanel { with, path, read: None }, window, cx);
        self.get(
            query,
            window,
            cx,
            Box::new(move |this, answer, _, cx| {
                let read = answer.and_then(|bytes| ipc::decode::<ClashFile>("ClashFile", &bytes).map_err(|why| why.to_string()));
                if let Some(panel) = this.clash_panel_mut().filter(|panel| panel.with.id == key_with && panel.path == key_path) {
                    panel.read = Some(read);
                }
                cx.notify();
            }),
        );
    }

    /// Both sides of the file, each under the name of whose change it is.
    pub(super) fn clash_file_body(&self, panel: &ClashFilePanel, look: &Look, cx: &Context<Self>) -> AnyElement {
        let body = div().id("clash-file-body").flex_1().min_h_0().overflow_y_scroll().flex().flex_col().gap(look.length(token::SPACE_4)).px(look.length(token::SPACE_6)).py(look.length(token::SPACE_4));
        let Some(read) = panel.read.as_ref() else { return body.child(stand_ins(look, "clash-file", &[0.9, 0.8, 0.7, 0.6])).into_any_element() };
        let file = match read {
            Ok(file) => file,
            Err(why) => return body.child(labelled(div().id("clash-file-note").text_color(look.color(token::FG_MUTED)).child(SharedString::from(why.clone())), Role::Note, why.clone())).into_any_element(),
        };
        let ours = self.shown.as_ref().and_then(|id| self.record(id, cx)).and_then(|job| job.title.clone()).unwrap_or_else(|| "This Job".to_string());
        let theirs = panel.with.label.clone();
        let cut = |shown: usize, total: usize| format!("The first {shown} of {total} lines.");
        let (mut mine, mut other) = (diff::drawn_of(&file.ours, cut), diff::drawn_of(&file.theirs, cut));
        // Each side's colliding hunks, marked against the other's as Classic's sheet does (`markCollisions`).
        let plain = (mine.clone(), other.clone());
        diff::mark_collisions(&mut mine, &plain.1, &format!("Collides with {theirs}"));
        diff::mark_collisions(&mut other, &plain.0, &format!("Collides with {ours}"));
        let side = |key: &'static str, name: String, drawn: diff::Drawn| {
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(name)))
                .child(if drawn.files.is_empty() {
                    div().text_color(look.color(token::FG_MUTED)).child("No lines changed in this file.").into_any_element()
                } else {
                    diff::unified(key, &drawn, look)
                })
        };
        body.child(side("clash-ours", ours, mine)).child(side("clash-theirs", theirs, other)).into_any_element()
    }
}

fn bay_history(panel: &BayPanel, look: &Look) -> AnyElement {
    let Some(history) = panel.history.as_ref() else { return stand_ins(look, "bay-history", &[0.8, 0.7, 0.6]) };
    let rows = history.iter().enumerate().map(|(at, one)| {
        div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .min_h(look.length(token::H_ROW))
            .child(
                div()
                    .id(SharedString::from(format!("history-span:{at}")))
                    .flex_none()
                    .w(look.length(token::SPACE_12) + look.length(token::SPACE_4))
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .tooltip(tip("Held"))
                    .child(SharedString::from(one.span.clone())),
            )
            .child(div().flex_1().min_w_0().truncate().child(SharedString::from(one.who.clone())))
            .children(one.branch.clone().map(|branch| div().flex_shrink_0().max_w(look.length(token::SPACE_12) * 4.).truncate().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(branch))))
    });
    labelled(div().id("bay-history").flex().flex_col().children(rows), Role::List, "History").into_any_element()
}

/// What the rescue Scout read (`SlotFinding`): its verdict and items, its marks, then the files it
/// read, the searches and the slot's commits, the ones only here first.
fn finding_of(slot: &WorktreeSlot, look: &Look) -> AnyElement {
    let Some(rescue) = slot.rescue.as_ref() else { return div().into_any_element() };
    let verdict = rescue.verdict.map(|verdict| {
        let unfinished = verdict == SlotVerdict::Unfinished;
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(if unfinished { "Unfinished" } else { "Scraps" }))
            .when(unfinished && !rescue.items.is_empty(), |answer| {
                answer.child(labelled(div().id("finding-left").flex().flex_col().children(rescue.items.iter().map(|one| div().child(SharedString::from(format!("\u{2022} {one}"))))), Role::List, "Left to do"))
            })
            .when(!unfinished && !rescue.items.is_empty(), |answer| answer.child(div().text_color(look.color(token::FG_MUTED)).child(SharedString::from(rescue.items[0].clone()))))
    });
    let mark = |id: &'static str, icon: &'static str, said: &'static str| labelled(div().id(id).tooltip(tip(said)).child(glyph(look, icon, Size::Small, look.color(token::FG_MUTED))), Role::Image, said).into_any_element();
    let mut meta: Vec<AnyElement> = Vec::new();
    if rescue.state == SlotFindingState::Reading {
        meta.push(mark("finding-reading", "scan-search", "Reading"));
    }
    if rescue.state == SlotFindingState::Stopped {
        meta.push(mark("finding-stopped", "power", "Stopped"));
    }
    meta.push(
        labelled(
            div()
                .id("finding-commit")
                .flex()
                .items_center()
                .gap(px(2.))
                .tooltip(tip("Commit it read"))
                .child(glyph(look, "git-commit-horizontal", Size::Small, look.color(token::FG_MUTED)))
                .child(SharedString::from(short_sha(&rescue.commit).to_string())),
            Role::Image,
            format!("Commit it read: {}", short_sha(&rescue.commit)),
        )
        .into_any_element(),
    );
    if rescue.uncommitted {
        meta.push(mark("finding-uncommitted", "file-pen-line", "Uncommitted changes on top"));
    }
    if rescue.cut > 0 {
        meta.push(labelled(div().id("finding-cut").tooltip(tip("Characters of the change cut before it read them")).child(SharedString::from(format!("{} cut", rescue.cut))), Role::Image, format!("{} cut", rescue.cut)).into_any_element());
    }
    let failed = rescue.why.clone().filter(|_| rescue.state == SlotFindingState::Failed).map(|why| div().text_color(look.color(token::ERROR)).child(SharedString::from(why)));
    let summary = rescue.summary.clone().map(|summary| div().child(SharedString::from(summary)));
    let mut commits: Vec<&ipc::SlotCommit> = slot.stranded.as_ref().map(|stranded| stranded.commits.iter().collect()).unwrap_or_default();
    commits.sort_by_key(|one| one.home != CommitHome::OnlyHere);
    let commit_rows = (!commits.is_empty()).then(|| {
        let rows = commits.into_iter().map(|one| {
            let (tag, said) = match one.home {
                CommitHome::OnlyHere => ("Only here".to_string(), format!("On no remote branch and not on {}", slot.base)),
                CommitHome::OnRemote => ("On the remote".to_string(), "On a remote branch".to_string()),
                CommitHome::OnMain => (format!("On {}", slot.base), format!("On local {}, not pushed", slot.base)),
            };
            let tone = look.color(if one.home == CommitHome::OnlyHere { token::SLOT_STRANDED } else { token::FG_MUTED });
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(div().flex_1().min_w_0().truncate().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).child(SharedString::from(format!("{} {}", short_sha(&one.sha), one.subject))))
                .child(
                    div()
                        .id(SharedString::from(format!("commit-home:{}", one.sha)))
                        .flex_none()
                        .px(look.length(token::SPACE_1))
                        .rounded(look.length(token::RADIUS_SM))
                        .bg(tone.opacity(0.15))
                        .text_color(tone)
                        .text_size(look.length(token::TEXT_2XS))
                        .tooltip(tip(said))
                        .child(SharedString::from(tag)),
                )
        });
        group(look, "Commits", labelled(div().id("finding-commits").flex().flex_col().children(rows), Role::List, "Commits").into_any_element())
    });
    let lists = div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_2))
        .when(!rescue.read.is_empty(), |lists| lists.child(group(look, "Read", names(look, "Read", &rescue.read))))
        .when(!rescue.searched.is_empty(), |lists| lists.child(group(look, "Searched", names(look, "Searched", &rescue.searched))))
        .children(commit_rows);
    labelled(
        confirm_frame(look, "finding")
            .children(verdict)
            .child(div().flex().items_center().gap(look.length(token::SPACE_2)).font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_MUTED)).children(meta))
            .children(failed)
            .children(summary)
            .child(lists),
        Role::Group,
        "Finding",
    )
    .into_any_element()
}

fn danger_button(look: &Look, id: &'static str, said: &'static str, on: bool) -> gpui::Stateful<gpui::Div> {
    let ground = look.color(token::ERROR);
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .h(look.length(token::H_ROW))
        .px(look.length(token::SPACE_3))
        .rounded(look.length(token::RADIUS_MD))
        .bg(ground)
        .text_color(gpui::white())
        .text_size(look.length(token::TEXT_XS))
        .child(said)
        .when(on, |button| button.cursor_pointer().hover(move |style| style.bg(ground.opacity(0.85))))
        .when(!on, |button| button.opacity(0.5))
}

fn confirm_frame(look: &Look, id: &'static str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_2))
        .p(look.length(token::SPACE_3))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(look.color(token::BORDER_SUBTLE))
        .bg(look.color(token::BG_SUNKEN))
}

/// One line of a confirm (`ConfirmLine`): a glyph that names what it is on hover, the words, a figure.
fn confirm_line(look: &Look, icon: &'static str, said: &'static str, word: String, figure: Option<String>, under: Option<AnyElement>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(look.length(token::SPACE_1))
        .child(
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(labelled(div().id(SharedString::from(format!("confirm-mark:{icon}"))).flex_none().tooltip(tip(said)).child(glyph(look, icon, Size::Small, look.color(token::FG_MUTED))), Role::Image, said))
                .child(div().flex_1().min_w_0().child(SharedString::from(word)))
                .children(figure.map(|figure| {
                    div().id("confirm-pid").flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_MUTED)).tooltip(tip("Process id")).child(SharedString::from(figure))
                })),
        )
        .children(under)
        .into_any_element()
}

/// A list of names in mono (`Names`).
fn names(look: &Look, label: &'static str, items: &[String]) -> AnyElement {
    labelled(
        div()
            .id(label)
            .flex()
            .flex_col()
            .pl(look.length(token::SPACE_4))
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_XS))
            .text_color(look.color(token::FG_MUTED))
            .children(items.iter().map(|one| div().truncate().child(SharedString::from(one.clone())))),
        Role::List,
        label,
    )
    .into_any_element()
}

fn group(look: &Look, label: &'static str, body: AnyElement) -> gpui::Div {
    div().flex().flex_col().gap(look.length(token::SPACE_1)).child(div().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(label)).child(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(json: &str) -> WorktreeSlot {
        ipc::decode::<WorktreeSlot>("WorktreeSlot", json.as_bytes()).expect("a slot")
    }

    #[test]
    fn a_long_id_is_cut_to_its_ends() {
        assert_eq!(short_of("session-abcdef-1234"), "sess\u{2026}1234");
        assert_eq!(short_of("mock-01"), "mock-01");
    }

    #[test]
    fn a_span_reads_in_minutes_then_hours() {
        assert_eq!(span_of(48 * 60_000), "48m");
        assert_eq!(span_of((2 * 60 + 5) * 60_000), "2h 05m");
    }

    #[test]
    fn a_terminal_holder_splits_into_command_and_pid() {
        assert_eq!(holder_parts("nvim (pid 44698)"), ("nvim".to_string(), Some("44698".to_string())));
        assert_eq!(holder_parts("a Job"), ("a Job".to_string(), None));
    }

    #[test]
    fn a_stranded_bay_reads_stranded_before_closed_and_names_why() {
        let one = slot(r#"{"manifest_id":"armada","slot":4,"path":"/x","held":{"state":"stranded","why":"its Job ended"},"base":"main","warm":true,"closed":true}"#);
        let view = view_of(&one);
        assert_eq!(view.state, BayState::Stranded);
        assert_eq!(view.said.as_deref(), Some("Stranded: its Job ended"));
    }

    #[test]
    fn a_free_bay_closed_reads_closed() {
        let one = slot(r#"{"manifest_id":"armada","slot":2,"path":"/x","held":{"state":"free"},"base":"main","warm":false,"closed":true}"#);
        assert_eq!(view_of(&one).state, BayState::Closed);
    }
}
