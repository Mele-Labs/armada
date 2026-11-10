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
            Screen::Jobs => {
                let jobs = crate::mock::jobs(walk.scenario)?;
                let list: Entity<JobsList> = cx.new(|cx| JobsList::mock(&jobs, cx));
                let keys = list.read(cx).focus_handle().clone();
                let threads = crate::mock::threads(walk.scenario)?;
                let screen = cx.new(|cx| JobsScreen::new(list, Some(threads), window, cx));
                let focus = Focus { keys, within: screen.read(cx).within().clone() };
                window.focus(&focus.keys, cx);
                Ok(Stage { view: screen.into(), focus })
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
        Verb::Key(key) => {
            let keystroke = Keystroke::parse(key).map_err(|why| Missed(format!("the key {key} does not parse: {why}")))?;
            if !focus.within.contains_focused(window, cx) {
                window.focus(&focus.keys, cx);
            }
            window.dispatch_keystroke(keystroke, cx);
            Ok(())
        }
        Verb::Press => {
            let at = find(step, window, cx)?.bounds.center();
            press(MouseButton::Left, at, window, cx);
            Ok(())
        }
        Verb::RightPress(on) => {
            // A Jobs row, else a button such as one of the review sheet's items.
            let window_id = window.window_handle().window_id();
            let drawn = ["row", "button"]
                .into_iter()
                .filter_map(role_named)
                .find_map(|role| cx.try_global::<LabelBook>().and_then(|book| book.find(window_id, role, on)))
                .cloned()
                .ok_or_else(|| Missed(format!("no row named \"{on}\" to press")))?;
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

fn press(button: MouseButton, at: Point<Pixels>, window: &mut Window, cx: &mut App) {
    let at = point(at.x, at.y);
    window.dispatch_event(
        PlatformInput::MouseDown(MouseDownEvent { button, position: at, modifiers: Modifiers::default(), click_count: 1, first_mouse: false }),
        cx,
    );
    window.dispatch_event(PlatformInput::MouseUp(MouseUpEvent { button, position: at, modifiers: Modifiers::default(), click_count: 1 }), cx);
}
