//! The screen a walk plays on, and playing one step on it. The window and the check both play
//! through here, so what the check passes is what the owner sees.

use gpui::{
    point, px, AnyView, App, AppContext, Entity, FocusHandle, Keystroke, Modifiers, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, PlatformInput, Point, Window,
};

use super::{Screen, Step, Verb, Walk};
use crate::jobs::{JobsList, JobsScreen};
use crate::labels::{role_named, Drawn, LabelBook};

/// The screen a walk plays on, fresh from its scenario.
pub struct Stage {
    pub view: AnyView,
    pub focus: Focus,
}

/// Where a step's keys go: the screen's own focus, unless something drawn inside the screen, such
/// as a menu it opened, holds them.
#[derive(Clone)]
pub struct Focus {
    pub keys: FocusHandle,
    pub within: FocusHandle,
}

impl Stage {
    pub fn new(walk: &Walk, window: &mut Window, cx: &mut App) -> Result<Stage, String> {
        crate::terminal::shells::hold_walk(cx);
        // A walk starts with nothing kept; what it forgets here is only in memory, never in the owner's file.
        crate::drafts::forget_all(cx);
        match walk.screen {
            Screen::Dispatch => {
                super::strip_marks::leave(cx);
                crate::titlebar::scenes::begin(cx);
                let records = crate::mock::jobs(walk.scenario)?;
                let list: Entity<JobsList> = cx.new(|cx| JobsList::mock(&records, cx));
                let keys = list.read(cx).focus_handle().clone();
                let jobs = cx.new(|cx| JobsScreen::new(list, None, window, cx));
                let screen = cx.new(|cx| crate::dispatch::DispatchScreen::new(jobs, &records, window, cx));
                let focus = Focus { keys, within: screen.read(cx).within.clone() };
                window.focus(&focus.keys, cx);
                Ok(Stage { view: screen.into(), focus })
            }
            Screen::Jobs => {
                super::strip_marks::leave(cx);
                crate::titlebar::scenes::begin(cx);
                let jobs = crate::mock::jobs(walk.scenario)?;
                let list: Entity<JobsList> = cx.new(|cx| JobsList::mock(&jobs, cx));
                let keys = list.read(cx).focus_handle().clone();
                let threads = crate::mock::threads(walk.scenario)?;
                let screen = cx.new(|cx| JobsScreen::new(list, Some(threads), window, cx));
                let focus = Focus { keys, within: screen.read(cx).within().clone() };
                window.focus(&focus.keys, cx);
                Ok(Stage { view: screen.into(), focus })
            }
            Screen::All => {
                // Every piece at once: the bar and the strip at rest, the dispatch sheet and the notes layer
                // around the Jobs screen, and the Job head's stand-in with no ring until a scene draws one.
                crate::titlebar::scenes::begin(cx);
                super::strip_marks::enter(cx);
                let records = crate::mock::jobs(walk.scenario)?;
                let list: Entity<JobsList> = cx.new(|cx| JobsList::mock(&records, cx));
                let keys = list.read(cx).focus_handle().clone();
                let threads = Some(crate::mock::threads(walk.scenario)?).filter(|threads| !threads.is_empty());
                let jobs = cx.new(|cx| JobsScreen::new(list, threads, window, cx));
                let screen = cx.new(|cx| crate::dispatch::DispatchScreen::new(jobs, &records, window, cx));
                let focus = Focus { keys, within: screen.read(cx).within.clone() };
                window.focus(&focus.keys, cx);
                let stood = cx.new(|cx| super::stand_in::JobHeadStandIn::new(screen.into(), cx));
                Ok(Stage { view: stood.into(), focus })
            }
            Screen::JobsInApp => {
                super::strip_marks::leave(cx);
                crate::titlebar::scenes::begin(cx);
                let jobs = crate::mock::jobs(walk.scenario)?;
                let list: Entity<JobsList> = cx.new(|cx| JobsList::mock(&jobs, cx));
                let keys = list.read(cx).focus_handle().clone();
                let threads = crate::mock::threads(walk.scenario)?;
                let screen = cx.new(|cx| JobsScreen::new(list, Some(threads), window, cx));
                let held = cx.new(|cx| crate::operator::OperatorScreen::mock_behind(screen, window, cx));
                let focus = Focus { keys, within: held.read(cx).within.clone() };
                window.focus(&focus.keys, cx);
                Ok(Stage { view: held.into(), focus })
            }
            Screen::App => {
                super::strip_marks::leave(cx);
                crate::titlebar::scenes::begin(cx);
                let records = crate::mock::jobs(walk.scenario)?;
                let list: Entity<JobsList> = cx.new(|cx| JobsList::mock(&records, cx));
                let keys = list.read(cx).focus_handle().clone();
                let threads = crate::mock::threads(walk.scenario)?;
                let jobs = cx.new(|cx| JobsScreen::new(list, Some(threads), window, cx));
                let held: gpui::AnyView = cx.new(|cx| crate::operator::OperatorScreen::mock_behind(jobs.clone(), window, cx)).into();
                let screen = cx.new(|cx| crate::dispatch::DispatchScreen::new(jobs, &records, window, cx).over(held));
                let focus = Focus { keys, within: screen.read(cx).within.clone() };
                // Settings over the whole, as main.rs stacks it, so ⌘, opens it and a note can be made on it.
                let over = cx.new(|cx| crate::settings::SettingsScreen::mock(screen.into(), window, cx));
                window.focus(&focus.keys, cx);
                Ok(Stage { view: over.into(), focus })
            }
            Screen::Operator => {
                super::strip_marks::leave(cx);
                crate::titlebar::scenes::begin(cx);
                let records = crate::mock::jobs(walk.scenario)?;
                let list: Entity<JobsList> = cx.new(|cx| JobsList::mock(&records, cx));
                let jobs = cx.new(|cx| JobsScreen::new(list, None, window, cx));
                let screen = cx.new(|cx| crate::operator::OperatorScreen::mock(jobs, window, cx));
                let focus = Focus { keys: screen.read(cx).page.read(cx).focus.clone(), within: screen.read(cx).within.clone() };
                window.focus(&focus.keys, cx);
                Ok(Stage { view: screen.into(), focus })
            }
            Screen::Settings => {
                super::strip_marks::leave(cx);
                crate::titlebar::scenes::begin(cx);
                let jobs = crate::mock::jobs(walk.scenario)?;
                let list: Entity<JobsList> = cx.new(|cx| JobsList::mock(&jobs, cx));
                let keys = list.read(cx).focus_handle().clone();
                let threads = crate::mock::threads(walk.scenario)?;
                let screen = cx.new(|cx| JobsScreen::new(list, Some(threads), window, cx));
                let held = cx.new(|cx| crate::operator::OperatorScreen::mock_behind(screen, window, cx));
                let over = cx.new(|cx| crate::settings::SettingsScreen::mock(held.into(), window, cx));
                if walk.name == crate::settings::walk::NO_FUNNEL {
                    over.update(cx, |over, cx| over.without_funnel(cx));
                }
                if walk.name == crate::settings::walk::WAYS {
                    over.update(cx, |over, cx| over.refusing("pictures_whole", cx));
                }
                if walk.name == crate::settings::walk::WAYS {
                    // The first-run screen is over all of it, as the app opens on a Fleet that has not been asked.
                    window.focus(&keys, cx);
                    let settings = over.read(cx).overlay.clone();
                    let first = cx.new(|cx| crate::first_run::FirstRunScreen::new(over.into(), settings, None, cx));
                    let focus = Focus { keys, within: first.read(cx).within.clone() };
                    return Ok(Stage { view: first.into(), focus });
                }
                let focus = Focus { keys, within: over.read(cx).within.clone() };
                window.focus(&focus.keys, cx);
                Ok(Stage { view: over.into(), focus })
            }
            Screen::Marks => {
                crate::titlebar::scenes::begin(cx);
                super::strip_marks::enter(cx);
                let jobs = crate::mock::jobs(walk.scenario)?;
                let list: Entity<JobsList> = cx.new(|cx| JobsList::mock(&jobs, cx));
                let keys = list.read(cx).focus_handle().clone();
                // A scenario with threads draws them, as the Jobs screen does; one without stays as it was.
                let threads = Some(crate::mock::threads(walk.scenario)?).filter(|threads| !threads.is_empty());
                let screen = cx.new(|cx| JobsScreen::new(list, threads, window, cx));
                // Inside the app's frame, as the Jobs screen always is, so Fleet's states raise its toasts.
                let held = cx.new(|cx| crate::operator::OperatorScreen::mock_behind(screen, window, cx));
                let focus = Focus { keys, within: held.read(cx).within.clone() };
                window.focus(&focus.keys, cx);
                let stood = cx.new(|cx| super::stand_in::JobHeadStandIn::new(held.into(), cx));
                Ok(Stage { view: stood.into(), focus })
            }
        }
    }
}

