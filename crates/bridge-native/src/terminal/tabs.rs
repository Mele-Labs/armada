//! A Job's terminals as tabs of its panes: + Terminal opens a new shell in the pane pressed, and a shell
//! Fleet opened or brought forward gets its tab. Each shell keeps one pane for the app's life, so a tab
//! closed and opened again finds its screen as it was.

use std::collections::HashMap;
use std::rc::Rc;

use gpui::{App, AppContext, ClipboardItem, Entity, Focusable, SharedString, Window};
use ipc::TagKind;

use super::pane::{Phase, TerminalPane};
use super::shells::{self, Which};
use crate::job_view::{offer_tab, on_job_shown, JobView, TabAct, TabOffer, TabOffers, ViewPane, ViewTab};

/// The glyph of a terminal's tab and of + Terminal, as Electron's pane kinds draw a Terminal (`kinds.ts`).
const GLYPH: &str = "square-terminal";

/// Each shell's pane, by Job and terminal id.
#[derive(Default)]
struct Panes(HashMap<(String, String), Entity<TerminalPane>>);

impl gpui::Global for Panes {}

/// Each shell's name as Fleet last gave it: `Terminal 2`, or the person's.
#[derive(Default)]
struct Names(HashMap<(String, String), SharedString>);

impl gpui::Global for Names {}

fn name_of(job: &str, terminal: &str, cx: &App) -> SharedString {
    cx.try_global::<Names>().and_then(|names| names.0.get(&(job.to_string(), terminal.to_string())).cloned()).unwrap_or_else(|| unnamed(terminal))
}

fn tab_id(terminal: &str) -> String {
    format!("terminal:{terminal}")
}

/// Put Terminal in every Job's +, and an `Open <name>` row for each shell of the Job on screen that has no tab.
pub fn init(cx: &mut App) {
    offer_tab(TabOffer { id: "terminal", label: "Terminal".into(), glyph: GLYPH, open: Rc::new(|job, pane, window, cx| open_new(job, pane, window, cx)) }, cx);
    on_job_shown(|job, _, cx| untabbed(job, cx), cx);
}

const OPEN_PREFIX: &str = "terminal-open:";

/// An offer's id lives as long as the app: one per terminal id, `t1` to however many a Job has had.
#[derive(Default)]
struct OfferIds(HashMap<String, &'static str>);

impl gpui::Global for OfferIds {}

fn offer_id(terminal: &str, cx: &mut App) -> &'static str {
    let ids = cx.default_global::<OfferIds>();
    ids.0.entry(terminal.to_string()).or_insert_with(|| Box::leak(format!("{OPEN_PREFIX}{terminal}").into_boxed_str()))
}

/// Fleet's shells of Job `job` that no tab holds, offered as `Open <name>` (`PaneView.tsx:706-727`).
fn untabbed(job: Option<String>, cx: &mut App) {
    cx.default_global::<TabOffers>().0.retain(|one| !one.id.starts_with(OPEN_PREFIX));
    let (Some(job), Some(shells)) = (job, shells::of(cx)) else { return };
    let listing = shells.list(&job, cx);
    cx.spawn(async move |cx| {
        let Ok(listed) = listing.await else { return };
        let _ = cx.update(|cx| {
            let tabbed = JobView::live(cx).map(|view| view.read(cx).view_tab_ids(&job)).unwrap_or_default();
            for one in listed.terminals.into_iter().filter(|one| !tabbed.contains(&tab_id(&one.terminal))) {
                let name: SharedString = one.name.clone().into();
                cx.default_global::<Names>().0.insert((job.clone(), one.terminal.clone()), name.clone());
                let terminal = one.terminal.clone();
                let id = offer_id(&terminal, cx);
                let open = Rc::new(move |job: String, pane: ViewPane, window: &mut Window, cx: &mut App| {
                    cx.default_global::<TabOffers>().0.retain(|one| one.id != id);
                    open(&job, &terminal, Some(pane), true, window, cx);
                });
                offer_tab(TabOffer { id, label: format!("Open {name}").into(), glyph: GLYPH, open }, cx);
            }
        });
    })
    .detach();
}

/// Follow Fleet's news of a Job's shells: one opened gets its tab behind, one brought forward comes to the front.
pub fn follow(fleet: &Entity<crate::fleet::FleetConnection>, cx: &mut App) {
    cx.subscribe(fleet, |_, crate::fleet::Shells(news), cx| {
        for one in news.clone() {
            match one {
                crate::fleet::ShellNews::Opened(at) => open_in(at.session_id.as_str().to_string(), at.terminal, false, cx),
                crate::fleet::ShellNews::Shown(at) => open_in(at.session_id.as_str().to_string(), at.terminal, true, cx),
                // The pane reads the end on its own socket.
                crate::fleet::ShellNews::Exited(_) => {}
            }
        }
    })
    .detach();
}

