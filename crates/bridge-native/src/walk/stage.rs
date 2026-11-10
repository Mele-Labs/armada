//! The screen a walk plays on, and playing one step on it. The window and the check both play
//! through here, so what the check passes is what the owner sees.

use gpui::{
    point, AnyView, App, AppContext, Entity, FocusHandle, Keystroke, Modifiers, MouseButton, MouseDownEvent,
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
                let jobs = cx.new(|cx| JobsScreen::new(list, cx));
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
                let screen = cx.new(|cx| JobsScreen::new(list, cx));
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
                let jobs = cx.new(|cx| JobsScreen::new(list, cx));
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
                let screen = cx.new(|cx| JobsScreen::new(list, cx));
                let focus = Focus { keys, within: screen.read(cx).within().clone() };
                window.focus(&focus.keys, cx);
                let stood = cx.new(|cx| super::stand_in::JobHeadStandIn::new(screen.into(), cx));
                Ok(Stage { view: stood.into(), focus })
            }
        }
    }
}

/// What a step could not do, in the words the check prints.
#[derive(Debug)]
pub struct Missed(pub String);

/// The element a step aims at, as the window last drew it.
pub fn find(step: &Step, window: &Window, cx: &App) -> Result<Drawn, Missed> {
    let role = role_named(step.target.role).ok_or_else(|| Missed(format!("no role named {}", step.target.role)))?;
    cx.try_global::<LabelBook>()
        .and_then(|book| book.find(window.window_handle().window_id(), role, step.target.name))
        .cloned()
        .ok_or_else(|| Missed(format!("no {} named \"{}\"", step.target.role, step.target.name)))
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
        Verb::RightPress(on) => {
            let role = role_named("row").expect("a row is a role");
            let drawn = cx
                .try_global::<LabelBook>()
                .and_then(|book| book.find(window.window_handle().window_id(), role, on))
                .cloned()
                .ok_or_else(|| Missed(format!("no row named \"{on}\" to press")))?;
            press(MouseButton::Right, drawn.bounds.center(), window, cx);
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
        Verb::Type(text) => {
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
