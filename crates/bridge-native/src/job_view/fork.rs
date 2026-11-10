//! Fork from selection (A8): the bar over selected words with Reply, Fork from here and Teach, and
//! the same three as a menu. Words are selected in a person's, an agent's or a Fleet message.

use gpui::{
    anchored, canvas, deferred, div, prelude::*, AnyElement, Anchor, Bounds, Context, Div, KeyDownEvent, MouseButton, MouseDownEvent, MouseUpEvent, Pixels,
    Point, Role, SharedString, Stateful, Window,
};
use gpui_base::TextSelection;
use ipc::{ForkFrom, StartSession};

use super::menus::Teach;
use super::rows::{glyph, tip, Size};
use super::JobView;
use crate::dropdown::Entry;
use crate::labels::labelled;
use crate::look::{token, ActiveLook};

/// Words selected in one block, and where the bar for them stands.
pub(super) struct Picked {
    row: String,
    words: String,
    /// Where the first line of the selection starts: the bar stands over it.
    at: Point<Pixels>,
    /// The box the press and the release span: a right press inside it is on the selected words.
    span: Bounds<Pixels>,
}

#[derive(Default)]
pub(super) struct Fork {
    down: Option<Point<Pixels>>,
    shown: Option<Picked>,
}

/// A draft's block id is its card key, which starts `draft:`.
fn draft_row(row: &str) -> Option<&str> {
    row.starts_with("draft:").then_some(row)
}

/// Words quoted for a note: each line after `>`, then a blank line.
fn quoted(words: &str) -> String {
    let lines: Vec<String> = words.replace('\r', "").trim().lines().map(|line| if line.trim().is_empty() { ">".to_string() } else { format!("> {}", line.trim()) }).collect();
    format!("{}\n\n", lines.join("\n"))
}

fn spanning(a: Point<Pixels>, b: Point<Pixels>) -> Bounds<Pixels> {
    let origin = gpui::point(a.x.min(b.x), a.y.min(b.y));
    Bounds { origin, size: gpui::size((a.x - b.x).abs().max(gpui::px(1.)), (a.y - b.y).abs().max(gpui::px(1.))) }
}

impl JobView {
    /// A block of words a selection can lie in: it says where it is drawn, under the id of its row.
    pub(super) fn block(&self, page: &super::Page, row: &str, words: AnyElement, cx: &Context<Self>) -> AnyElement {
        let spot = page.keys.spot(&format!("block:{row}"), cx);
        let edges = spot.bounds.clone();
        labelled(
            div()
                .id(SharedString::from(format!("words:{row}")))
                .relative()
                .w_full()
                .child(words)
                .child(canvas(move |bounds, _, _| edges.set(bounds), |_, _, _, _| {}).absolute().top_0().left_0().size_full()),
            Role::Group,
            format!("Words {row}"),
        )
        .into_any_element()
    }

