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
                let jobs = cx.new(|cx| JobsScreen::new(list, None, window, cx));
                let screen = cx.new(|cx| crate::dispatch::DispatchScreen::new(jobs, &records, window, cx));
                let focus = Focus { keys, within: screen.read(cx).within.clone() };
                window.focus(&focus.keys, cx);
                let stood = cx.new(|cx| super::stand_in::JobHeadStandIn::new(screen.into(), cx));
                Ok(Stage { view: stood.into(), focus })
            }
            Screen::Marks => {
                crate::titlebar::scenes::begin(cx);
                super::strip_marks::enter(cx);
                let jobs = crate::mock::jobs(walk.scenario)?;
                let list: Entity<JobsList> = cx.new(|cx| JobsList::mock(&jobs, cx));
                let keys = list.read(cx).focus_handle().clone();
                let screen = cx.new(|cx| JobsScreen::new(list, None, window, cx));
                let focus = Focus { keys, within: screen.read(cx).within().clone() };
                window.focus(&focus.keys, cx);
                let stood = cx.new(|cx| super::stand_in::JobHeadStandIn::new(screen.into(), cx));
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
    // The last one drawn: an element the list measured off screen comes first in the book.
    cx.try_global::<LabelBook>()
        .and_then(|book| book.drawn(window.window_handle().window_id()).iter().rev().find(|drawn| drawn.role == role && drawn.name == step.target.name).cloned())
        .ok_or_else(|| {
            let seen: Vec<String> = cx.try_global::<LabelBook>().map(|b| b.drawn(window.window_handle().window_id()).iter().filter(|d| d.role == role).map(|d| d.name.to_string()).collect()).unwrap_or_default();
            Missed(format!("no {} named \"{}\"; drawn: {:?}", step.target.role, step.target.name, seen))
        })
}