/// How many elements the step's target named when a `Gone` step pressed, so the step reads as met once fewer are
/// drawn: a name two elements share, such as the head's Stop and the thread's, goes when the one pressed does.
static GONE_FROM: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// How many elements the window last drew under the step's target.
pub fn drawn_count(step: &Step, window: &Window, cx: &App) -> usize {
    let Some(role) = role_named(step.target.role) else { return 0 };
    cx.try_global::<LabelBook>()
        .map_or(0, |book| book.drawn(window.window_handle().window_id()).iter().filter(|drawn| drawn.role == role && drawn.name == step.target.name).count())
}

/// Whether a `Gone` step's target is no longer drawn as often as it was when the step pressed it.
pub fn gone(step: &Step, window: &Window, cx: &App) -> bool {
    drawn_count(step, window, cx) < GONE_FROM.load(std::sync::atomic::Ordering::Relaxed)
}

/// What a step could not do, in the words the check prints.
#[derive(Debug)]
pub struct Missed(pub String);

/// The element a step aims at, as the window last drew it.
pub fn find(step: &Step, window: &Window, cx: &App) -> Result<Drawn, Missed> {
    let role = role_named(step.target.role).ok_or_else(|| Missed(format!("no role named {}", step.target.role)))?;
    // The last one drawn: an element the list measured off screen comes first in the book. A part's
    // first ring takes the first, as the press after it does, so two rows of one name ring the one pressed.
    let first = matches!(step.verb, Verb::FreshIn(..));
    cx.try_global::<LabelBook>()
        .and_then(|book| {
            let mut drawn = book.drawn(window.window_handle().window_id()).iter().filter(|drawn| drawn.role == role && drawn.name == step.target.name);
            if first { drawn.next() } else { drawn.last() }.cloned()
        })
        .ok_or_else(|| {
            let seen: Vec<String> = cx.try_global::<LabelBook>().map(|b| b.drawn(window.window_handle().window_id()).iter().filter(|d| d.role == role).map(|d| d.name.to_string()).collect()).unwrap_or_default();
            Missed(format!("no {} named \"{}\"; drawn: {:?}", step.target.role, step.target.name, seen))
        })
}

