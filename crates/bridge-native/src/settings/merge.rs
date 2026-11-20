//! Merging two projects of one name from the picker: the card that says which stays and what comes over, asks which
//! of two differing values to keep, and `POST /projects/merge` once Merge is pressed. The approved sketch "Merge into:
//! picker row and card" (17 Nov 2026).
//!
//! **Everything the card says is worked out from the two projects** (`facts`), so it is read without a window.

use gpui::{div, prelude::*, svg, AnyElement, Context, FontWeight, Keystroke, MouseButton, Role, SharedString, Window};
use ipc::{MacColour, Project, ProjectMember, ProjectRow, ProjectTaskState, ValuesSet};

use super::overlay::SettingsOverlay;
use super::project::{owner_of, Key, THIS_MAC};
use super::rows::{segment, segment_track, Pick};
use super::view::{ghost_with, refusal, ringed, small, switch_track, tip};
use super::wire::Refused;
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, Look};

/// The merge being asked about, and what he has chosen on it.
pub struct Merging {
    pub kept: String,
    pub gone: String,
    /// The value keys where he takes the gone project's word.
    pub from_gone: Vec<String>,
    /// Fleet's refusal, in its words.
    pub refused: Option<String>,
    pub sending: bool,
}

/// One project's tile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tile {
    pub id: String,
    pub name: String,
    /// Whose it is: This Mac, or the Mac's name and colour.
    pub owner: String,
    pub colour: Option<MacColour>,
    pub tasks: usize,
    pub jobs: usize,
}

impl Tile {
    /// "12 tasks · 3 Jobs".
    pub fn counts(&self) -> String {
        format!("{} · {}", plural(self.tasks, "task"), plural(self.jobs, "Job"))
    }
}

/// A value both projects set, to different words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict {
    pub key: &'static str,
    pub name: &'static str,
    pub kept: String,
    pub gone: String,
}

/// What the card says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Facts {
    pub title: String,
    pub kept: Tile,
    pub gone: Tile,
    pub sentence: String,
    pub conflicts: Vec<Conflict>,
    /// Tasks a Job holds on the gone side, which come over open.
    pub held: Option<String>,
    /// The kept project is this Mac's own and is not shared, so Fleet refuses the merge.
    pub needs_share: bool,
    /// What the share row says: Merge needs the kept project shared, and what follows from it.
    pub share_said: String,
    /// Where the other Mac runs an older build: why Merge is off. It cannot read the acts a merge sends.
    pub older: Option<String>,
}

fn plural(n: usize, one: &str) -> String {
    if n == 1 { format!("1 {one}") } else { format!("{n} {one}s") }
}

/// A Mac's name, lowercased where it is This Mac and not at the start of a sentence.
fn mid(owner: &str) -> String {
    if owner == THIS_MAC { "this Mac".into() } else { owner.to_string() }
}

fn possessive(owner: &str, start: bool) -> String {
    format!("{}'s", if start { owner.to_string() } else { mid(owner) })
}

