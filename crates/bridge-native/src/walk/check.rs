//! `--walk <name> --check`: play the walk window step by step with no window on screen, and fail
//! on the first step that faults or whose target is not drawn. A Job runs it before it shows the
//! walk. `--shots <dir>` also writes each step as a picture, which is how a Job looks at a native
//! screen without putting a window in front of the owner; `--hold <ms>` takes each picture that long
//! into the step, so a moving thing is caught mid-way. `--pace <ms>` moves the clock that far between
//! one Next and the next, as a person reading each step does, and fails a step still playing then:
//! his Next would do nothing.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{px, size, AppContext, Bounds, Entity, HeadlessAppContext, Pixels};
use image::RgbaImage;

use super::window::{Next, Take, WalkWindow};
use super::Walk;

/// Play `walk` to its end. `Ok` says every step was reached and its target drawn.
pub fn run(walk: &'static Walk, takes: Vec<Take>, shots: Option<PathBuf>, hold: Option<u64>, pace: Option<u64>, width: Option<f32>, height: Option<f32>) -> Result<(), String> {
    play(walk, takes, shots, hold, pace, width, height, None)
}

/// What a step's picture was taken of: its place in the walk, the picture, and where the screen sits in the window.
pub type Seen<'a> = &'a mut dyn FnMut(usize, &RgbaImage, Option<Bounds<Pixels>>);

/// [`run`], handing each step's picture to `seen` as well: the shot check reads them instead of a folder.
#[allow(clippy::too_many_arguments)]
pub fn play(walk: &'static Walk, takes: Vec<Take>, shots: Option<PathBuf>, hold: Option<u64>, pace: Option<u64>, width: Option<f32>, height: Option<f32>, mut seen: Option<Seen<'_>>) -> Result<(), String> {
    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(crate::assets::Assets), || {
        gpui_platform::current_headless_renderer()
    });
    cx.update(crate::init_app);
    // The check's clock does not move between draws, so a picture would hold a turn's first frame
    // forever; with motion reduced each one-shot animation draws where it ends.
    cx.update(|cx| cx.set_reduce_motion(true));
    // The window is taken out of the app while it is updated, so its root is read through this.
    let held: Rc<RefCell<Option<Entity<WalkWindow>>>> = Rc::default();
    let keep = held.clone();
    let handle = cx
        .open_window(size(px(width.unwrap_or(1100.)), px(height.unwrap_or(820.))), move |window, cx| {
            let walking = cx.new(|cx| WalkWindow::new(walk, None, takes, window, cx));
            *keep.borrow_mut() = Some(walking.clone());
            walking
        })
        .map_err(|why| why.to_string())?;
    let walking = held.borrow().clone().ok_or("the walk window did not open")?;
    // A window nobody is in is not active, and a focus listener hears nothing in one: the check's window is the one in front.
    cx.update_window(handle.into(), |_, window, _| window.activate_window()).map_err(|why| why.to_string())?;
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
        settle_pages(&mut cx, handle.into())?;
        // A person reads the step for this long, and his Next does nothing while it still plays.
        if let Some(ms) = pace {
            for _ in 0..ms.div_ceil(100) {
                cx.advance_clock(std::time::Duration::from_millis(100));
                cx.run_until_parked();
                cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
            }
            if walking.read_with(&cx, |walking, _| walking.playing()) {
                return Err(format!("step {}: still playing {ms} ms after Next, so a person's Next does nothing", at + 1));
            }
        }
        // The check's clock is its own: a step that lets time go by is played to its end by moving it on.
        for _ in 0..600 {
            if !walking.read_with(&cx, |walking, _| walking.playing()) {
                break;
            }
            cx.advance_clock(std::time::Duration::from_millis(100));
            // A step that waits on a page waits on WebKit, which answers on the run loop.
            if crate::web::in_use() {
                crate::web::pump(0.01);
            }
            cx.run_until_parked();
            cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
        }
        // A tooltip comes up after the pointer has rested; the check's clock is moved past that, or a hover is shot before it.
        if matches!(walk.steps[at].verb, super::Verb::Hover) {
            cx.advance_clock(std::time::Duration::from_millis(700));
            cx.run_until_parked();
            cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
        }
        settle_pages(&mut cx, handle.into())?;
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
        if shots.is_some() || seen.is_some() {
            // An animation reads the wall clock, so a picture taken at once always catches its first frame.
            if let Some(ms) = hold {
                std::thread::sleep(std::time::Duration::from_millis(ms));
                cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
            }
            let picture = cx.capture_screenshot(handle.into()).map_err(|why| why.to_string())?;
            if let Some(dir) = &shots {
                let path = dir.join(format!("{}-{:02}.png", walk.name, at + 1));
                picture.save(&path).map_err(|why| format!("{}: {why}", path.display()))?;
            }
            if let Some(seen) = seen.as_mut() {
                seen(at, &picture, walking.read_with(&cx, |walking, _| walking.stage()));
            }
        }
        if at + 1 < walk.steps.len() {
            cx.update_window(handle.into(), |_, window, cx| window.dispatch_action(Box::new(Next), cx)).map_err(|why| why.to_string())?;
        }
    }
    Ok(())
}


/// A web page answers on the main run loop, which nothing turns in this window: it is turned here for as long as
/// WebKit owes an answer, a few quiet turns in a row so what an answer sets off is seen to its end.
fn settle_pages(cx: &mut HeadlessAppContext, handle: gpui::AnyWindowHandle) -> Result<(), String> {
    if !crate::web::in_use() {
        return Ok(());
    }
    let mut quiet = 0;
    for _ in 0..1000 {
        crate::web::pump(0.01);
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
        if crate::web::pending() {
            quiet = 0;
        } else {
            quiet += 1;
            if quiet >= 5 {
                break;
            }
        }
    }
    Ok(())
}