/// Do what reaching `step` does. A press needs its target drawn first; a key goes to the screen.
pub fn perform(step: &Step, focus: &Focus, window: &mut Window, cx: &mut App) -> Result<(), Missed> {
    match step.verb {
        Verb::Look | Verb::Later(_) => Ok(()),
        // The walk window builds the screen again before it reaches here (`WalkWindow::play_to`).
        Verb::Fresh | Verb::FreshIn(..) => Ok(()),
        Verb::Width(points) => {
            let height = window.viewport_size().height;
            window.resize(gpui::size(gpui::px(points as f32), height));
            // A window off screen hears no resize from the platform; on screen this is said twice.
            window.bounds_changed(cx);
            Ok(())
        }
        Verb::Widen(points) => {
            let size = window.viewport_size();
            if size.width < px(points as f32) {
                window.resize(gpui::size(px(points as f32), size.height));
                window.bounds_changed(cx);
            }
            Ok(())
        }
        // The walk window has stopped playing for this one (`WalkWindow::play_to`), so the key is the person's.
        Verb::NoteKey => perform(&Step { verb: Verb::Key("alt-cmd-a"), ..*step }, focus, window, cx),
        Verb::Key(key) => {
            let keystroke = Keystroke::parse(key).map_err(|why| Missed(format!("the key {key} does not parse: {why}")))?;
            if !focus.within.contains_focused(window, cx) {
                window.focus(&focus.keys, cx);
            }
            strike(keystroke, window, cx);
            Ok(())
        }
        Verb::Keys(keys) => {
            if !focus.within.contains_focused(window, cx) {
                window.focus(&focus.keys, cx);
            }
            for key in keys {
                let keystroke = Keystroke::parse(key).map_err(|why| Missed(format!("the key {key} does not parse: {why}")))?;
                window.draw(cx).clear(cx);
                strike(keystroke, window, cx);
            }
            Ok(())
        }
        Verb::Press => {
            let target = find(step, window, cx)?;
            let at = aim(&reveal(target, window, cx));
            press(MouseButton::Left, at, window, cx);
            Ok(())
        }
        Verb::DoublePress => {
            let target = find(step, window, cx)?;
            let at = aim(&reveal(target, window, cx));
            for click_count in [1, 2] {
                let down = MouseDownEvent { button: MouseButton::Left, position: at, modifiers: Modifiers::default(), click_count, first_mouse: false };
                window.dispatch_event(PlatformInput::MouseDown(down), cx);
                window.dispatch_event(PlatformInput::MouseUp(MouseUpEvent { button: MouseButton::Left, position: at, modifiers: Modifiers::default(), click_count }), cx);
            }
            Ok(())
        }
        Verb::CmdPress => {
            let at = find(step, window, cx)?.bounds.center();
            let modifiers = Modifiers { platform: true, ..Modifiers::default() };
            window.dispatch_event(PlatformInput::MouseDown(MouseDownEvent { button: MouseButton::Left, position: at, modifiers, click_count: 1, first_mouse: false }), cx);
            window.dispatch_event(PlatformInput::MouseUp(MouseUpEvent { button: MouseButton::Left, position: at, modifiers, click_count: 1 }), cx);
            Ok(())
        }
        Verb::Gone => {
            // The first drawn, which is the head's where the thread draws a twin below it.
            let role = role_named(step.target.role).ok_or_else(|| Missed(format!("no role named {}", step.target.role)))?;
            let found = cx
                .try_global::<LabelBook>()
                .and_then(|book| book.drawn(window.window_handle().window_id()).iter().find(|drawn| drawn.role == role && drawn.name == step.target.name).cloned());
            let Some(found) = found else { return find(step, window, cx).map(|_| ()) };
            let found = reveal(found, window, cx);
            GONE_FROM.store(drawn_count(step, window, cx), std::sync::atomic::Ordering::Relaxed);
            press(MouseButton::Left, aim(&found), window, cx);
            Ok(())
        }
        Verb::RightPress(on) => {
            let book = cx.try_global::<LabelBook>();
            let id = window.window_handle().window_id();
            // The one of that name that shows, where a narrow window draws a clipped twin first.
            let found = |word: &str| {
                let role = role_named(word).expect("a role the walk names");
                let all: Vec<Drawn> = book.map(|book| book.drawn(id).iter().filter(|one| one.role == role && one.name == on).cloned().collect()).unwrap_or_default();
                all.iter().find(|one| shows(one)).or(all.first()).cloned()
            };
            let drawn = found("row").or_else(|| found("button")).ok_or_else(|| Missed(format!("no row or button named \"{on}\" to press")))?;
            let drawn = reveal(drawn, window, cx);
            press(MouseButton::Right, aim(&drawn), window, cx);
            Ok(())
        }
        Verb::Type(text) => {
            // The press focuses the box; each letter then goes in as the platform's input method would.
            let at = aim(&reveal(find(step, window, cx)?, window, cx));
            press(MouseButton::Left, at, window, cx);
            // The words go after what the box holds, wherever in it the press put the caret.
            if let Ok(end) = Keystroke::parse("end") {
                strike(end, window, cx);
            }
            for ch in text.chars() {
                let key = if ch == ' ' { "space".to_string() } else { ch.to_string() };
                let keystroke = Keystroke { modifiers: Modifiers::default(), key, key_char: Some(ch.to_string()) };
                window.dispatch_keystroke(keystroke, cx);
            }
            Ok(())
        }
        Verb::Open(role, on) => {
            let role = role_named(role).ok_or_else(|| Missed(format!("no role named {role}")))?;
            // The last one drawn: an element the list measured off screen comes first in the book, and the
            // head's own Approve is drawn before the thread's.
            let drawn = cx
                .try_global::<LabelBook>()
                .and_then(|book| book.drawn(window.window_handle().window_id()).iter().rev().find(|drawn| drawn.role == role && drawn.name == on).cloned())
                .ok_or_else(|| Missed(format!("nothing named \"{on}\" to press")))?;
            let drawn = reveal(drawn, window, cx);
            press(MouseButton::Left, aim(&drawn), window, cx);
            Ok(())
        }
        Verb::OpenIn(article, on) => {
            let id = window.window_handle().window_id();
            let find_in = |cx: &App| -> Result<(Drawn, Drawn), Missed> {
                let drawn = cx.try_global::<LabelBook>().map(|book| book.drawn(id).to_vec()).unwrap_or_default();
                let card = drawn
                    .iter()
                    .rev()
                    .find(|one| one.role == gpui::Role::Article && one.name == article)
                    .cloned()
                    .ok_or_else(|| Missed(format!("no article named \"{article}\"")))?;
                let button = drawn
                    .iter()
                    .rev()
                    .find(|one| one.role == gpui::Role::Button && one.name == on && card.bounds.contains(&one.bounds.center()))
                    .cloned()
                    .ok_or_else(|| Missed(format!("no button named \"{on}\" in \"{article}\"")))?;
                Ok((card, button))
            };
            let (_, button) = find_in(cx)?;
            let button = reveal(button, window, cx);
            press(MouseButton::Left, aim(&button), window, cx);
            Ok(())
        }
        Verb::PressWithin(role, within, on) => {
            let role = role_named(role).ok_or_else(|| Missed(format!("no role named {role}")))?;
            let drawn = cx.try_global::<LabelBook>().map(|book| book.drawn(window.window_handle().window_id()).to_vec()).unwrap_or_default();
            let at = drawn.iter().rposition(|one| one.role == role && one.name == within).ok_or_else(|| Missed(format!("nothing named \"{within}\"")))?;
            let outer = drawn[at].clone();
            // Its own, drawn after it: a scrolled element's bounds can reach under a bar drawn later.
            let button = drawn[at..]
                .iter()
                .find(|one| one.role == gpui::Role::Button && one.name == on && outer.bounds.contains(&one.bounds.center()))
                .cloned()
                .ok_or_else(|| Missed(format!("no button named \"{on}\" in \"{within}\"")))?;
            press(MouseButton::Left, aim(&reveal(button, window, cx)), window, cx);
            Ok(())
        }
        Verb::RightOpen(role, on) => {
            let role = role_named(role).ok_or_else(|| Missed(format!("no role named {role}")))?;
            // The last one drawn: an element the list measured off screen comes first in the book.
            let drawn = cx
                .try_global::<LabelBook>()
                .and_then(|book| book.drawn(window.window_handle().window_id()).iter().rev().find(|drawn| drawn.role == role && drawn.name == on).cloned())
                .ok_or_else(|| Missed(format!("nothing named \"{on}\" to press")))?;
            press(MouseButton::Right, aim(&reveal(drawn, window, cx)), window, cx);
            Ok(())
        }
        Verb::Wheel(role, on, by) => {
            let role = role_named(role).ok_or_else(|| Missed(format!("no role named {role}")))?;
            let drawn = cx
                .try_global::<LabelBook>()
                .and_then(|book| book.drawn(window.window_handle().window_id()).iter().rev().find(|drawn| drawn.role == role && drawn.name == on).cloned())
                .ok_or_else(|| Missed(format!("nothing named \"{on}\" to scroll")))?;
            window.dispatch_event(
                PlatformInput::Scroll(gpui::ScrollEvent {
                    // The part a person can point at: a long list's middle can be under its pane's clip.
                    position: drawn.shown.center(),
                    delta: gpui::ScrollDelta::Pixels(point(px(0.), px(-(by as f32)))),
                    modifiers: Modifiers::default(),
                    touch_phase: gpui::TouchPhase::Moved,
                    source: gpui::ScrollSource::default(),
                }),
                cx,
            );
            Ok(())
        }
        Verb::WheelTo(role, on, by) => {
            let role = role_named(role).ok_or_else(|| Missed(format!("no role named {role}")))?;
            for _ in 0..60 {
                if find(step, window, cx).is_ok() {
                    return Ok(());
                }
                let drawn = cx
                    .try_global::<LabelBook>()
                    .and_then(|book| book.drawn(window.window_handle().window_id()).iter().rev().find(|drawn| drawn.role == role && drawn.name == on).cloned())
                    .ok_or_else(|| Missed(format!("nothing named \"{on}\" to scroll")))?;
                window.dispatch_event(
                    PlatformInput::Scroll(gpui::ScrollEvent {
                        position: drawn.shown.center(),
                        delta: gpui::ScrollDelta::Pixels(point(px(0.), px(-(by as f32)))),
                        modifiers: Modifiers::default(),
                        touch_phase: gpui::TouchPhase::Moved,
                        source: gpui::ScrollSource::default(),
                    }),
                    cx,
                );
                window.refresh();
                window.draw(cx).clear(cx);
            }
            Ok(())
        }
        Verb::PressStart(role, on) => {
            let role = role_named(role).ok_or_else(|| Missed(format!("no role named {role}")))?;
            let drawn = cx
                .try_global::<LabelBook>()
                .and_then(|book| book.drawn(window.window_handle().window_id()).iter().rev().find(|drawn| drawn.role == role && drawn.name == on).cloned())
                .ok_or_else(|| Missed(format!("nothing named \"{on}\" to press in")))?;
            let at = point(drawn.bounds.left() + px(12.), drawn.bounds.top() + px(12.));
            window.dispatch_event(
                PlatformInput::MouseDown(MouseDownEvent { button: MouseButton::Left, position: at, modifiers: Modifiers::default(), click_count: 1, first_mouse: false }),
                cx,
            );
            window.dispatch_event(PlatformInput::MouseUp(MouseUpEvent { button: MouseButton::Left, position: at, modifiers: Modifiers::default(), click_count: 1 }), cx);
            Ok(())
        }
        Verb::Select(role, on) | Verb::SelectAll(role, on) => {
            let all = matches!(step.verb, Verb::SelectAll(..));
            let role = role_named(role).ok_or_else(|| Missed(format!("no role named {role}")))?;
            // The last one drawn: an element the list measured off screen comes first in the book.
            let drawn = cx
                .try_global::<LabelBook>()
                .and_then(|book| book.drawn(window.window_handle().window_id()).iter().rev().find(|drawn| drawn.role == role && drawn.name == on).cloned())
                .ok_or_else(|| Missed(format!("nothing named \"{on}\" to select in")))?;
            let drawn = reveal(drawn, window, cx);
            // The last line, or the last in the shown part's upper two thirds, clear of what a pane lays over its
            // foot (the subagents card) when the words run past the view.
            let line = (drawn.bounds.bottom() - px(16.)).min(drawn.shown.top() + drawn.shown.size.height * (2. / 3.));
            // All of it runs from the first line's start to the last line's end; otherwise the left third of the last line.
            let from = point(drawn.bounds.left() + px(3.), if all { drawn.bounds.top() + px(12.) } else { line });
            let to = point(if all { drawn.bounds.right() - px(3.) } else { drawn.bounds.left() + drawn.bounds.size.width / 3. }, line);
            window.dispatch_event(
                PlatformInput::MouseDown(MouseDownEvent { button: MouseButton::Left, position: from, modifiers: Modifiers::default(), click_count: 1, first_mouse: false }),
                cx,
            );
            for step in 1..=6 {
                let at = point(from.x + (to.x - from.x) * (step as f32 / 6.), from.y + (to.y - from.y) * (step as f32 / 6.));
                window.dispatch_event(PlatformInput::MouseMove(MouseMoveEvent { position: at, pressed_button: Some(MouseButton::Left), modifiers: Modifiers::default() }), cx);
            }
            window.dispatch_event(PlatformInput::MouseUp(MouseUpEvent { button: MouseButton::Left, position: to, modifiers: Modifiers::default(), click_count: 1 }), cx);
            Ok(())
        }
        Verb::RightPressOn(role, name) => {
            let role = role_named(role).ok_or_else(|| Missed(format!("no role named {role}")))?;
            let at = cx
                .try_global::<LabelBook>()
                .and_then(|book| book.find(window.window_handle().window_id(), role, name))
                .map(|drawn| drawn.bounds.center())
                .ok_or_else(|| Missed(format!("nothing named \"{name}\" to press")))?;
            press(MouseButton::Right, at, window, cx);
            Ok(())
        }
        Verb::Scene(name) => {
            // Narrowing takes the start off the row, and the keys with it if they were there: they go back to the screen.
            if name == "narrow-window" {
                window.focus(&focus.keys, cx);
            }
            // A title bar scene is the bar's alone: a Job head stand-in a walk drew before it goes.
            super::stand_in::hide(cx);
            crate::titlebar::scenes::apply(name, cx).map_err(Missed)
        }
        Verb::PressOnFirst(doors) => {
            let shown = |role: gpui::Role, name: &str, cx: &App| {
                cx.try_global::<LabelBook>().and_then(|book| {
                    book.drawn(window.window_handle().window_id()).iter().find(|drawn| drawn.role == role && drawn.name == name && drawn.shown.size.width > px(0.) && drawn.shown.size.height > px(0.)).cloned()
                })
            };
            let at = doors
                .iter()
                .find_map(|(role, name)| role_named(role).and_then(|role| shown(role, name, cx)))
                .ok_or_else(|| Missed(format!("none of {doors:?} is drawn to press")))?;
            let at = reveal(at, window, cx);
            let at = aim(&at);
            press(MouseButton::Left, at, window, cx);
            Ok(())
        }
        Verb::PressOn(role, name) => {
            let role = role_named(role).ok_or_else(|| Missed(format!("no role named {role}")))?;
            // The first one showing: a list's off-screen measure of the same element comes first in the
            // book, and a press there lands on whatever sits at its place.
            let at = cx
                .try_global::<LabelBook>()
                .and_then(|book| {
                    let mut named = book.drawn(window.window_handle().window_id()).iter().filter(|drawn| drawn.role == role && drawn.name == name);
                    let first = named.clone().next();
                    named.find(|drawn| drawn.shown.size.width > px(0.) && drawn.shown.size.height > px(0.)).or(first).cloned()
                })
                .ok_or_else(|| Missed(format!("nothing named \"{name}\" to press")))?;
            let at = reveal(at, window, cx);
            let at = aim(&at);
            press(MouseButton::Left, at, window, cx);
            Ok(())
        }
        Verb::Marks(name) => {
            // A scene is a jump: the keys go back to the screen first, so a chip or a card the scene
            // takes away does not leave them on nothing.
            window.focus(&focus.keys, cx);
            // The pointer goes to a corner no scene draws in, so a card or tip it rested on is not the scene's.
            let corner = point(gpui::px(1.), gpui::px(1.));
            window.dispatch_event(PlatformInput::MouseMove(MouseMoveEvent { position: corner, pressed_button: None, modifiers: Modifiers::default() }), cx);
            super::strip_marks::apply(name, cx).map_err(Missed)
        }
        Verb::PausedScope => {
            let role = role_named("button").expect("a button is a role");
            let at = cx
                .try_global::<LabelBook>()
                .and_then(|book| book.find(window.window_handle().window_id(), role, "Active"))
                .map(|drawn| drawn.bounds.center())
                .ok_or_else(|| Missed("no button named \"Active\"".into()))?;
            press(MouseButton::Left, at, window, cx);
            // The menu is drawn before Paused is pressed, as a person's second press would find it. Paused is
            // found by name, so the scopes' order can change under the step.
            window.defer(cx, |window, cx| {
                window.draw(cx).clear(cx);
                let role = role_named("menuitemcheckbox").expect("a menu item checkbox is a role");
                let paused = cx.try_global::<LabelBook>().and_then(|book| book.find(window.window_handle().window_id(), role, "Paused")).map(|drawn| drawn.bounds.center());
                if let Some(at) = paused {
                    press(MouseButton::Left, at, window, cx);
                }
            });
            Ok(())
        }
        Verb::Answer(name) => {
            if let Some(page) = crate::operator::live(cx) {
                // A name the page does not know is the dispatch screen's, where the whole app is in the walk.
                match page.update(cx, |page, cx| page.scene(name, cx)) {
                    Err(why) if why.starts_with("no Operator scene named") && crate::dispatch::screen::live(cx).is_some() => {}
                    done => return done.map_err(Missed),
                }
            }
            let screen = crate::dispatch::screen::live(cx).ok_or_else(|| Missed("no dispatch screen is drawn yet".into()))?;
            screen.update(cx, |screen, cx| screen.scene(name, window, cx)).map_err(Missed)
        }
        Verb::Typed(text) => {
            for ch in text.chars() {
                let key = if ch == ' ' { "space".to_string() } else { ch.to_string() };
                let keystroke = Keystroke { modifiers: Modifiers::default(), key, key_char: Some(ch.to_string()) };
                window.dispatch_keystroke(keystroke, cx);
            }
            Ok(())
        }
        Verb::Drag => {
            let bounds = find(step, window, cx)?.bounds;
            // From the element's top-left to 70% across and 85% down: a box of its own, whose top-right
            // corner (where the pin stands) is clear of anything drawn at the element's edge, and whose
            // card still opens below it.
            let from = bounds.origin;
            let to = point(bounds.origin.x + bounds.size.width * 0.7, bounds.origin.y + bounds.size.height * 0.85);
            let none = Modifiers::default();
            window.dispatch_event(PlatformInput::MouseMove(MouseMoveEvent { position: from, pressed_button: None, modifiers: none }), cx);
            window.dispatch_event(
                PlatformInput::MouseDown(MouseDownEvent { button: MouseButton::Left, position: from, modifiers: none, click_count: 1, first_mouse: false }),
                cx,
            );
            window.dispatch_event(
                PlatformInput::MouseMove(MouseMoveEvent { position: to, pressed_button: Some(MouseButton::Left), modifiers: none }),
                cx,
            );
            window.dispatch_event(PlatformInput::MouseUp(MouseUpEvent { button: MouseButton::Left, position: to, modifiers: none, click_count: 1 }), cx);
            Ok(())
        }
        Verb::Lift(from) => {
            let role = role_named("row").expect("a row is a role");
            let start = cx
                .try_global::<LabelBook>()
                .and_then(|book| book.find(window.window_handle().window_id(), role, from))
                .map(|drawn| drawn.bounds.center())
                .ok_or_else(|| Missed(format!("no row named \"{from}\"")))?;
            let over = find(step, window, cx)?.bounds;
            let to = point(over.center().x, over.origin.y + over.size.height * 0.25);
            let none = Modifiers::default();
            let held = Some(MouseButton::Left);
            window.dispatch_event(PlatformInput::MouseMove(MouseMoveEvent { position: start, pressed_button: None, modifiers: none }), cx);
            window.dispatch_event(
                PlatformInput::MouseDown(MouseDownEvent { button: MouseButton::Left, position: start, modifiers: none, click_count: 1, first_mouse: false }),
                cx,
            );
            // The first move past the threshold lifts the row; the list is drawn again so the next move reaches the rows' drop handlers.
            let past = point(start.x, start.y - gpui::px(8.));
            window.dispatch_event(PlatformInput::MouseMove(MouseMoveEvent { position: past, pressed_button: held, modifiers: none }), cx);
            window.defer(cx, move |window, cx| {
                window.draw(cx).clear(cx);
                window.dispatch_event(PlatformInput::MouseMove(MouseMoveEvent { position: to, pressed_button: held, modifiers: none }), cx);
            });
            Ok(())
        }
        Verb::Fits(role, name) => {
            let inner = find(step, window, cx)?.bounds;
            let outer_role = role_named(role).ok_or_else(|| Missed(format!("no role named {role}")))?;
            let outer = cx
                .try_global::<LabelBook>()
                .and_then(|book| book.find(window.window_handle().window_id(), outer_role, name))
                .map(|drawn| drawn.bounds)
                .ok_or_else(|| Missed(format!("no {role} named \"{name}\"")))?;
            if inner.left() < outer.left() || inner.right() > outer.right() {
                return Err(Missed(format!(
                    "the {} \"{}\" runs past the {role} \"{name}\": {:?} against {:?}",
                    step.target.role, step.target.name, (inner.left(), inner.right()), (outer.left(), outer.right())
                )));
            }
            Ok(())
        }
        Verb::Absent(role, name) => {
            let absent_role = role_named(role).ok_or_else(|| Missed(format!("no role named {role}")))?;
            if cx.try_global::<LabelBook>().and_then(|book| book.find(window.window_handle().window_id(), absent_role, name)).is_some() {
                return Err(Missed(format!("a {role} named \"{name}\" is drawn")));
            }
            find(step, window, cx).map(|_| ())
        }
        Verb::Release => {
            let at = window.mouse_position();
            window.dispatch_event(
                PlatformInput::MouseUp(MouseUpEvent { button: MouseButton::Left, position: at, modifiers: Modifiers::default(), click_count: 1 }),
                cx,
            );
            Ok(())
        }
        Verb::Hover => {
            let target = find(step, window, cx)?;
            let mut shown = reveal(target, window, cx);
            // What the pointer brings up can move the thread under it a frame later; it follows the target there.
            for _ in 0..5 {
                let at = aim(&shown);
                window.dispatch_event(
                    PlatformInput::MouseMove(MouseMoveEvent { position: at, pressed_button: None, modifiers: Modifiers::default() }),
                    cx,
                );
                let now = settle(shown.clone(), window, cx);
                if now.bounds.contains(&at) {
                    break;
                }
                shown = now;
            }
            Ok(())
        }
    }
}