    /// The thread's own presses and keys that start, show and end a selection's acts.
    pub(super) fn fork_hooks(&self, thread: Stateful<Div>, cx: &Context<Self>) -> Stateful<Div> {
        thread
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _, cx| {
                    this.fork.down = Some(event.position);
                    if this.fork.shown.take().is_some() {
                        cx.notify();
                    }
                }),
            )
            .on_mouse_up(MouseButton::Left, cx.listener(|this, event: &MouseUpEvent, window, cx| this.fork_released(event.position, window, cx)))
            .capture_any_mouse_down(cx.listener(|this, event: &MouseDownEvent, window, cx| {
                if event.button != MouseButton::Right {
                    return;
                }
                let Some(picked) = this.fork.shown.as_ref() else { return };
                if picked.span.contains(&event.position) {
                    cx.stop_propagation();
                    this.selection_menu(event.position, false, window, cx);
                }
            }))
            // A draft card has a menu key of its own; words picked in it take it first.
            .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                let key = event.keystroke.key.as_str();
                let picked_in_draft = this.fork.shown.as_ref().is_some_and(|picked| draft_row(&picked.row).is_some());
                if this.pop.is_none() && picked_in_draft && (key == "menu" || (key == "f10" && event.keystroke.modifiers.shift)) {
                    cx.stop_propagation();
                    let at = this.fork.shown.as_ref().map(|picked| picked.at).unwrap_or_default();
                    this.selection_menu(at, true, window, cx);
                }
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if this.fork.shown.is_none() || this.pop.is_some() {
                    return;
                }
                match event.keystroke.key.as_str() {
                    "escape" => {
                        cx.stop_propagation();
                        this.fork_done(window, cx);
                    }
                    "menu" => {}
                    "f10" if event.keystroke.modifiers.shift => {
                        cx.stop_propagation();
                        let at = this.fork.shown.as_ref().map(|picked| picked.at).unwrap_or_default();
                        this.selection_menu(at, true, window, cx);
                    }
                    _ => {}
                }
            }))
    }

    fn fork_released(&mut self, up: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(down) = self.fork.down.take() else { return };
        if !TextSelection::has_selection(window, cx) {
            return;
        }
        let words = TextSelection::selected_text(window, cx);
        if words.trim().is_empty() {
            return;
        }
        // Copy on select: the same words go to the clipboard at the release.
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(words.clone()));
        let Some(row) = self.shown.as_ref().and_then(|job| self.pages.get(job)).and_then(|page| page.keys.block_at(down, up)) else { return };
        let first = if down.y <= up.y { down } else { up };
        self.fork.shown = Some(Picked { row, words, at: first, span: spanning(down, up) });
        cx.notify();
    }

    /// The selection's acts are over: the selection and the bar go.
    fn fork_done(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.fork.shown = None;
        TextSelection::clear(window, cx);
        cx.notify();
    }

    /// The menu of the selected words: a draft's own, or the thread's Teach, Reply and Fork.
    fn selection_menu(&mut self, at: Point<Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        match self.fork.shown.as_ref().filter(|picked| picked.row.starts_with("draft:")).map(|picked| (picked.row.clone(), picked.words.clone())) {
            Some((row, words)) => self.draft_menu(at, from_keys, row, words, window, cx),
            None => self.fork_menu(at, from_keys, window, cx),
        }
    }

    /// Edit and Add note on words selected in an open draft: each offered where the draft has it.
    fn draft_menu(&mut self, at: Point<Pixels>, from_keys: bool, row: String, words: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(key) = draft_row(&row) else { return };
        let Some(card) = self.shown.as_ref().and_then(|job| self.pages.get(job)).and_then(|page| page.keys.get(key)) else { return };
        let mut entries = Vec::new();
        if card.change.is_some() {
            entries.push(Entry::Act { id: "edit", label: "Edit".into(), icon: Some("pencil"), danger: false });
        }
        if card.send_back.is_some() {
            entries.push(Entry::Act { id: "note", label: "Add note".into(), icon: Some("message-square-plus"), danger: false });
        }
        if entries.is_empty() {
            return;
        }
        let back = self.focus.clone();
        self.open_pop("Selection", entries, at, from_keys, Some(back), move |this, id, window, cx| this.draft_pick(&row, &words, id, window, cx), window, cx);
    }

    fn draft_pick(&mut self, row: &str, words: &str, choice: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        self.fork.shown = None;
        TextSelection::clear(window, cx);
        let Some(key) = draft_row(row) else { return };
        let Some(job) = self.shown.clone() else { return };
        let card = self.pages.get(&job).and_then(|page| page.keys.get(key));
        let run = card.and_then(|card| if choice == "edit" { card.change } else { card.send_back }).map(|(_, run)| run);
        let Some(run) = run else { return };
        run(self, window, cx);
        if choice == "note" {
            // The words come into the note as a quote, as they do in a reply.
            let area = self.pages.get(&job).and_then(|page| page.cards.boxes.get(&format!("{key}:{:?}", super::acts::Mode::Note)).cloned());
            if let Some(area) = area {
                let quote = quoted(words);
                area.update(cx, |area, cx| area.set_value(quote, window, cx));
            }
        }
        cx.notify();
    }

    fn fork_menu(&mut self, at: Point<Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let live = self.shown.as_ref().is_some_and(|job| !self.closed(job, cx));
        let mut entries = vec![Entry::Act { id: "teach", label: "Teach".into(), icon: Some("graduation-cap"), danger: false }];
        if live {
            entries.push(Entry::Act { id: "reply", label: "Reply".into(), icon: Some("reply"), danger: false });
        }
        entries.push(Entry::Act { id: "fork", label: "Fork from here".into(), icon: Some("git-fork"), danger: false });
        let back = self.focus.clone();
        self.open_pop("Selection", entries, at, from_keys, Some(back), |this, id, window, cx| this.fork_act(id, window, cx), window, cx);
    }

    /// One of the three acts on the selected words.
    fn fork_act(&mut self, act: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(picked) = self.fork.shown.take() else { return };
        TextSelection::clear(window, cx);
        match act {
            "reply" => self.quote(&picked.words, window, cx),
            "teach" => self.teach_open(Teach { kind: "words", text: picked.words.clone(), at: None, ids: Vec::new() }, window, cx),
            "fork" => self.fork_from(picked.row, picked.words, window, cx),
            _ => {}
        }
        cx.notify();
    }

    /// A new Job forked at the message, its box seeded with the quote, and that Job opened.
    fn fork_from(&mut self, row: String, words: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let Some(manifest) = self.record(&job, cx).and_then(|record| record.manifest_id.clone()) else {
            if let Some(page) = self.pages.get_mut(&job) {
                page.cards.said.insert("fork".into(), "This Job has no repository to fork in".into());
            }
            cx.notify();
            return;
        };
        let start = StartSession {
            manifest_id: manifest,
            title: None,
            model: None,
            effort: None,
            mode: None,
            fork: Some(ForkFrom { session_id: ipc::SessionId::carried(job.to_string()), at: Some(row) }),
            root: None,
            request: None,
            after: None,
            held: false,
            project_rows: Vec::new(),
            on_your_words: None,
        };
        let body = ipc::encode(&start).map_err(|why| format!("{why:?}"));
        let Some(fleet) = self.fleet.clone() else { return };
        let Some(port) = fleet.read(cx).held().port() else { return };
        let quoted: String = words.trim().lines().map(|line| if line.is_empty() { ">".to_string() } else { format!("> {line}") }).collect::<Vec<_>>().join("\n");
        cx.spawn_in(window, async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let body = body?;
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::post(port, "/sessions/start", body))?;
                    if (200..300).contains(&status) {
                        return ipc::decode::<ipc::SessionRecord>("a forked Job", &bytes).map(|record| record.id.as_str().to_string()).map_err(|why| format!("{why:?}"));
                    }
                    Err(ipc::decode::<ipc::WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message))
                })
                .await;
            let _ = this.update(cx, |this, cx| match answer {
                Ok(id) => {
                    if this.drafts.set(&id, &format!("{quoted}\n\n")) {
                        this.keep_soon(cx);
                    }
                    this.open_job(id, cx);
                }
                Err(why) => {
                    if let Some(page) = this.shown.clone().and_then(|job| this.pages.get_mut(&job)) {
                        page.cards.said.insert("fork".into(), why.into());
                    }
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// The bar over the selected words: Reply, Fork from here, Teach.
    pub(super) fn render_fork_bar(&self, cx: &Context<Self>) -> Option<AnyElement> {
        let picked = self.fork.shown.as_ref().filter(|picked| !picked.row.starts_with("draft:"))?;
        let look = cx.look();
        let live = self.shown.as_ref().is_some_and(|job| !self.closed(job, cx));
        let act = |id: &'static str, said: &'static str, icon: &'static str| {
            labelled(
                div()
                    .id(SharedString::from(format!("selection-{id}")))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(look.length(token::H_CONTROL_SM))
                    .rounded(look.length(token::RADIUS_MD))
                    .cursor_pointer()
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .tooltip(tip(said))
                    .child(glyph(look, icon, Size::Mark, look.color(token::FG_MUTED)))
                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                        window.prevent_default();
                        cx.stop_propagation();
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.fork_act(id, window, cx);
                    })),
                Role::Button,
                said,
            )
        };
        let bar = labelled(
            div()
                .id("selection-bar")
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .p(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .bg(look.color(token::BG_OVERLAY))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                .occlude()
                .children(live.then(|| act("reply", "Reply", "reply")))
                .child(act("fork", "Fork from here", "git-fork"))
                .child(act("teach", "Teach", "graduation-cap")),
            Role::Toolbar,
            "Selection",
        );
        let above = gpui::point(picked.at.x + picked.span.size.width.min(gpui::px(96.)) / 2., picked.at.y - look.length(token::SPACE_2));
        Some(deferred(anchored().position(above).anchor(Anchor::BottomCenter).snap_to_window_with_margin(look.length(token::SPACE_2)).child(bar)).with_priority(1).into_any_element())
    }
}
