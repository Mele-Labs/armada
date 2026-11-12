//! A command an agent showed, run where its Job works: Run on a shell block of the thread, and a Needs you
//! run item. Each runs on a terminal of its own (`POST /sessions/run/start`), never one of the Job's shells,
//! drawn under what started it, as Electron's `InlineTerminal` and `HeldTerminal` draw it (`TerminalRun.tsx:212-235`).
//! A Job's thread holds one run, as its dock does: a block's Run stops the one going.

use std::collections::HashMap;
use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, App, Entity, Role, SharedString, Subscription};

use super::pane::{PaneEvent, Phase, TerminalPane};
use super::shells::{self, Which};
use crate::labels::labelled;
use crate::look::{token, Look, Token};
use crate::titlebar::parts::{glyph, tip};

/// Between a message's key and a block's place in it, in a block's name.
pub const AT: char = '\u{1f}';

struct Held {
    pane: Entity<TerminalPane>,
    /// Put away with Close: the block keeps its mark, which brings it back.
    shown: bool,
    _events: Subscription,
}

/// The thread's run of each Job, with the block that started it and where that block ends in its message.
struct Thread {
    block: String,
    end: usize,
    held: Held,
}

#[derive(Default)]
struct Runs {
    threads: HashMap<String, Thread>,
    /// A Needs you item's run, by Job and command (`waitingRunKey`).
    items: HashMap<(String, String), Held>,
}

impl gpui::Global for Runs {}

const SHELLS: &[&str] = &["sh", "bash", "zsh", "shell", "console"];

const COMMON: &[&str] = &[
    "cd", "ls", "git", "gh", "pnpm", "npm", "npx", "yarn", "cargo", "armada", "make", "docker", "kubectl", "brew", "node", "python", "python3", "pip", "curl", "cat",
    "echo", "grep", "rg", "rm", "mv", "cp", "mkdir", "touch", "chmod", "sudo", "tail", "head", "find", "sed", "awk", "xargs", "kill", "ps", "sh", "bash", "zsh",
    "source", "export",
];

/// A first word that opens a line of code, not of a command.
const CODE_WORDS: &[&str] =
    &["const", "let", "var", "function", "def", "class", "import", "from", "return", "fn", "pub", "use", "struct", "enum", "impl", "interface", "package", "public", "private", "type", "async", "await"];

/// The command a fenced block holds, or none where it is not a shell block (`commandOf`, `RunnableBlock.tsx:191`).
/// A `console` block loses its `$ ` prompts; a block with no language runs only where it reads as shell.
pub fn command_of(language: Option<&str>, source: &str) -> Option<String> {
    let body = source.strip_suffix('\n').unwrap_or(source);
    match language.filter(|language| !language.is_empty()) {
        None => reads_as_shell(body).then(|| strip_prompts(body)),
        Some("console") => Some(strip_prompts(body)),
        Some(language) if SHELLS.contains(&language) => Some(body.to_string()),
        Some(_) => None,
    }
}

fn strip_prompts(body: &str) -> String {
    body.split('\n').map(|line| line.strip_prefix("$ ").or_else(|| line.strip_prefix('$')).unwrap_or(line)).collect::<Vec<_>>().join("\n")
}

fn is_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

/// `^(\.{0,2}\/|~\/|\/|[\w.@-]+\/)[\w./@~-]+$`
fn is_path(word: &str) -> bool {
    let rest = ["../", "./", "~/", "/"].iter().find_map(|lead| word.strip_prefix(lead)).or_else(|| {
        let (head, rest) = word.split_once('/')?;
        (!head.is_empty() && head.chars().all(|ch| is_word_char(ch) || matches!(ch, '.' | '@' | '-'))).then_some(rest)
    });
    rest.is_some_and(|rest| !rest.is_empty() && rest.chars().all(|ch| is_word_char(ch) || matches!(ch, '.' | '/' | '@' | '~' | '-')))
}

