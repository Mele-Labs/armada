//! `--walk <name> --check`: play the walk window step by step with no window on screen, and fail
//! on the first step that faults or whose target is not drawn. A Job runs it before it shows the
//! walk. `--shots <dir>` also writes each step as a picture, which is how a Job looks at a native
//! screen without putting a window in front of the owner.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{px, size, AppContext, Entity, HeadlessAppContext};

use super::window::{Next, Take, WalkWindow};
use super::Walk;

/// Play `walk` to its end. `Ok` says every step was reached and its target drawn.
pub fn run(walk: &'static Walk, takes: Vec<Take>, shots: Option<PathBuf>) -> Result<(), String> {
    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(crate::assets::Assets), || {
        gpui_platform::current_headless_renderer()
    });
    cx.update(crate::init_app);
    // The window is taken out of the app while it is updated, so its root is read through this.
    let held: Rc<RefCell<Option<Entity<WalkWindow>>>> = Rc::default();
    let keep = held.clone();
    let handle = cx
        .open_window(size(px(1100.), px(760.)), move |window, cx| {
            let walking = cx.new(|cx| WalkWindow::new(walk, None, takes, window, cx));
            *keep.borrow_mut() = Some(walking.clone());
            walking
        })
        .map_err(|why| why.to_string())?;
    let walking = held.borrow().clone().ok_or("the walk window did not open")?;
    if let Some(dir) = &shots {
        std::fs::create_dir_all(dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    }
    for at in 0..walk.steps.len() {
        // A window on screen draws every frame; here a step's callbacks (a picture taken two frames
        // after a press) each need a draw of their own.
        for _ in 0..3 {
            cx.run_until_parked();
            cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
        }
        cx.run_until_parked();
        let read = cx
            .update_window(handle.into(), |_, window, cx| {
                window.draw(cx).clear(cx);
                walking.read(cx).reading(window, cx)
            })
            .map_err(|why| why.to_string())?;
        if read.at != at {
            return Err(format!("step {}: the walk stopped at step {}{}", at + 1, read.at + 1, read.fault.map(|why| format!(": {why}")).unwrap_or_default()));
        }
        if let Some(why) = read.fault.or(read.missing) {
            return Err(format!("step {}: {why}", at + 1));
        }
        if let Some(dir) = &shots {
            let picture = cx.capture_screenshot(handle.into()).map_err(|why| why.to_string())?;
            let path = dir.join(format!("{}-{:02}.png", walk.name, at + 1));
            picture.save(&path).map_err(|why| format!("{}: {why}", path.display()))?;
        }
        if at + 1 < walk.steps.len() {
            cx.update_window(handle.into(), |_, window, cx| window.dispatch_action(Box::new(Next), cx)).map_err(|why| why.to_string())?;
        }
    }
    Ok(())
}