/// + Terminal: Fleet opens a new shell at the size the last pane was drawn, and its tab comes to the front
/// of the pane whose + was pressed.
pub fn open_new(job: String, pane: ViewPane, window: &mut Window, cx: &mut App) {
    let Some(shells) = shells::of(cx) else { return };
    let opening = shells.open(&job, Which::New, super::start::size(cx), cx);
    window
        .spawn(cx, async move |cx| {
            let Ok(underway) = opening.await else { return };
            let Some(terminal) = underway.terminal else { return };
            let _ = cx.update(|window, cx| open(&job, &terminal, Some(pane), true, window, cx));
        })
        .detach();
}

/// Open Job `job`'s shell `terminal` in a tab of `place`, its pane kept from before where there was one.
/// No `place` leaves a tab where it stands, and a new one goes to the top pane. In front, it takes the keys.
pub fn open(job: &str, terminal: &str, place: Option<ViewPane>, front: bool, window: &mut Window, cx: &mut App) {
    let Some(pane) = shown(job, terminal, place, front, cx) else { return };
    if front {
        let focus = pane.focus_handle(cx);
        window.focus(&focus, cx);
    }
}

/// `open` with no window to hand the keys to: from Fleet's news.
fn open_in(job: String, terminal: String, front: bool, cx: &mut App) {
    shown(&job, &terminal, None, front, cx);
}

fn shown(job: &str, terminal: &str, place: Option<ViewPane>, front: bool, cx: &mut App) -> Option<Entity<TerminalPane>> {
    let view = JobView::live(cx)?;
    let pane = pane_of(job, terminal, cx)?;
    let tab = tab_of(job, terminal, &pane, cx);
    let place = placed(place, view.read(cx).view_tab_pane(job, &tab_id(terminal)));
    view.update(cx, |view, cx| view.open_view_tab_in(job, place, tab, front, cx));
    named(job.to_string(), terminal.to_string(), cx);
    Some(pane)
}

/// The pane a terminal's tab goes to: the one asked for, else where it stands, else the top pane.
fn placed(asked: Option<ViewPane>, standing: Option<ViewPane>) -> ViewPane {
    asked.or(standing).unwrap_or(ViewPane::Top)
}

fn pane_of(job: &str, terminal: &str, cx: &mut App) -> Option<Entity<TerminalPane>> {
    let key = (job.to_string(), terminal.to_string());
    if let Some(held) = cx.try_global::<Panes>().and_then(|panes| panes.0.get(&key)) {
        return Some(held.clone());
    }
    let shells = shells::of(cx)?;
    let pane = cx.new(|cx| TerminalPane::attached(job.to_string(), terminal.to_string(), shells, cx));
    cx.default_global::<Panes>().0.insert(key, pane.clone());
    Some(pane)
}

/// The tab's title until Fleet names it: `Terminal 2` for `t2`, as Fleet names it until a rename.
fn unnamed(terminal: &str) -> SharedString {
    format!("Terminal {}", terminal.trim_start_matches('t')).into()
}

/// Read the shell's name from Fleet, `Terminal 2` or the person's, and put it on its tab.
fn named(job: String, terminal: String, cx: &mut App) {
    let Some(shells) = shells::of(cx) else { return };
    let listing = shells.list(&job, cx);
    cx.spawn(async move |cx| {
        let Ok(listed) = listing.await else { return };
        let Some(one) = listed.terminals.into_iter().find(|one| one.terminal == terminal) else { return };
        let _ = cx.update(|cx| {
            let name = SharedString::from(one.name);
            cx.default_global::<Names>().0.insert((job.clone(), terminal.clone()), name.clone());
            let Some(view) = JobView::live(cx) else { return };
            view.update(cx, |view, cx| view.retitle_view_tab(&job, &tab_id(&terminal), name, cx));
        });
    })
    .detach();
}

/// Close the tab as × does: an ended shell goes at once, a running one once its program is asked about.
fn ask_close(job: String, terminal: String, window: &mut Window, cx: &mut App) {
    let ended = cx.try_global::<Panes>().and_then(|panes| panes.0.get(&(job.clone(), terminal.clone()))).is_none_or(|pane| *pane.read(cx).phase() == Phase::Ended);
    if ended {
        return close(job, terminal, window, cx);
    }
    let Some(shells) = shells::of(cx) else { return };
    let listing = shells.list(&job, cx);
    window.spawn(cx, async move |cx| {
        // Fleet unreachable is taken as running, as Electron's `busy` does (`job-shell.ts:284-296`).
        let running = listing.await.map_or(true, |listed| listed.terminals.iter().any(|one| one.terminal == terminal && one.running));
        let _ = cx.update(|window, cx| {
            if !running {
                return close(job, terminal, window, cx);
            }
            // A running program is asked about first, in Classic's words (`PaneView.tsx`'s End terminal).
            let Some(view) = JobView::live(cx) else { return };
            let name = name_of(&job, &terminal, cx);
            let (title, line) = (format!("End {name}"), format!("A program is running in {name}."));
            let on_yes = Rc::new(move |window: &mut Window, cx: &mut App| close(job.clone(), terminal.clone(), window, cx));
            view.update(cx, |view, cx| view.confirm(&title, &[line.as_str()], "End terminal", true, window, cx, on_yes));
        });
    })
    .detach();
}