/// A press in the middle of the row named `label`, as the window last drew it.
pub fn press_row(label: &str, window: &mut Window, cx: &mut App) {
    let role = role_named("row").expect("a row is a role");
    let at = cx
        .try_global::<LabelBook>()
        .and_then(|book| book.find(window.window_handle().window_id(), role, label))
        .map(|drawn| drawn.bounds.center());
    if let Some(at) = at {
        press(MouseButton::Left, at, window, cx);
    }
}

/// A target its pane clips away whole is scrolled into view first, as a person scrolls to a button
/// before pressing it: a walk sized at one width leaves things under a narrower window's edge. The
/// wheel turns over the nearest showing element around it, and the book is read again. A target
/// that shows at all stays where it is.
fn reveal(target: Drawn, window: &mut Window, cx: &mut App) -> Drawn {
    let was = target.bounds;
    let mut shown = scroll_to(target, window, cx);
    if shown.bounds != was {
        // What covers it is read from where it is drawn now, not from the frame before the scroll.
        shown = settle(shown, window, cx);
    }
    let at = shown.bounds;
    shown = uncover(shown, window, cx);
    if shown.bounds == at {
        return shown;
    }
    settle(shown, window, cx)
}

/// The target found again, drawn after drawn, until it stays put: after a scroll the book can still hold
/// where it was a frame before.
fn settle(mut target: Drawn, window: &mut Window, cx: &mut App) -> Drawn {
    let id = window.window_handle().window_id();
    for _ in 0..4 {
        // A view nobody marked changed is drawn from its last layout; the whole window is laid out again.
        window.refresh();
        window.draw(cx).clear(cx);
        let near = target.bounds.center();
        let distance = |one: &Drawn| {
            let at = one.bounds.center();
            (at.x - near.x).abs() + (at.y - near.y).abs()
        };
        let Some(now) = cx.try_global::<LabelBook>().and_then(|book| {
            book.drawn(id)
                .iter()
                .filter(|one| one.role == target.role && one.name == target.name && shows(one))
                .min_by(|a, b| distance(a).partial_cmp(&distance(b)).unwrap_or(std::cmp::Ordering::Equal))
                .cloned()
        }) else {
            break;
        };
        let moved = now.bounds != target.bounds;
        target = now;
        if !moved {
            break;
        }
    }
    target
}

