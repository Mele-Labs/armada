//! Pull requests › Offer review tools on a new pull request: a project's `review_on_new_pr`, one switch for each
//! review tool its repositories define. None on is off. A project's alone: Kit has no such key.
//!
//! The tools are read through the call the review page makes (`reviews::read_tools`), once for each repository
//! the project holds, off the main thread. A tool the project names that no repository defines still shows, on, so
//! it can be turned off.

use std::collections::BTreeMap;

use gpui::{div, prelude::*, AnyElement, Context, FocusHandle, Role, SharedString, Task, Toggled, Window};
use ipc::ValuesSet;

use super::overlay::SettingsOverlay;
use super::project::{Column, Key};
use super::view::{bone, refusal, ringed, small, switch_track};
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::reviews::model::ReviewTool;

/// What the page holds of the tools: which project they were read for, and the keys of each switch past the first.
#[derive(Default)]
pub struct ReviewOffer {
    project: Option<String>,
    /// `None` while read; `Err` is Fleet's own sentence.
    tools: Option<Result<Vec<ReviewTool>, String>>,
    /// A switch's key by tool id. The first switch takes the grid cell's.
    stops: BTreeMap<String, FocusHandle>,
    read: Option<Task<()>>,
}

/// One switch: the tool's id and name, and whether a repository defines it.
struct Shown {
    id: String,
    name: String,
    defined: bool,
}

/// The ids a project offers, as the word `write_cell` sends: Fleet's list is the words between commas.
pub fn word_of(ids: &[String]) -> String {
    ids.join(",")
}

/// The ids a word names, as `word_of` made it.
pub fn ids_of(word: &str) -> Vec<String> {
    word.split(',').map(str::trim).filter(|id| !id.is_empty()).map(str::to_string).collect()
}

/// The ids offered after `tool` is switched, in the list's order, with any a repository does not define kept as they were.
pub fn toggled(offered: &[String], tool: &str, list: &[ReviewTool]) -> Vec<String> {
    let on = !offered.iter().any(|id| id == tool);
    let mut next: Vec<String> = list.iter().filter(|one| if one.id == tool { on } else { offered.contains(&one.id) }).map(|one| one.id.clone()).collect();
    next.extend(offered.iter().filter(|id| id.as_str() != tool && !list.iter().any(|one| one.id == **id)).cloned());
    next
}

impl ReviewOffer {
    fn offered<'a>(&self, own: Option<&'a ValuesSet>) -> &'a [String] {
        own.map_or(&[], |own| own.review_on_new_pr.as_slice())
    }

    /// The switches, in the order drawn: each tool a repository defines, then each id named that none does.
    fn shown(&self, own: Option<&ValuesSet>) -> Vec<Shown> {
        let offered = self.offered(own);
        let Some(Ok(tools)) = &self.tools else { return Vec::new() };
        let mut shown: Vec<Shown> = tools.iter().map(|tool| Shown { id: tool.id.clone(), name: tool.name.clone(), defined: true }).collect();
        shown.extend(offered.iter().filter(|id| !tools.iter().any(|tool| tool.id == **id)).map(|id| Shown { id: id.clone(), name: id.clone(), defined: false }));
        shown
    }
}

impl SettingsOverlay {
    /// The block the project picked sets, as `GET /projects` last listed it.
    fn own_block(&self) -> Option<&ValuesSet> {
        let (id, _) = self.scope.project.as_ref()?;
        self.scope.projects.as_ref()?.projects.iter().find(|one| one.id == *id)?.values.as_ref()
    }

    /// Read the project picked's review tools, unless they are read for it already: one call for each repository it
    /// holds, the first definition of an id kept.
    pub(super) fn read_review_tools(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((id, _)) = self.scope.project.clone() else { return };
        if self.review_offer.project.as_ref() == Some(&id) && !matches!(self.review_offer.tools, Some(Err(_))) {
            return;
        }
        let Some(project) = self.scope.projects.as_ref().and_then(|all| all.projects.iter().find(|one| one.id == id)) else { return };
        let mut manifests: Vec<String> = Vec::new();
        for member in &project.members {
            let manifest = member.manifest_id.as_str().to_string();
            if !manifests.contains(&manifest) {
                manifests.push(manifest);
            }
        }
        self.review_offer.project = Some(id.clone());
        self.review_offer.tools = None;
        let reads: Vec<_> = manifests.iter().map(|manifest| self.wire.review_tools(manifest, cx)).collect();
        self.review_offer.read = Some(cx.spawn_in(window, async move |this, cx| {
            let mut tools: Vec<ReviewTool> = Vec::new();
            let mut refused: Option<String> = None;
            let mut read_any = reads.is_empty();
            for read in reads {
                match read.await {
                    Ok(found) => {
                        read_any = true;
                        for tool in found {
                            if !tools.iter().any(|one| one.id == tool.id) {
                                tools.push(tool);
                            }
                        }
                    }
                    Err(why) => refused = refused.or(Some(why.said)),
                }
            }
            let _ = this.update_in(cx, |this, _, cx| {
                // An answer for a project no longer picked is let go.
                if this.review_offer.project.as_ref() != Some(&id) {
                    return;
                }
                this.review_offer.tools = Some(match refused {
                    Some(why) if !read_any => Err(why),
                    _ => Ok(tools),
                });
                this.make_review_stops(cx);
                cx.notify();
            });
        }));
    }