/// Close the tab: the shell is ended by its `closed` hook.
fn close(job: String, terminal: String, window: &mut Window, cx: &mut App) {
    let Some(view) = JobView::live(cx) else { return };
    view.update(cx, |view, cx| view.close_view_tab(&job, &tab_id(&terminal), window, cx));
}

/// Gone from the strip: the shell ends, as Electron's tab ends it (`PaneView.tsx:289-304`), and its pane goes.
fn closed(job: &str, terminal: &str, cx: &mut App) {
    let key = (job.to_string(), terminal.to_string());
    let Some(pane) = cx.try_global::<Panes>().and_then(|panes| panes.0.get(&key)).cloned() else { return };
    pane.update(cx, |pane, cx| pane.end_shell(cx));
    cx.default_global::<Panes>().0.remove(&key);
}

fn tab_of(job: &str, terminal: &str, pane: &Entity<TerminalPane>, cx: &App) -> ViewTab {
    let act = |label: &'static str, danger: bool, act: Rc<dyn Fn(&mut Window, &mut App)>| TabAct { label: label.into(), danger, act };
    let (j, t) = (job.to_string(), terminal.to_string());
    let send = {
        let (j, t) = (j.clone(), t.clone());
        Rc::new(move |window: &mut Window, cx: &mut App| {
            let Some(view) = JobView::live(cx) else { return };
            let title = name_of(&j, &t, cx);
            view.update(cx, |view, cx| view.tag_in_draft(&j, TagKind::Terminal, &t, &title, window, cx));
        })
    };
    let copy = {
        let t = t.clone();
        Rc::new(move |_: &mut Window, cx: &mut App| cx.write_to_clipboard(ClipboardItem::new_string(t.clone())))
    };
    let rename = {
        let (j, t) = (j.clone(), t.clone());
        Rc::new(move |window: &mut Window, cx: &mut App| {
            let Some(view) = JobView::live(cx) else { return };
            view.update(cx, |view, cx| view.rename_view_tab(&j, &tab_id(&t), window, cx));
        })
    };
    // Kept with Enter: Fleet names the shell, and its answer is the tab's title; an empty name is Fleet's own again.
    let renamed = {
        let (j, t) = (j.clone(), t.clone());
        Rc::new(move |name: String, _: &mut Window, cx: &mut App| {
            let Some(shells) = shells::of(cx) else { return };
            let renaming = shells.rename(&j, &t, &name, cx);
            let (j, t) = (j.clone(), t.clone());
            cx.spawn(async move |cx| {
                let Ok(listed) = renaming.await else { return };
                let _ = cx.update(|cx| {
                    let name = SharedString::from(listed.name);
                    cx.default_global::<Names>().0.insert((j.clone(), t.clone()), name.clone());
                    let Some(view) = JobView::live(cx) else { return };
                    view.update(cx, |view, cx| view.retitle_view_tab(&j, &tab_id(&t), name, cx));
                });
            })
            .detach();
        })
    };
    let close_act = {
        let (j, t) = (j.clone(), t.clone());
        Rc::new(move |window: &mut Window, cx: &mut App| ask_close(j.clone(), t.clone(), window, cx))
    };
    let held = pane.clone();
    let veto = {
        let (j, t) = (j.clone(), t.clone());
        Rc::new(move |window: &mut Window, cx: &mut App| {
            if *held.read(cx).phase() == Phase::Ended {
                return true;
            }
            let (j, t) = (j.clone(), t.clone());
            // The veto runs inside the Job view's update: ask after it.
            window.defer(cx, move |window, cx| ask_close(j, t, window, cx));
            false
        })
    };
    let gone = Rc::new(move |_: &mut Window, cx: &mut App| closed(&j, &t, cx));
    let title = name_of(job, terminal, cx);
    ViewTab {
        id: tab_id(terminal),
        title,
        glyph: GLYPH,
        view: pane.clone().into(),
        menu: vec![act("Send to agent", false, send), act("Copy terminal ID", false, copy), act("Rename", false, rename), act("Close", true, close_act)],
        closing: Some(veto),
        closed: Some(gone),
        renamed: Some(renamed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_terminal_goes_to_the_pane_asked_for() {
        assert_eq!(placed(Some(ViewPane::Thread), Some(ViewPane::Top)), ViewPane::Thread);
        assert_eq!(placed(Some(ViewPane::Bottom), None), ViewPane::Bottom);
    }

    #[test]
    fn fleets_news_leaves_a_tab_where_it_stands_and_puts_a_new_one_on_top() {
        assert_eq!(placed(None, Some(ViewPane::Thread)), ViewPane::Thread);
        assert_eq!(placed(None, None), ViewPane::Top);
    }
}