fn shows(one: &Drawn) -> bool {
    one.shown.size.width > px(0.) && one.shown.size.height > px(0.)
}

fn holds(outer: gpui::Bounds<Pixels>, inner: gpui::Bounds<Pixels>) -> bool {
    outer.left() <= inner.left() && outer.right() >= inner.right() && outer.top() <= inner.top() && outer.bottom() >= inner.bottom()
}

/// A target drawn under something laid over its pane (the subagents card at the thread's foot) is wheeled up
/// clear of it, as a person scrolls a row out from under the card before pressing it. What lies over the
/// point is anything drawn after it there that is neither around it nor inside it.
fn uncover(target: Drawn, window: &mut Window, cx: &mut App) -> Drawn {
    let id = window.window_handle().window_id();
    let at = aim(&target);
    let cover = cx.try_global::<LabelBook>().and_then(|book| {
        let drawn = book.drawn(id);
        let mine = drawn.iter().position(|one| one.role == target.role && one.name == target.name && one.bounds == target.bounds)?;
        drawn[mine + 1..].iter().find(|one| shows(one) && one.shown.contains(&at) && !holds(one.bounds, target.bounds) && !holds(target.bounds, one.bounds)).cloned()
    });
    let Some(cover) = cover else { return target };
    let by = point(px(0.), cover.shown.top() - px(24.) - target.bounds.bottom());
    if by.y >= px(0.) {
        return target;
    }
    wheel(target, point(at.x, cover.shown.top() - px(4.)), by, window, cx)
}