    /// A key for each switch past the first, made when it is first listed.
    pub(super) fn make_review_stops(&mut self, cx: &mut Context<Self>) {
        let shown = self.review_offer.shown(self.own_block());
        for one in shown {
            self.review_offer.stops.entry(one.id).or_insert_with(|| cx.focus_handle().tab_stop(true));
        }
    }

    /// The key of the switch at `n` among those drawn: the first takes the grid cell's.
    fn review_stop(&self, n: usize, id: &str) -> Option<FocusHandle> {
        if n == 0 {
            self.scope.cells.get(&(Key::ReviewOnNewPr, Column::Own)).cloned()
        } else {
            self.review_offer.stops.get(id).cloned()
        }
    }

    /// The switches' keys in the order Tab reaches them.
    pub(super) fn review_stops(&self) -> Vec<FocusHandle> {
        self.review_offer.shown(self.own_block()).iter().enumerate().filter_map(|(n, one)| self.review_stop(n, &one.id)).collect()
    }

    /// Switch one tool and save the project's block whole, as every other value is saved.
    pub fn toggle_review_tool(&mut self, tool: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(Ok(list)) = self.review_offer.tools.clone() else { return };
        let next = toggled(self.review_offer.offered(self.own_block()), tool, &list);
        self.write_cell((Key::ReviewOnNewPr, Column::Own), Some(word_of(&next)), window, cx);
    }

    /// Space and Enter switch the tool that has the keys; ↑ and ↓ move among them, and past either end to the row
    /// beside. False where none of the switches has the keys.
    pub(super) fn review_tool_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let shown = self.review_offer.shown(self.own_block());
        let Some(at) = shown.iter().enumerate().position(|(n, one)| self.review_stop(n, &one.id).is_some_and(|handle| handle.is_focused(window))) else { return false };
        match key {
            "enter" | "space" => {
                let id = shown[at].id.clone();
                self.toggle_review_tool(&id, window, cx);
            }
            "up" | "down" => {
                let to = if key == "up" { at.checked_sub(1) } else { Some(at + 1).filter(|to| *to < shown.len()) };
                let handle = match to {
                    Some(to) => self.review_stop(to, &shown[to].id),
                    None => super::project::moved(&self.grid_of(cx), (Key::ReviewOnNewPr, Column::Own), key).and_then(|cell| self.scope.cells.get(&cell).cloned()),
                };
                if let Some(handle) = handle {
                    window.focus(&handle, cx);
                    crate::ui_sounds::moved();
                    cx.notify();
                }
            }
            _ => return false,
        }
        true
    }
}

/// The row's control: a switch for each tool, or bones while the tools are read, or why there are none.
pub fn cell(this: &SettingsOverlay, look: &Look, window: &Window, cx: &Context<SettingsOverlay>) -> AnyElement {
    let tall = look.length(token::H_CONTROL);
    let wide = look.length(token::SPACE_12) * 3.;
    let name = Key::ReviewOnNewPr.title();
    let Some(focus) = this.scope.cells.get(&(Key::ReviewOnNewPr, Column::Own)) else { return div().into_any_element() };
    let own = this.own_block();
    let shown = this.review_offer.shown(own);
    let message = |said: SharedString, failed: bool| {
        let line = if failed { refusal(look, said) } else { small(look, said, look.color(token::FG_SUBTLE)) };
        // The cell's key rests here, so the arrows pass through a row that has no switches.
        div().id("review-offer-none").track_focus(focus).flex().items_center().min_h(tall).child(line).into_any_element()
    };
    if this.scope.values.is_none() || this.review_offer.tools.is_none() {
        return div().id("review-offer-reading").track_focus(focus).child(bone(look, wide, tall)).into_any_element();
    }
    if let Some(Err(why)) = &this.review_offer.tools {
        return message(format!("The review tools could not be read. {why}").into(), true);
    }
    if shown.is_empty() {
        return message("This project's repositories define no review tools".into(), false);
    }
    let off = !this.live || this.scope.saving.is_some();
    let ticked = this.review_offer.offered(own);
    let rows = shown.into_iter().enumerate().filter_map(|(n, one)| {
        let handle = this.review_stop(n, &one.id)?;
        let on = ticked.iter().any(|id| *id == one.id);
        let id = one.id.clone();
        let pressed = handle.clone();
        let row = div()
            .id(SharedString::from(format!("review-tool-{}", one.id)))
            .track_focus(&handle)
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .h(look.length(token::H_CONTROL_LG))
            .px(look.length(token::SPACE_2))
            .mx(-look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(gpui::transparent_black())
            .aria_toggled(if on { Toggled::True } else { Toggled::False })
            .child(switch_track(look, on, off))
            .child(div().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(one.name.clone())))
            .when(!one.defined, |row| row.child(small(look, "No repository defines it", look.color(token::FG_SUBTLE))))
            .when(!off, |row| {
                row.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))).on_click(cx.listener(move |this, _, window, cx| {
                    window.focus(&pressed, cx);
                    this.toggle_review_tool(&id, window, cx);
                }))
            });
        Some(labelled(ringed(look, row, &handle, window), Role::Switch, one.name).into_any_element())
    });
    labelled(div().id("review-offer").flex().flex_col().children(rows), Role::Group, name).into_any_element()
}
