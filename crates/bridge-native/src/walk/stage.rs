//! The screen a walk plays on, and playing one step on it. The window and the check both play
//! through here, so what the check passes is what the owner sees.

use gpui::{
    point, AnyView, App, AppContext, Entity, FocusHandle, Keystroke, Modifiers, MouseButton, MouseDownEvent,
    MouseUpEvent, PlatformInput, Window,
};

use super::{Screen, Step, Verb, Walk};
use crate::jobs::{JobsList, JobsScreen};
use crate::labels::{role_named, Drawn, LabelBook};

/// The screen a walk plays on, fresh from its scenario.
pub struct Stage {
    pub view: AnyView,
    pub focus: FocusHandle,
}

impl Stage {
    pub fn new(walk: &Walk, window: &mut Window, cx: &mut App) -> Result<Stage, String> {
        match walk.screen {
            Screen::Jobs => {
                let jobs = crate::mock::jobs(walk.scenario)?;
                let list: Entity<JobsList> = cx.new(|cx| JobsList::mock(&jobs, cx));
                let focus = list.read(cx).focus_handle().clone();
                window.focus(&focus, cx);
                let screen = cx.new(|_| JobsScreen { list });
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
    cx.try_global::<LabelBook>()
        .and_then(|book| book.find(window.window_handle().window_id(), role, step.target.name))
        .cloned()
        .ok_or_else(|| Missed(format!("no {} named \"{}\"", step.target.role, step.target.name)))
}

/// Do what reaching `step` does. A press needs its target drawn first; a key goes to the screen.
pub fn perform(step: &Step, focus: &FocusHandle, window: &mut Window, cx: &mut App) -> Result<(), Missed> {
    match step.verb {
        Verb::Look => Ok(()),
        Verb::Key(key) => {
            let keystroke = Keystroke::parse(key).map_err(|why| Missed(format!("the key {key} does not parse: {why}")))?;
            window.focus(focus, cx);
            window.dispatch_keystroke(keystroke, cx);
            Ok(())
        }
        Verb::Press => {
            let drawn = find(step, window, cx)?;
            let at = drawn.bounds.center();
            let at = point(at.x, at.y);
            window.dispatch_event(
                PlatformInput::MouseDown(MouseDownEvent {
                    button: MouseButton::Left,
                    position: at,
                    modifiers: Modifiers::default(),
                    click_count: 1,
                    first_mouse: false,
                }),
                cx,
            );
            window.dispatch_event(
                PlatformInput::MouseUp(MouseUpEvent { button: MouseButton::Left, position: at, modifiers: Modifiers::default(), click_count: 1 }),
                cx,
            );
            Ok(())
        }
    }
}