/// Scroll by `by` at `over`, draw, and find the target where the scroll took it.
fn wheel(target: Drawn, over: Point<Pixels>, by: Point<Pixels>, window: &mut Window, cx: &mut App) -> Drawn {
    let id = window.window_handle().window_id();
    let t = target.bounds;
    window.dispatch_event(
        PlatformInput::Scroll(gpui::ScrollEvent {
            position: over,
            delta: gpui::ScrollDelta::Pixels(by),
            modifiers: Modifiers::default(),
            touch_phase: gpui::TouchPhase::Moved,
            source: gpui::ScrollSource::default(),
        }),
        cx,
    );
    window.draw(cx).clear(cx);
    // The same element where the scroll took it: the showing one of its name nearest there.
    let expected = t.center() + by;
    let distance = |one: &Drawn| {
        let at = one.bounds.center();
        (at.x - expected.x).abs() + (at.y - expected.y).abs()
    };
    cx.try_global::<LabelBook>()
        .and_then(|book| {
            book.drawn(id)
                .iter()
                .filter(|one| one.role == target.role && one.name == target.name && shows(one))
                .min_by(|a, b| distance(a).partial_cmp(&distance(b)).unwrap_or(std::cmp::Ordering::Equal))
                .cloned()
        })
        .unwrap_or(target)
}