/// Whether a block with no language reads as shell (`readsAsShell`, `RunnableBlock.tsx:159`). Conservative: a
/// doubt is no button.
pub fn reads_as_shell(source: &str) -> bool {
    let drawn = |ch: char| ('\u{2190}'..='\u{21ff}').contains(&ch) || ('\u{2500}'..='\u{257f}').contains(&ch) || ('\u{25a0}'..='\u{25ff}').contains(&ch);
    if source.chars().any(drawn) || source.contains("=>") || source.contains("->") || source.contains("<-") {
        return false;
    }
    let lines: Vec<&str> = source.split('\n').filter(|line| !line.trim().is_empty()).collect();
    if lines.is_empty() {
        return false;
    }
    let mut continued = false;
    for raw in lines {
        let line = raw.trim();
        let line = line.strip_prefix("$ ").unwrap_or(line);
        let was = continued;
        continued = line.ends_with('\\') || line.ends_with("&&") || line.ends_with("||") || line.ends_with('|');
        if line.starts_with('#') {
            continue;
        }
        if (line.ends_with(';') && !line.ends_with("\\;")) || line.ends_with(['{', '}', ',']) || line.starts_with(['{', '}', '[', ']', '"']) {
            return false;
        }
        if line.ends_with(['?', '!']) || line.strip_suffix('.').is_some_and(|head| head.ends_with(|ch: char| ch.is_ascii_alphanumeric() || ch == ')')) {
            return false;
        }
        if was {
            continue;
        }
        if line.split_once(' ').is_some_and(|(first, _)| !first.is_empty() && line[first.len()..].starts_with(" = ")) {
            return false;
        }
        let mut words = line.split_whitespace();
        let first = words.next().unwrap_or_default();
        let rest: Vec<&str> = words.collect();
        let assigns = first.split_once('=').is_some_and(|(name, _)| name.starts_with(|ch: char| ch.is_ascii_alphabetic() || ch == '_') && name.chars().all(is_word_char));
        if assigns {
            continue;
        }
        if is_path(first) {
            if !rest.is_empty() || first.starts_with("./") || first.starts_with("scripts/") || first.starts_with("bin/") || first.ends_with(".sh") {
                continue;
            }
            return false;
        }
        let worded = first.starts_with(|ch: char| ch.is_ascii_lowercase()) && first.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '_' | '+' | '-'));
        if !worded || CODE_WORDS.contains(&first) {
            return false;
        }
        if COMMON.contains(&first) {
            continue;
        }
        if !rest.iter().any(|word| word.starts_with('-') || word.contains(['/', '.', '~', '=', '$', '"', '\'', '*', '@'])) {
            return false;
        }
    }
    !continued
}

/// Redraw the Job view a run is drawn in: its height follows what the run has printed. After the current
/// update, as a run item is started from inside the Job view's own.
fn redraw(cx: &mut App) {
    cx.defer(|cx| {
        if let Some(view) = crate::job_view::JobView::live(cx) {
            view.update(cx, |_, cx| cx.notify());
        }
    });
}

/// A run's pane, opened where Job `job` works once it is drawn and measured.
fn pane(job: &str, command: &str, cx: &mut App) -> Option<Entity<TerminalPane>> {
    let shells = shells::of(cx)?;
    Some(cx.new(|cx| TerminalPane::new(job.to_string(), Which::Command(command.to_string()), shells, cx)))
}

/// Run on block `block` of Job `job`'s thread, which ends at `end` in its message: the run going is stopped, and this
/// one is drawn under the block.
pub fn run_block(job: &str, block: &str, end: usize, command: &str, cx: &mut App) {
    let Some(pane) = pane(job, command, cx) else { return };
    let job_key = job.to_string();
    let events = cx.subscribe(&pane, move |_, event: &PaneEvent, cx| {
        if matches!(event, PaneEvent::Closed) {
            if let Some(thread) = cx.default_global::<Runs>().threads.get_mut(&job_key) {
                thread.held.shown = false;
            }
        }
        redraw(cx);
    });
    let runs = cx.default_global::<Runs>();
    let before = runs.threads.insert(job.to_string(), Thread { block: block.to_string(), end, held: Held { pane, shown: true, _events: events } });
    if let Some(before) = before {
        before.held.pane.update(cx, |pane, cx| pane.end_shell(cx));
    }
    redraw(cx);
}

/// Where Job `job`'s thread run is drawn: its block, where that block ends, and its pane, while it is shown.
pub fn in_thread(job: &str, cx: &App) -> Option<(String, usize, Entity<TerminalPane>)> {
    let thread = cx.try_global::<Runs>()?.threads.get(job)?;
    thread.held.shown.then(|| (thread.block.clone(), thread.end, thread.held.pane.clone()))
}

/// The thread's run of Job `job`, where block `block` started it: shown, or put away.
fn of_block(job: &str, block: &str, cx: &App) -> Option<(bool, Entity<TerminalPane>)> {
    let thread = cx.try_global::<Runs>()?.threads.get(job).filter(|thread| thread.block == block)?;
    Some((thread.held.shown, thread.held.pane.clone()))
}

/// A run's mark: the terminal glyph while it runs, then passed, failed or stopped (`TerminalMark`).
fn mark(pane: &TerminalPane) -> (&'static str, Token, &'static str) {
    match (pane.phase(), pane.exit()) {
        (Phase::Ended, Some((Some(0), _))) => ("circle-check", token::RUN_PASSED, "Passed"),
        (Phase::Ended, Some((Some(_), _) | (None, false))) => ("circle-x", token::RUN_FAILED, "Failed"),
        (Phase::Ended, _) => ("circle-x", token::RUN_STOPPED, "Stopped"),
        _ => ("square-terminal", token::RUN_RUNNING, "Running"),
    }
}

/// A ghost icon button over a block's corner, shown while the pointer is on the words or it has the keys.
fn corner_act(id: &'static str, icon: &'static str, said: SharedString, ink: Token, group: &'static str, look: &Look) -> gpui::Stateful<gpui::Div> {
    let ring = look.color(token::ACCENT);
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .size(look.length(token::H_CONTROL_SM))
        .rounded(look.length(token::RADIUS_MD))
        .border(look.length(token::BORDER_WIDTH))
        .border_color(gpui::transparent_black())
        .cursor_pointer()
        .tab_stop(true)
        .focus_visible(move |style| style.opacity(1.).border_color(ring))
        .group_hover(group, |style| style.opacity(1.))
        .hover({
            let hover = look.color(token::BG_HOVER);
            move |style| style.bg(hover)
        })
        .tooltip(tip(said))
        .child(glyph(icon, look.length(token::SPACE_4), look.color(ink)))
}