fn joined(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [only] => only.clone(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

fn repository_of(member: &ProjectMember) -> String {
    match &member.folder {
        Some(folder) => format!("{}/{folder}", member.manifest_id.as_str()),
        None => member.manifest_id.as_str().to_string(),
    }
}

fn tasks_of(one: &Project) -> usize {
    one.rows.iter().filter(|row| matches!(row, ProjectRow::Task { .. })).count()
}

/// The words a value key goes by.
fn key_name(key: &str) -> &'static str {
    match key {
        "pr_mode" => Key::PrMode.title(),
        "auto_merge" => Key::AutoMerge.title(),
        "review_model" => Key::ReviewModel.title(),
        "look" => "Look",
        "tracker" => Key::Tracker.title(),
        "reviews" => "Reviews",
        "agent_starts" => Key::AgentStarts.title(),
        "delivery" => Key::Delivery.title(),
        "workflows_off" => "Workflows switched off",
        _ => "Value",
    }
}

/// A word as the settings pages say it, else as it is.
fn word_said(key: Key, word: &str) -> String {
    key.fixed().into_iter().find(|(one, _)| one == word).map_or_else(|| word.to_string(), |(_, said)| said)
}

/// What a project's value for `key` reads as in a segment.
fn value_said(key: &str, set: &ValuesSet) -> String {
    let some = |key: Key, word: &Option<String>| word.as_deref().map(|word| format!("{} {}", key.title(), word_said(key, word)));
    match key {
        "pr_mode" => set.pr_mode.as_ref().map(|mode| word_said(Key::PrMode, mode.as_wire())),
        "auto_merge" => set.auto_merge.as_deref().map(|word| word_said(Key::AutoMerge, word)),
        "review_model" => set.review_model.clone(),
        "look" => set.look.as_ref().map(|look| {
            let parts: Vec<String> = [some(Key::Theme, &look.theme), some(Key::Layout, &look.layout), some(Key::Density, &look.density)].into_iter().flatten().collect();
            parts.join(", ")
        }),
        "tracker" => set.tracker.as_ref().map(|block| block.get("provider").map_or_else(|| "Set".to_string(), |provider| word_said(Key::Tracker, provider))),
        "reviews" => set.reviews.as_ref().map(|reviews| {
            let parts: Vec<String> = [some(Key::Plans, &reviews.plans), some(Key::PullRequests, &reviews.pull_requests), some(Key::Walks, &reviews.walks)].into_iter().flatten().collect();
            parts.join(", ")
        }),
        "agent_starts" => set.agent_starts.as_deref().map(|word| word_said(Key::AgentStarts, word)),
        "delivery" => set.delivery.as_deref().map(|word| word_said(Key::Delivery, word)),
        "workflows_off" => Some(set.workflows_off.join(", ")).filter(|said| !said.is_empty()),
        _ => None,
    }
    .unwrap_or_default()
}

/// The card's words from the two projects. `jobs` counts the Jobs Bridge holds that were started for a project.
pub fn facts(kept: &Project, gone: &Project, macs: &[ipc::Mac], jobs: &dyn Fn(&str) -> usize) -> Facts {
    let tile = |one: &Project| Tile {
        id: one.id.clone(),
        name: one.name.clone(),
        owner: owner_of(macs, one),
        colour: one.mac.as_deref().map(|id| super::project::mac_of(macs, id).1),
        tasks: tasks_of(one),
        jobs: jobs(&one.id),
    };
    let (kept_tile, gone_tile) = (tile(kept), tile(gone));
    let (kept_values, gone_values) = (kept.values.clone().unwrap_or_default(), gone.values.clone().unwrap_or_default());
    let conflicts = kept_values
        .conflicts(&gone_values)
        .into_iter()
        .map(|key| Conflict { key, name: key_name(key), kept: value_said(key, &kept_values), gone: value_said(key, &gone_values) })
        .collect();

    let repositories: Vec<String> = gone.members.iter().filter(|member| !kept.members.contains(member)).map(repository_of).collect();
    let mut parts = Vec::new();
    if gone_tile.tasks > 0 {
        parts.push(format!("its {}", plural(gone_tile.tasks, "task")));
    }
    if gone_tile.jobs > 0 {
        parts.push(if gone_tile.jobs == 1 { "its Job".to_string() } else { format!("its {} Jobs", gone_tile.jobs) });
    }
    match repositories.as_slice() {
        [] => {}
        [one] => parts.push(format!("the repository {one}")),
        many => parts.push(format!("the repositories {}", joined(many))),
    }
    let singular = parts.len() == 1 && gone_tile.tasks <= 1 && gone_tile.jobs <= 1 && repositories.len() <= 1;
    let mut sentence = format!("{} {} goes.", possessive(&gone_tile.owner, true), gone_tile.name);
    if !parts.is_empty() {
        let mut said = format!("{} {} into {} {}.", joined(&parts), if singular { "comes" } else { "come" }, possessive(&kept_tile.owner, false), kept_tile.name);
        if let Some(first) = said.get(..1) {
            said = format!("{}{}", first.to_uppercase(), &said[1..]);
        }
        sentence = format!("{sentence} {said}");
    }

    let held_tasks = gone.rows.iter().filter(|row| matches!(row, ProjectRow::Task { state: ProjectTaskState::Held, .. })).count();
    let holders = {
        let mut ids: Vec<&str> = gone.rows.iter().filter_map(|row| if let ProjectRow::Task { held_by: Some(holder), .. } = row { Some(holder.session_id.as_str()) } else { None }).collect();
        ids.sort_unstable();
        ids.dedup();
        ids.len().max(1)
    };
    let held = (held_tasks > 0).then(|| {
        let on = mid(&gone_tile.owner);
        let (jobs, verb) = if holders == 1 { ("its Job", "holds") } else { ("its Jobs", "hold") };
        format!("{} {jobs} on {on} {verb} {} over open.", plural(held_tasks, "task"), if held_tasks == 1 { "comes" } else { "come" })
    });

    let needs = format!("Merge is off until {} is shared with your other Macs.", kept.name);
    let share_said = match gone_tile.jobs {
        0 => needs,
        1 => format!("{needs} {} Job follows {} only once it is shared.", possessive(&gone_tile.owner, true), kept.name),
        _ => format!("{needs} {} Jobs follow {} only once it is shared.", possessive(&gone_tile.owner, true), kept.name),
    };
    let older = [kept, gone].into_iter().find_map(|one| one.mac.as_deref()).and_then(|id| macs.iter().find(|mac| mac.id == id)).filter(|mac| mac.link == Some(ipc::MacLink::OlderBuild)).map(|mac| format!("{} runs an older build. Merge is off until it updates", mac.name));
    Facts {
        title: format!("Merge the two {} projects", kept.name),
        needs_share: kept.mac.is_none() && !kept.shared_with_macs,
        kept: kept_tile,
        gone: gone_tile,
        sentence,
        conflicts,
        held,
        share_said,
        older,
    }
}

/// A place the keys rest on in the card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    Swap,
    Share,
    Value(usize),
    Cancel,
    Merge,
}

impl SettingsOverlay {
    /// The two projects of the merge asked, kept first.
    fn merge_pair(&self) -> Option<(&Project, &Project)> {
        let asking = self.scope.merging.as_ref()?;
        let all = &self.scope.projects.as_ref()?.projects;
        Some((all.iter().find(|one| one.id == asking.kept)?, all.iter().find(|one| one.id == asking.gone)?))
    }

    fn merge_facts(&self, cx: &gpui::App) -> Option<Facts> {
        let (kept, gone) = self.merge_pair()?;
        Some(facts(kept, gone, crate::pairing::macs(cx), &|id| self.wire.project_jobs(id, cx)))
    }