/// Do what reaching `step` does. A press needs its target drawn first; a key goes to the screen.
pub fn perform(step: &Step, focus: &Focus, window: &mut Window, cx: &mut App) -> Result<(), Missed> {
    match step.verb {
        Verb::Look => Ok(()),
        // The walk window builds the screen again before it reaches here (`WalkWindow::play_to`).
        Verb::Fresh => Ok(()),
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
            let at = find(step, window, cx)?.bounds.center();
            press(MouseButton::Left, at, window, cx);
            Ok(())
        }
        Verb::Gone => {
            // The first drawn, which is the head's where the thread draws a twin below it.
            let role = role_named(step.target.role).ok_or_else(|| Missed(format!("no role named {}", step.target.role)))?;
            let found = cx
                .try_global::<LabelBook>()
                .and_then(|book| book.drawn(window.window_handle().window_id()).iter().find(|drawn| drawn.role == role && drawn.name == step.target.name).cloned());
            let Some(found) = found else { return find(step, window, cx).map(|_| ()) };
            GONE_FROM.store(drawn_count(step, window, cx), std::sync::atomic::Ordering::Relaxed);
            press(MouseButton::Left, found.bounds.center(), window, cx);
            Ok(())
        }
        Verb::RightPress(on) => {
            let book = cx.try_global::<LabelBook>();
            let found = |word: &str| {
                let role = role_named(word).expect("a role the walk names");
                book.and_then(|book| book.find(window.window_handle().window_id(), role, on)).cloned()
            };
            let drawn = found("row").or_else(|| found("button")).ok_or_else(|| Missed(format!("no row or button named \"{on}\" to press")))?;
            press(MouseButton::Right, drawn.bounds.center(), window, cx);
            Ok(())
        }
        Verb::Type(text) => {
            // The press focuses the box; each letter then goes in as the platform's input method would.
            let at = find(step, window, cx)?.bounds.center();
            press(MouseButton::Left, at, window, cx);
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
            press(MouseButton::Left, drawn.bounds.center(), window, cx);
            Ok(())
        }
        Verb::OpenIn(article, on) => {
            let id = window.window_handle().window_id();
            let drawn = cx.try_global::<LabelBook>().map(|book| book.drawn(id).to_vec()).unwrap_or_default();
            let card = drawn
                .iter()
                .rev()
                .find(|one| one.role == gpui::Role::Article && one.name == article)
                .ok_or_else(|| Missed(format!("no article named \"{article}\"")))?;
            let button = drawn
                .iter()
                .rev()
                .find(|one| one.role == gpui::Role::Button && one.name == on && card.bounds.contains(&one.bounds.center()))
                .ok_or_else(|| Missed(format!("no button named \"{on}\" in \"{article}\"")))?;
            press(MouseButton::Left, button.bounds.center(), window, cx);
            Ok(())
        }
        Verb::RightOpen(role, on) => {
            let role = role_named(role).ok_or_else(|| Missed(format!("no role named {role}")))?;
            // The last one drawn: an element the list measured off screen comes first in the book.
            let drawn = cx
                .try_global::<LabelBook>()
                .and_then(|book| book.drawn(window.window_handle().window_id()).iter().rev().find(|drawn| drawn.role == role && drawn.name == on).cloned())
                .ok_or_else(|| Missed(format!("nothing named \"{on}\" to press")))?;
            press(MouseButton::Right, drawn.bounds.center(), window, cx);
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
                    position: drawn.bounds.center(),
                    delta: gpui::ScrollDelta::Pixels(point(px(0.), px(-(by as f32)))),
                    modifiers: Modifiers::default(),
                    touch_phase: gpui::TouchPhase::Moved,
                    source: gpui::ScrollSource::default(),
                }),
                cx,
            );
            Ok(())
        }
        Verb::Select(role, on) => {
            let role = role_named(role).ok_or_else(|| Missed(format!("no role named {role}")))?;
            // The last one drawn: an element the list measured off screen comes first in the book.
            let drawn = cx
                .try_global::<LabelBook>()
                .and_then(|book| book.drawn(window.window_handle().window_id()).iter().rev().find(|drawn| drawn.role == role && drawn.name == on).cloned())
                .ok_or_else(|| Missed(format!("nothing named \"{on}\" to select in")))?;
            let line = drawn.bounds.bottom() - px(16.);
            let from = point(drawn.bounds.left() + px(3.), line);
            let to = point(drawn.bounds.left() + drawn.bounds.size.width / 3., line);
            window.dispatch_event(
                PlatformInput::MouseDown(MouseDownEvent { button: MouseButton::Left, position: from, modifiers: Modifiers::default(), click_count: 1, first_mouse: false }),
                cx,
            );
            for step in 1..=6 {
                let at = point(from.x + (to.x - from.x) * (step as f32 / 6.), from.y);
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
            crate::titlebar::scenes::apply(name, cx).map_err(Missed)
        }
        Verb::PressOn(role, name) => {
            let role = role_named(role).ok_or_else(|| Missed(format!("no role named {role}")))?;
            let at = cx
                .try_global::<LabelBook>()
                .and_then(|book| book.find(window.window_handle().window_id(), role, name))
                .map(|drawn| drawn.bounds.center())
                .ok_or_else(|| Missed(format!("nothing named \"{name}\" to press")))?;
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
            // The menu is drawn, and holds the keys, before each key is sent, as a person's second press would find it.
            window.defer(cx, |window, cx| {
                window.draw(cx).clear(cx);
                if let Ok(down) = Keystroke::parse("down") {
                    window.dispatch_keystroke(down, cx);
                }
                window.defer(cx, |window, cx| {
                    window.draw(cx).clear(cx);
                    if let Ok(enter) = Keystroke::parse("enter") {
                        window.dispatch_keystroke(enter, cx);
                    }
                });
            });
            Ok(())
        }
        Verb::Answer(name) => {
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
        Verb::Hover => {
            let at = find(step, window, cx)?.bounds.center();
            window.dispatch_event(
                PlatformInput::MouseMove(MouseMoveEvent { position: at, pressed_button: None, modifiers: Modifiers::default() }),
                cx,
            );
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