/// A fenced block's acts over its corner (`RunnableBlock.tsx:458-494`): Copy, and Run where it holds a command and
/// `run` says one can run here. While its run is drawn under it the run's own head has the keys, so the block has
/// none; put away with Close, its mark stays and a press brings the run back (`DockedMark`).
#[allow(clippy::too_many_arguments)]
pub fn block_acts(job: &str, block: String, end: usize, command: Option<String>, copy: AnyElement, group: &'static str, look: &Look, cx: &App) -> AnyElement {
    if let Some((shown, pane)) = of_block(job, &block, cx) {
        if shown {
            return div().into_any_element();
        }
        let (icon, ink, said) = mark(pane.read(cx));
        let job = job.to_string();
        let show = corner_act("run-show", icon, "Show terminal".into(), ink, group, look).opacity(1.).on_click(move |_, _, cx| {
            if let Some(thread) = cx.default_global::<Runs>().threads.get_mut(&job) {
                thread.held.shown = true;
            }
            redraw(cx);
        });
        return labelled(show, Role::Button, format!("Show terminal: {said}")).into_any_element();
    }
    let Some(command) = command else { return copy };
    let job = job.to_string();
    // Named by its command, as Electron's group around the block is (`RunnableBlock.tsx:456`).
    let name = format!("Run {}", command.lines().next().unwrap_or_default());
    let run = corner_act("run", "play", "Run".into(), token::FG_SUBTLE, group, look).opacity(0.).on_click(move |_, _, cx| run_block(&job, &block, end, &command, cx));
    div().flex().gap(look.length(token::SPACE_1)).child(copy).child(labelled(run, Role::Button, name)).into_any_element()
}

/// A Needs you run item pressed: its command runs where Job `job` works, held under its row until Close.
/// `ended` hears how it exited, so the row's mark and the item's settling follow.
pub fn run_item(job: &str, command: &str, ended: Rc<dyn Fn(Option<i32>, bool, &mut App)>, cx: &mut App) {
    let Some(pane) = pane(job, command, cx) else { return };
    let key = (job.to_string(), command.to_string());
    let events = cx.subscribe(&pane, {
        let key = key.clone();
        move |pane, event: &PaneEvent, cx| {
            match event {
                PaneEvent::Closed => {
                    cx.default_global::<Runs>().items.remove(&key);
                }
                PaneEvent::Ended => {
                    let (code, stopped) = pane.read(cx).exit().unwrap_or((None, true));
                    ended(code, stopped, cx);
                }
                _ => {}
            }
            redraw(cx);
        }
    });
    let before = cx.default_global::<Runs>().items.insert(key, Held { pane, shown: true, _events: events });
    if let Some(before) = before {
        before.pane.update(cx, |pane, cx| pane.end_shell(cx));
    }
    redraw(cx);
}

/// Where the screen of a Needs you item's run was last drawn.
pub fn item_drawn(job: &str, command: &str, cx: &App) -> Option<gpui::Bounds<gpui::Pixels>> {
    cx.try_global::<Runs>()?.items.get(&(job.to_string(), command.to_string()))?.pane.read(cx).drawn()
}

/// The run held under a Needs you item's row, while it has one.
pub fn under_item(job: &str, command: &str, cx: &App) -> Option<AnyElement> {
    let held = cx.try_global::<Runs>()?.items.get(&(job.to_string(), command.to_string()))?;
    held.shown.then(|| held.pane.clone().into_any_element())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shell_fence_runs_and_other_languages_do_not() {
        assert_eq!(command_of(Some("bash"), "cargo build\n").as_deref(), Some("cargo build"));
        assert_eq!(command_of(Some("console"), "$ git status\n$ ls\n").as_deref(), Some("git status\nls"));
        assert_eq!(command_of(Some("rust"), "fn main() {}\n"), None);
        assert_eq!(command_of(None, "pnpm install\n").as_deref(), Some("pnpm install"));
        assert_eq!(command_of(None, "./scripts/preview --restart\n").as_deref(), Some("./scripts/preview --restart"));
    }

    #[test]
    fn an_untagged_block_of_code_prose_or_a_diagram_does_not_read_as_shell() {
        for source in ["const x = 1;", "Run the tests first.", "{\"a\": 1}", "a -> b", "crates/fleet/src/lib.rs", "foo = bar", "git add . &&"] {
            assert!(!reads_as_shell(source), "{source:?}");
        }
        for source in ["git status", "FOO=1 cargo test", "deploy --prod", "# a comment\nls -la", "cargo build &&\n  cargo test"] {
            assert!(reads_as_shell(source), "{source:?}");
        }
    }
}