    /// The places the keys rest on, in the order Tab walks them.
    pub fn merge_stops(&self, facts: &Facts) -> Vec<(Stop, gpui::FocusHandle)> {
        let mut stops = vec![(Stop::Swap, self.scope.merge_swap.clone())];
        if facts.needs_share {
            stops.push((Stop::Share, self.scope.merge_share.clone()));
        }
        stops.extend(facts.conflicts.iter().enumerate().filter_map(|(n, _)| self.scope.merge_values.get(n).map(|handle| (Stop::Value(n), handle.clone()))));
        stops.push((Stop::Cancel, self.scope.cancel_stop.clone()));
        stops.push((Stop::Merge, self.scope.merge_confirm.clone()));
        stops
    }

    /// The picker's Merge on project `id`, the row it was pressed beside: that project and its twin, this Mac's own kept.
    pub fn ask_merge(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(all) = self.scope.projects.as_ref().map(|all| all.projects.as_slice()) else { return };
        let Some(edited) = all.iter().find(|one| one.id == id) else { return };
        let Some(twin) = super::project::twin_of(edited, all) else { return };
        let (own, copy) = if edited.mac.is_none() { (edited, twin) } else { (twin, edited) };
        self.scope.merging = Some(Merging { kept: own.id.clone(), gone: copy.id.clone(), from_gone: Vec::new(), refused: None, sending: false });
        let cancel = self.scope.cancel_stop.clone();
        window.defer(cx, move |window, cx| window.focus(&cancel, cx));
        cx.notify();
    }

    /// Cancel, Esc, and a press outside the card: nothing is changed, and the keys go back to the picker.
    pub fn cancel_merge(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.scope.merging.as_ref().is_none_or(|asking| asking.sending) {
            return;
        }
        self.scope.merging = None;
        window.focus(&self.scope.picker, cx);
        cx.notify();
    }

    /// Swap: the other project is kept, and what he chose between differing values goes, as it was about the other side.
    pub fn swap_merge(&mut self, cx: &mut Context<Self>) {
        let Some(asking) = self.scope.merging.as_mut().filter(|asking| !asking.sending) else { return };
        std::mem::swap(&mut asking.kept, &mut asking.gone);
        asking.from_gone.clear();
        asking.refused = None;
        cx.notify();
    }

    /// The segment of a differing value: `from_gone` takes the gone project's word, and not takes the kept one's.
    pub fn choose_value(&mut self, n: usize, from_gone: bool, cx: &mut Context<Self>) {
        let Some(key) = self.merge_facts(cx).and_then(|facts| facts.conflicts.get(n).map(|one| one.key)) else { return };
        let Some(asking) = self.scope.merging.as_mut().filter(|asking| !asking.sending) else { return };
        asking.from_gone.retain(|one| one != key);
        if from_gone {
            asking.from_gone.push(key.to_string());
        }
        cx.notify();
    }