fn scroll_to(target: Drawn, window: &mut Window, cx: &mut App) -> Drawn {
    if shows(&target) {
        return target;
    }
    let t = target.bounds;
    // Cut off at the side by a row that scrolls sideways (the quick bar's saved lines): the row is wheeled
    // over the nearest neighbour that shows, until the target's far edge sits where that neighbour's does.
    let s = target.shown;
    if s.size.height > px(0.) && s.size.width < t.size.width {
        let id = window.window_handle().window_id();
        let right = s.left() <= t.left();
        let neighbour = cx.try_global::<LabelBook>().and_then(|book| {
            book.drawn(id)
                .iter()
                .filter(|one| {
                    let b = one.shown;
                    shows(one) && (b.top() - t.top()).abs() <= px(1.) && (b.bottom() - t.bottom()).abs() <= px(1.) && if right { b.right() <= t.left() } else { b.left() >= t.right() }
                })
                .min_by(|a, b| {
                    let gap = |one: &Drawn| if right { t.left() - one.shown.right() } else { one.shown.left() - t.right() };
                    f32::from(gap(a)).partial_cmp(&f32::from(gap(b))).unwrap_or(std::cmp::Ordering::Equal)
                })
                .cloned()
        });
        if let Some(neighbour) = neighbour {
            let n = neighbour.shown;
            let by = if right { n.right() - t.right() } else { n.left() - t.left() };
            return wheel(target, n.center(), point(by, px(0.)), window, cx);
        }
    }
    let id = window.window_handle().window_id();
    // Around it, or else around it give or take a hair: a message's hover bar hangs just past the message.
    // Never one that shows the target's whole place: that one is not what cuts it off (the pane under a hidden row's place).
    let within = |slack: Pixels| {
        cx.try_global::<LabelBook>().and_then(|book| {
            book.drawn(id)
                .iter()
                .filter(|one| {
                    let b = one.bounds;
                    shows(one) && !holds(one.shown, t) && b.left() - slack <= t.left() && b.right() + slack >= t.right() && b.top() - slack <= t.top() && b.bottom() + slack >= t.bottom()
                })
                .min_by(|a, b| (f32::from(a.bounds.size.width) * f32::from(a.bounds.size.height)).partial_cmp(&(f32::from(b.bounds.size.width) * f32::from(b.bounds.size.height))).unwrap_or(std::cmp::Ordering::Equal))
                .cloned()
        })
    };
    // Or else the nearest one its pane clips in the same columns: it sits in the same scrolled content.
    let clipped = || {
        cx.try_global::<LabelBook>().and_then(|book| {
            let off = |one: &Drawn| {
                let b = one.bounds;
                if t.bottom() < b.top() { b.top() - t.bottom() } else if t.top() > b.bottom() { t.top() - b.bottom() } else { px(0.) }
            };
            book.drawn(id)
                .iter()
                .filter(|one| {
                    let b = one.bounds;
                    shows(one) && one.shown.size.height < b.size.height - px(1.) && b.left() - px(16.) <= t.left() && b.right() + px(16.) >= t.right()
                })
                .min_by(|a, b| f32::from(off(a)).partial_cmp(&f32::from(off(b))).unwrap_or(std::cmp::Ordering::Equal))
                .cloned()
        })
    };
    let around = within(px(0.)).or_else(|| within(px(16.))).or_else(clipped);
    let Some(around) = around else { return target };
    let view = around.shown;
    let margin = px(24.);
    let along = |low: Pixels, high: Pixels, from: Pixels, to: Pixels, foot: Pixels| {
        if low < from { from - low + margin } else if high > to { to - high - foot } else { px(0.) }
    };
    // Up from below, it stops a third of the view above the foot, clear of what a pane lays there (the subagents card).
    let foot = margin.max(view.size.height / 3.);
    let by = point(along(t.left(), t.right(), view.left(), view.right(), margin), along(t.top(), t.bottom(), view.top(), view.bottom(), foot));
    if by == point(px(0.), px(0.)) {
        return target;
    }
    wheel(target, view.center(), by, window, cx)
}