    /// The switch: share the kept project with his other Macs, as the edit with `shared_with_macs` alone.
    pub fn share_kept(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(asking) = self.scope.merging.as_mut().filter(|asking| !asking.sending) else { return };
        if self.scope.saving.is_some() {
            return;
        }
        asking.refused = None;
        let id = asking.kept.clone();
        self.scope.saving = Some((Key::Shared, super::project::Column::Own));
        let sent = self.wire.share_with_macs(&id, true, cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = sent.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.scope.saving = None;
                match answer {
                    Ok(projects) => {
                        this.scope.projects = Some(projects);
                        // The switch is gone once it is on: the keys go to Merge.
                        window.focus(&this.scope.merge_confirm, cx);
                    }
                    Err(why) => {
                        if let Some(asking) = this.scope.merging.as_mut() {
                            asking.refused = Some(why.said);
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Merge: Fleet makes the two one and answers every project; the card goes and the picker keeps the kept project.
    /// A refusal is said in the card, which stays.
    pub fn confirm_merge(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(facts) = self.merge_facts(cx) else { return };
        let Some(asking) = self.scope.merging.as_mut() else { return };
        if asking.sending || facts.needs_share || facts.older.is_some() {
            return;
        }
        asking.from_gone.retain(|key| facts.conflicts.iter().any(|one| one.key == key));
        asking.sending = true;
        asking.refused = None;
        let merge = ipc::MergeProjects { kept: asking.kept.clone(), gone: asking.gone.clone(), from_gone: asking.from_gone.clone() };
        let kept = (facts.kept.id.clone(), facts.kept.name.clone());
        let sent = self.wire.merge_projects(merge, cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = sent.await;
            let _ = this.update_in(cx, |this, window, cx| this.merged(kept, answer, window, cx));
        })
        .detach();
        cx.notify();
    }

    fn merged(&mut self, kept: (String, String), answer: Result<ipc::Projects, Refused>, window: &mut Window, cx: &mut Context<Self>) {
        match answer {
            Ok(projects) => {
                self.scope.projects = Some(projects);
                self.scope.merging = None;
                self.edit(Some(kept), window, cx);
                window.focus(&self.scope.picker, cx);
            }
            Err(why) => {
                if let Some(asking) = self.scope.merging.as_mut() {
                    asking.sending = false;
                    asking.refused = Some(why.said);
                }
            }
        }
        cx.notify();
    }

    /// Press the stop the keys are on.
    fn press_stop(&mut self, stop: Stop, window: &mut Window, cx: &mut Context<Self>) {
        match stop {
            Stop::Swap => self.swap_merge(cx),
            Stop::Share => self.share_kept(window, cx),
            Stop::Value(n) => {
                let from_gone = self.merge_facts(cx).and_then(|facts| facts.conflicts.get(n).map(|one| one.key)).is_some_and(|key| self.scope.merging.as_ref().is_some_and(|asking| !asking.from_gone.iter().any(|one| one == key)));
                self.choose_value(n, from_gone, cx);
            }
            Stop::Cancel => self.cancel_merge(window, cx),
            Stop::Merge => self.confirm_merge(window, cx),
        }
    }

    /// The card's keys, while it is asked: ⌘S swaps, ⌘↩ merges, Esc cancels, Tab and the arrows move between its
    /// controls, ← and → choose on a differing value, Enter and Space press. ⌘ keys besides those go to the app.
    pub(super) fn merge_key(&mut self, key: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(facts) = self.merge_facts(cx) else { return false };
        let held = &key.modifiers;
        if held.platform && !held.control && !held.alt && !held.shift {
            return match key.key.as_str() {
                "enter" => {
                    self.confirm_merge(window, cx);
                    true
                }
                "s" => {
                    self.swap_merge(cx);
                    true
                }
                _ => false,
            };
        }
        if held.platform || held.control || held.alt {
            return false;
        }
        let stops = self.merge_stops(&facts);
        let at = stops.iter().position(|(_, handle)| handle.is_focused(window));
        let go = |by: isize, window: &mut Window, cx: &mut Context<Self>| {
            let next = match at {
                Some(at) => (at as isize + by).rem_euclid(stops.len() as isize) as usize,
                None => 0,
            };
            window.focus(&stops[next].1, cx);
            cx.notify();
        };
        let on = at.map(|at| stops[at].0);
        match (key.key.as_str(), on) {
            ("escape", _) => self.cancel_merge(window, cx),
            ("tab", _) => go(if held.shift { -1 } else { 1 }, window, cx),
            ("left" | "right", Some(Stop::Value(n))) => {
                let was = self.scope.merging.as_ref().map(|asking| asking.from_gone.clone());
                self.choose_value(n, key.key == "right", cx);
                if was != self.scope.merging.as_ref().map(|asking| asking.from_gone.clone()) {
                    crate::ui_sounds::moved();
                }
            }
            ("up" | "left", _) => {
                crate::ui_sounds::moved();
                go(-1, window, cx)
            }
            ("down" | "right", _) => {
                crate::ui_sounds::moved();
                go(1, window, cx)
            }
            ("enter" | "space", Some(stop)) => self.press_stop(stop, window, cx),
            _ => {}
        }
        true
    }

    /// The card over the page: the two tiles, what comes over, the values to choose between, then Cancel and Merge.
    pub(super) fn render_merge(&self, look: &Look, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        let asking = self.scope.merging.as_ref()?;
        let facts = self.merge_facts(cx)?;
        let ink = look.color(token::FG_DEFAULT);
        let muted = look.color(token::FG_MUTED);
        let glyph = look.length(token::SPACE_4);
        let escalated = look.color(token::STATUS_ESCALATED);
        let success = look.color(token::STATUS_COMPLETED_SUCCESS);
        let busy = asking.sending;

        let tile = |tile: &Tile, kept: bool| {
            let hue = if kept { success } else { muted };
            let hue = tile.colour.map_or_else(|| look.color(token::FG_MUTED), |colour| crate::settings::macs::colour_of(look, colour));
            let chip = crate::pairing::pill_capped(look, SharedString::from(format!("merge-chip-{}", tile.id)), &tile.owner, hue);
            let name = format!("{} {}: {}, {}", if kept { "Kept" } else { "Goes" }, tile.name, tile.owner, tile.counts());
            labelled(
                div()
                    .id(SharedString::from(if kept { "merge-kept" } else { "merge-gone" }))
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .p(look.length(token::SPACE_3))
                    .rounded(look.length(token::RADIUS_LG))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(if kept { success } else { look.color(token::BORDER_DEFAULT) })
                    .bg(if kept { look.color(token::STATUS_COMPLETED_SUCCESS_BG) } else { look.color(token::BG_SUNKEN) })
                    .when(!kept, |tile| tile.opacity(0.6))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_1))
                            .text_size(look.length(token::TEXT_2XS))
                            .line_height(look.length(token::LEADING_2XS))
                            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                            .text_color(hue)
                            .child(svg().flex_none().path(icon_path(if kept { "check" } else { "x" })).size(look.length(token::SPACE_3)).text_color(hue))
                            .child(if kept { "KEPT" } else { "GOES" }),
                    )
                    .child(div().flex().items_center().gap(look.length(token::SPACE_2)).child(div().min_w(look.length(token::SPACE_12) * 1.5).truncate().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(tile.name.clone()))).child(chip))
                    .child(small(look, tile.counts(), muted)),
                Role::Group,
                name,
            )
        };

        let swap = ringed(look, ghost_with(look, "merge-swap", Some("arrow-left-right"), "Swap", busy), &self.scope.merge_swap, window)
            .track_focus(&self.scope.merge_swap)
            .flex_col()
            .h_auto()
            .py(look.length(token::SPACE_2))
            .tooltip(tip("⌘S"))
            .when(!busy, |button| button.on_click(cx.listener(|this, _, _, cx| this.swap_merge(cx))));
        let tiles = div().flex().items_center().gap(look.length(token::SPACE_3)).child(tile(&facts.kept, true)).child(labelled(swap, Role::Button, "Swap kept and gone")).child(tile(&facts.gone, false));

        let differ = (!facts.conflicts.is_empty()).then(|| {
            let rows = facts.conflicts.iter().enumerate().map(|(n, one)| {
                let from_gone = asking.from_gone.iter().any(|key| key == one.key);
                let focus = self.scope.merge_values.get(n).cloned();
                let mut track = segment_track(look, SharedString::from(format!("merge-value-{}", one.key)));
                if let Some(focus) = &focus {
                    track = track.track_focus(focus);
                }
                for (side, said) in [(false, one.kept.clone()), (true, one.gone.clone())] {
                    let on = from_gone == side;
                    let part = segment(look, SharedString::from(format!("merge-value-{}-{}", one.key, if side { "gone" } else { "kept" })), said.clone().into(), if on { Pick::Set } else { Pick::No }, busy)
                        .aria_toggled(if on { gpui::Toggled::True } else { gpui::Toggled::False })
                        .when(!busy && !on, |part| {
                            part.on_click(cx.listener(move |this, _, _, cx| this.choose_value(n, side, cx)))
                        });
                    track = track.child(labelled(part, Role::RadioButton, said));
                }
                let track = match &focus {
                    Some(focus) => ringed(look, track, focus, window),
                    None => track,
                };
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(look.length(token::SPACE_3))
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_SM))
                    .child(div().min_w_0().truncate().child(one.name))
                    .child(labelled(track, Role::RadioGroup, one.name))
            });
            div()
                .flex()
                .flex_col()
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(escalated)
                .overflow_hidden()
                .child(
                    div()
                        .px(look.length(token::SPACE_3))
                        .py(look.length(token::SPACE_1))
                        .bg(look.color(token::STATUS_ESCALATED_BG))
                        .text_size(look.length(token::TEXT_2XS))
                        .line_height(look.length(token::LEADING_2XS))
                        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                        .text_color(escalated)
                        .child("VALUES THAT DIFFER"),
                )
                .children(rows)
        });

        let held = facts.held.clone().map(|said| {
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_SM))
                .text_color(muted)
                .child(svg().flex_none().path(icon_path("circle-dot")).size(glyph).text_color(muted))
                .child(SharedString::from(said))
        });

        let share = facts.needs_share.then(|| {
            let off = self.scope.saving.is_some() || busy;
            let switch = div()
                .id("merge-share")
                .flex_none()
                .rounded_full()
                .track_focus(&self.scope.merge_share)
                .child(switch_track(look, false, off))
                .when(!off, |switch| switch.cursor_pointer().on_click(cx.listener(|this, _, window, cx| this.share_kept(window, cx))));
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(look.length(token::SPACE_3))
                .p(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(escalated)
                .bg(look.color(token::STATUS_ESCALATED_BG))
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(div().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(Key::Shared.title()))
                        .child(small(look, facts.share_said.clone(), muted)),
                )
                .child(labelled(ringed(look, switch, &self.scope.merge_share, window), Role::Switch, Key::Shared.title()))
        });

        let older = facts.older.clone().map(|why| labelled(refusal(look, why.clone()).id("project-merge-older"), Role::Alert, why));
        let refused = asking.refused.clone().map(|why| labelled(refusal(look, why.clone()).id("project-merge-refused"), Role::Alert, why));
        let button = |id: &'static str, label: &'static str, primary: bool, off: bool| {
            let base = div()
                .id(id)
                .flex_none()
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_3))
                .flex()
                .items_center()
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .when(off, |button| button.opacity(0.5))
                .when(!off, |button| button.cursor_pointer());
            if primary {
                base.border_color(look.color(token::ACCENT)).bg(look.color(token::ACCENT)).text_color(look.color(token::TEXT_ON_ACCENT)).when(!off, |button| button.hover(|style| style.opacity(0.9)))
            } else {
                base.border_color(look.color(token::BORDER_DEFAULT)).bg(look.color(token::BG_RAISED)).text_color(ink).when(!off, |button| button.hover(|style| style.bg(look.color(token::BG_HOVER))))
            }
            .child(label)
        };
        let blocked = facts.needs_share || facts.older.is_some();
        let merge_off = blocked || busy;
        let why = facts.older.clone().unwrap_or_else(|| format!("Share {} with your other Macs first", facts.kept.name));
        let merge = ringed(look, button("project-merge-confirm", "Merge", true, merge_off), &self.scope.merge_confirm, window)
            .track_focus(&self.scope.merge_confirm)
            .tooltip(tip(if blocked { why } else { "⌘↩".to_string() }))
            .when(!merge_off, |button| button.on_click(cx.listener(|this, _, window, cx| this.confirm_merge(window, cx))));
        let cancel = ringed(look, button("project-merge-cancel", "Cancel", false, busy), &self.scope.cancel_stop, window)
            .track_focus(&self.scope.cancel_stop)
            .tooltip(tip("Esc"))
            .on_click(cx.listener(|this, _, window, cx| this.cancel_merge(window, cx)));

        let said = SharedString::from(facts.title.clone());
        let card = div()
            .id("project-merge")
            .occlude()
            .w_full()
            .max_w(look.length(token::W_DIALOG_WIDE))
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_4))
            .p(look.length(token::PAD_CARD))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .text_color(ink)
            // A press inside the card is not a press on the scrim.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(div().text_size(look.length(token::TEXT_BASE)).line_height(look.length(token::LEADING_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(said.clone()))
            .child(tiles)
            .child(div().text_size(look.length(token::TEXT_SM)).text_color(muted).child(SharedString::from(facts.sentence.clone())))
            .children(differ)
            .children(held)
            .children(share)
            .children(older)
            .children(refused)
            .child(div().flex().justify_end().gap(look.length(token::SPACE_2)).child(labelled(cancel, Role::Button, "Cancel")).child(labelled(merge, Role::Button, "Merge")));
        Some(
            div()
                .id("project-merge-scrim")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .p(look.length(token::SPACE_6))
                .bg(look.color(token::SCRIM))
                .occlude()
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| {
                    this.cancel_merge(window, cx);
                    cx.stop_propagation();
                }))
                .child(labelled(card, Role::Dialog, said))
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Entity, TestAppContext, VisualTestContext};
    use ipc::{ManifestId, ProjectHolder};

    use super::super::mock::MockFleet;
    use super::super::wire::Wire;
    use super::*;

    fn task(n: usize, held: bool) -> ProjectRow {
        ProjectRow::Task {
            id: format!("t{n}"),
            manifest_id: ManifestId::carried("web"),
            title: format!("Task {n}"),
            notes: String::new(),
            state: if held { ProjectTaskState::Held } else { ProjectTaskState::Open },
            held_by: held.then(|| ProjectHolder { session_id: "job".into(), title: None }),
            parent: None,
            issues: Vec::new(),
            pulls: Vec::new(),
        }
    }

    fn member(name: &str) -> ProjectMember {
        ProjectMember { manifest_id: ManifestId::carried(name), folder: None }
    }

    fn pair() -> (Project, Project) {
        let own = Project { id: "own".into(), name: "Pricing".into(), rows: (1..=2).map(|n| task(n, false)).collect(), members: vec![member("web")], ..Project::default() };
        let copy = Project {
            id: "copy".into(),
            name: "Pricing".into(),
            rows: (3..=9).map(|n| task(n, n == 9)).collect(),
            members: vec![member("web"), member("pricing-web")],
            mac: Some("mac-air".into()),
            ..Project::default()
        };
        (own, copy)
    }

    #[test]
    fn the_sentence_is_built_from_what_comes_over_and_leaves_an_empty_clause_out() {
        let (own, copy) = pair();
        let jobs = |id: &str| if id == "copy" { 1 } else { 3 };
        let said = facts(&own, &copy, &[], &jobs);
        assert_eq!(said.title, "Merge the two Pricing projects");
        assert_eq!(said.sentence, "mac-air's Pricing goes. Its 7 tasks, its Job and the repository pricing-web come into this Mac's Pricing.");
        assert_eq!((said.kept.counts().as_str(), said.gone.counts().as_str()), ("2 tasks · 3 Jobs", "7 tasks · 1 Job"));
        let none = facts(&own, &copy, &[], &|_| 0);
        assert_eq!(none.sentence, "mac-air's Pricing goes. Its 7 tasks and the repository pricing-web come into this Mac's Pricing.", "no Job, no clause");
        let swapped = facts(&copy, &own, &[], &jobs);
        assert_eq!(swapped.sentence, "This Mac's Pricing goes. Its 2 tasks and its 3 Jobs come into mac-air's Pricing.", "no repository the kept one lacks");
        let empty = Project { id: "e".into(), name: "Pricing".into(), ..Project::default() };
        assert_eq!(facts(&own, &empty, &[], &|_| 0).sentence, "This Mac's Pricing goes.", "nothing comes over");
        let one = Project { rows: vec![task(1, false)], ..empty };
        assert_eq!(facts(&own, &one, &[], &|_| 0).sentence, "This Mac's Pricing goes. Its 1 task comes into this Mac's Pricing.");
    }

    #[test]
    fn a_held_task_is_said_only_where_the_gone_side_holds_one() {
        let (own, copy) = pair();
        assert_eq!(facts(&own, &copy, &[], &|_| 1).held.as_deref(), Some("1 task its Job on mac-air holds comes over open."));
        assert_eq!(facts(&copy, &own, &[], &|_| 1).held, None);
    }

    #[test]
    fn only_the_keys_both_set_differently_are_rows_and_each_reads_as_a_word() {
        let (mut own, mut copy) = pair();
        own.values = Some(ValuesSet { delivery: Some("auto_land".into()), auto_merge: Some("always".into()), tracker: None, ..ValuesSet::default() });
        copy.values = Some(ValuesSet { delivery: Some("open_pr".into()), auto_merge: Some("always".into()), agent_starts: Some("ask_me".into()), ..ValuesSet::default() });
        let said = facts(&own, &copy, &[], &|_| 0);
        assert_eq!(said.conflicts.iter().map(|one| (one.key, one.name)).collect::<Vec<_>>(), [("delivery", "How work lands")], "a key one leaves out, or both set alike, is no row");
        assert_eq!(said.conflicts[0].kept, "Auto land");
        own.values = None;
        assert!(facts(&own, &copy, &[], &|_| 0).conflicts.is_empty(), "no differing value, no section");
    }

    #[test]
    fn the_kept_project_needs_sharing_only_while_it_is_this_macs_own_and_not_shared() {
        let (mut own, copy) = pair();
        assert!(facts(&own, &copy, &[], &|_| 0).needs_share);
        assert!(!facts(&copy, &own, &[], &|_| 0).needs_share, "a copy kept is the other Mac's to share");
        own.shared_with_macs = true;
        assert!(!facts(&own, &copy, &[], &|_| 0).needs_share);
    }

    fn opened(cx: &mut TestAppContext) -> (Entity<SettingsOverlay>, &mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        });
        let (overlay, cx) = cx.add_window_view(|window, cx| {
            let mut mock = MockFleet::new();
            mock.mergeable_projects();
            SettingsOverlay::new(Wire::Mock(Box::new(mock)), window, cx)
        });
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.show(None, false, window, cx)));
        cx.run_until_parked();
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.edit(Some(("pricing".into(), "Pricing".into())), window, cx)));
        cx.run_until_parked();
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.ask_merge("pricing", window, cx)));
        (overlay, cx)
    }

    fn card(overlay: &Entity<SettingsOverlay>, cx: &mut VisualTestContext) -> Facts {
        overlay.read_with(cx, |this, cx| this.merge_facts(cx).expect("the card"))
    }

    #[gpui::test]
    fn pressing_merge_in_the_picker_opens_the_card(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.cancel_merge(window, cx)));
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.open_picker(false, window, cx)));
        cx.run_until_parked();
        for _ in 0..3 {
            cx.run_until_parked();
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }
        let at = cx.update(|window, cx| super::super::view::bounds_of(window, cx, Role::MenuItem, "Merge with Pricing on mac-air")).expect("the submenu is drawn");
        cx.simulate_click(at.center(), gpui::Modifiers::none());
        cx.run_until_parked();
        overlay.read_with(cx, |this, _| assert!(this.scope.merging.is_some(), "the card opens"));
    }

    #[gpui::test]
    fn the_arrows_carry_the_submenu_to_another_project_and_its_merge_acts_on_that_one(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.cancel_merge(window, cx)));
        // Editing something else: the submenu opens beside it, and the arrows move it to Pricing.
        let (other, at) = overlay.read_with(cx, |this, _| {
            let all = &this.scope.projects.as_ref().expect("projects").projects;
            (all.iter().find(|one| one.id != "pricing" && one.id != "pricing-air" && one.mac.is_none()).map(|one| (one.id.clone(), one.name.clone())).expect("another project"), all.iter().position(|one| one.id == "pricing").expect("Pricing"))
        });
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.edit(Some(other), window, cx)));
        cx.run_until_parked();
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.open_picker(false, window, cx)));
        redraw(cx);
        // Every project first, then each project down to Pricing.
        for _ in 0..at + 2 {
            cx.simulate_keystrokes("down");
        }
        redraw(cx);
        assert!(cx.update(|window, cx| super::super::view::bounds_of(window, cx, Role::MenuItem, "Merge with Pricing on mac-air")).is_some(), "the submenu stands beside Pricing");
        cx.simulate_keystrokes("right enter");
        cx.run_until_parked();
        overlay.read_with(cx, |this, _| {
            let asking = this.scope.merging.as_ref().expect("the card opens");
            assert_eq!((asking.kept.as_str(), asking.gone.as_str()), ("pricing", "pricing-air"), "the merge is about the row's project");
        });
    }

    fn redraw(cx: &mut VisualTestContext) {
        for _ in 0..3 {
            cx.run_until_parked();
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }
    }

    #[test]
    fn a_mac_on_an_older_build_keeps_merge_off_and_says_why() {
        let (own, copy) = pair();
        let air = |link| ipc::Mac { id: "mac-air".into(), name: "Air".into(), link, ..ipc::Mac::default() };
        assert_eq!(facts(&own, &copy, &[air(Some(ipc::MacLink::OlderBuild))], &|_| 0).older.as_deref(), Some("Air runs an older build. Merge is off until it updates"));
        assert_eq!(facts(&own, &copy, &[air(Some(ipc::MacLink::Linked))], &|_| 0).older, None);
        assert_eq!(facts(&own, &copy, &[], &|_| 0).older, None, "a Mac the list does not hold is not marked");
        assert_eq!(facts(&own, &copy, &[], &|_| 0).share_said, "Merge is off until Pricing is shared with your other Macs.");
    }

    #[gpui::test]
    fn a_long_mac_name_is_cut_short_and_the_project_name_keeps_its_room(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.cancel_merge(window, cx)));
        let long = "Nick-Meles-MacBook-Pro---Kiavi";
        overlay.update(cx, |this, _| {
            for one in this.scope.projects.iter_mut().flat_map(|all| all.projects.iter_mut()).filter(|one| one.mac.is_some()) {
                one.mac = Some(long.into());
            }
        });
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.open_picker(false, window, cx)));
        redraw(cx);
        let (menu, chip) = cx.update(|window, cx| (super::super::view::bounds_of(window, cx, Role::Menu, "Editing"), super::super::view::bounds_of(window, cx, Role::Image, long)));
        let (menu, chip) = (menu.expect("the menu"), chip.expect("the chip"));
        let cap = cx.update(|_, cx| crate::look::ActiveLook::look(cx).length(token::SPACE_12) * 2.25);
        assert!(chip.size.width <= cap, "the chip stops at its width: {:?}", chip.size.width);
        assert!(chip.right() <= menu.right(), "the chip stays in the menu");
        assert!(chip.left() - menu.left() > gpui::px(120.), "the name has room before the chip: {:?}", chip.left() - menu.left());
    }

    #[gpui::test]
    fn the_card_keeps_this_macs_own_and_swap_makes_the_copy_the_kept_one(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        let said = card(&overlay, cx);
        assert_eq!((said.kept.id.as_str(), said.gone.id.as_str()), ("pricing", "pricing-air"));
        assert_eq!(said.sentence, "mac-air's Pricing goes. Its 7 tasks, its Job and the repository pricing-web come into this Mac's Pricing.");
        assert_eq!(said.conflicts.iter().map(|one| one.key).collect::<Vec<_>>(), ["auto_merge", "delivery"]);
        overlay.update(cx, |this, _| this.scope.merging.as_mut().expect("asked").from_gone.push("delivery".into()));
        cx.update(|_, cx| overlay.update(cx, |this, cx| this.swap_merge(cx)));
        let swapped = card(&overlay, cx);
        assert_eq!((swapped.kept.id.as_str(), swapped.gone.id.as_str()), ("pricing-air", "pricing"));
        overlay.read_with(cx, |this, _| assert!(this.scope.merging.as_ref().is_some_and(|asking| asking.from_gone.is_empty()), "a choice was about the other side"));
    }

    #[gpui::test]
    fn merge_is_refused_here_until_the_kept_project_is_shared(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        assert!(card(&overlay, cx).needs_share);
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.confirm_merge(window, cx)));
        cx.run_until_parked();
        overlay.read_with(cx, |this, _| {
            let asking = this.scope.merging.as_ref().expect("the card stays");
            assert!(!asking.sending, "nothing was sent");
            assert_eq!(this.scope.projects.as_ref().map(|all| all.projects.len()), Some(5), "both Pricings are still listed");
        });
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.share_kept(window, cx)));
        cx.run_until_parked();
        assert!(!card(&overlay, cx).needs_share, "the switch shared it");
    }

    #[gpui::test]
    fn merge_sends_the_keys_he_took_from_the_gone_side_and_the_picker_keeps_the_kept_project(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.share_kept(window, cx)));
        cx.run_until_parked();
        // The second differing row, delivery, takes the copy's word; Merging keeps this Mac's.
        cx.update(|_, cx| overlay.update(cx, |this, cx| this.choose_value(1, true, cx)));
        overlay.read_with(cx, |this, _| assert_eq!(this.scope.merging.as_ref().map(|asking| asking.from_gone.clone()), Some(vec!["delivery".to_string()])));
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.confirm_merge(window, cx)));
        cx.run_until_parked();
        overlay.read_with(cx, |this, _| {
            assert!(this.scope.merging.is_none(), "the card closes");
            assert_eq!(this.scope.project, Some(("pricing".into(), "Pricing".into())), "the picker keeps the kept project");
            let all = &this.scope.projects.as_ref().expect("Fleet's answer").projects;
            assert_eq!(all.iter().filter(|one| one.name == "Pricing").count(), 1);
            let kept = all.iter().find(|one| one.id == "pricing").expect("the kept project");
            let values = kept.values.as_ref().expect("values");
            assert_eq!(values.delivery.as_deref(), Some("open_pr"), "the gone side's word, as he chose");
            assert_eq!(values.auto_merge.as_deref(), Some("always"), "the kept side's, as he left it");
            assert_eq!(values.agent_starts.as_deref(), Some("ask_me"), "a key only the gone side set comes over");
            assert!(kept.rows.iter().all(|row| !matches!(row, ProjectRow::Task { state: ProjectTaskState::Held, .. })), "a held task comes over open");
        });
    }

    #[gpui::test]
    fn cancel_changes_nothing_and_a_refusal_is_said_in_fleets_words(cx: &mut TestAppContext) {
        let (overlay, cx) = opened(cx);
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.cancel_merge(window, cx)));
        overlay.read_with(cx, |this, _| {
            assert!(this.scope.merging.is_none());
            assert_eq!(this.scope.projects.as_ref().map(|all| all.projects.len()), Some(5));
        });
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.ask_merge("pricing", window, cx)));
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.share_kept(window, cx)));
        cx.run_until_parked();
        overlay.update(cx, |this, _| {
            let asking = this.scope.merging.as_mut().expect("asked");
            asking.gone = asking.kept.clone();
        });
        cx.update(|window, cx| overlay.update(cx, |this, cx| this.confirm_merge(window, cx)));
        cx.run_until_parked();
        overlay.read_with(cx, |this, _| {
            let asking = this.scope.merging.as_ref().expect("the card stays");
            assert_eq!(asking.refused.as_deref(), Some("A merge takes one project of this Mac's and one copy of another Mac's"));
            assert!(!asking.sending);
        });
    }
}