/// Where a person presses it: the middle of the part a clipping pane lets show, as Wheel aims.
fn aim(drawn: &Drawn) -> Point<Pixels> {
    if drawn.shown.size.width > px(0.) && drawn.shown.size.height > px(0.) { drawn.shown.center() } else { drawn.bounds.center() }
}

fn press(button: MouseButton, at: Point<Pixels>, window: &mut Window, cx: &mut App) {
    let at = point(at.x, at.y);
    window.dispatch_event(
        PlatformInput::MouseDown(MouseDownEvent { button, position: at, modifiers: Modifiers::default(), click_count: 1, first_mouse: false }),
        cx,
    );
    window.dispatch_event(PlatformInput::MouseUp(MouseUpEvent { button, position: at, modifiers: Modifiers::default(), click_count: 1 }), cx);
}

/// A key as a person's finger sends it: down, and for Enter and Space up again, which is what presses a
/// focused element.
fn strike(keystroke: Keystroke, window: &mut Window, cx: &mut App) {
    let lifts = (keystroke.key == "enter" || keystroke.key == "space") && !keystroke.modifiers.modified();
    window.dispatch_keystroke(keystroke.clone(), cx);
    if lifts {
        window.dispatch_event(PlatformInput::KeyUp(gpui::KeyUpEvent { keystroke }), cx);
    }
}
