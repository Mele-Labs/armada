//! The prototype drawn headless, each state written as a picture and the time from markup to a
//! drawn frame measured, so a Job looks at it and times it with no window on the owner's screen.
//! A function and not a test because the Mac platform is made on the main thread, which a test
//! never runs on (`features/shots.rs` is the same).

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::{px, size, AnyWindowHandle, AppContext, Entity, HeadlessAppContext};

use super::window::PrototypeWindow;
use super::Source;
use crate::dispatch::face::Offered;

/// The sample scene as the mock Job saved it.
fn sample() -> Result<Source, String> {
    let artifact = crate::mock_parts::prototype();
    artifact
        .parts
        .iter()
        .find_map(|part| match &part.content {
            ipc::PartContent::Prototype(prototype) => Some(Source::from(prototype)),
            _ => None,
        })
        .ok_or_else(|| "the mock artifact holds no prototype".to_string())
}

pub(crate) fn draw(cx: &mut HeadlessAppContext, handle: AnyWindowHandle, times: usize) -> Result<(), String> {
    for _ in 0..times {
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
    }
    Ok(())
}

fn millis(at: Duration) -> f64 {
    at.as_secs_f64() * 1000.
}

fn list(times: &[f64]) -> String {
    times.iter().map(|ms| format!("{ms:.1}")).collect::<Vec<_>>().join(" ")
}

/// One picture of each state, one with the note capture on, and the timing, which is printed.
pub fn run(dir: &Path, runs: usize) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(crate::assets::Assets), || gpui_platform::current_headless_renderer());
    cx.update(crate::init_app);
    cx.update(|cx| cx.set_reduce_motion(true));
    let source = sample()?;
    let held: Rc<RefCell<Option<Entity<PrototypeWindow>>>> = Rc::default();
    let keep = held.clone();
    let first = source.clone();
    let handle: AnyWindowHandle = cx
        .open_window(size(px(960.), px(720.)), move |window, cx| {
            let opened = cx.new(|cx| PrototypeWindow::new(first, Offered { id: "mock-01".into(), title: "Edit the merge line".into() }, None, window, cx));
            *keep.borrow_mut() = Some(opened.clone());
            opened
        })
        .map_err(|why| why.to_string())?
        .into();
    let opened = held.borrow().clone().ok_or("the window did not open")?;
    cx.update_window(handle, |_, window, _| window.activate_window()).map_err(|why| why.to_string())?;
    draw(&mut cx, handle, 3)?;
    let shoot = |cx: &mut HeadlessAppContext, name: &str| -> Result<(), String> {
        let picture = cx.capture_screenshot(handle).map_err(|why| why.to_string())?;
        let path = dir.join(format!("{name}.png"));
        picture.save(&path).map_err(|why| format!("{}: {why}", path.display()))
    };
    shoot(&mut cx, "prototype-window-list")?;

    let view = cx.update_window(handle, |_, _, cx| opened.read(cx).view().clone()).map_err(|why| why.to_string())?;
    let layer = cx.update_window(handle, |_, _, cx| opened.read(cx).layer().clone()).map_err(|why| why.to_string())?;
    cx.update_window(handle, |_, _, cx| view.update(cx, |view, cx| view.go("detail", cx))).map_err(|why| why.to_string())?;
    draw(&mut cx, handle, 3)?;
    shoot(&mut cx, "prototype-window-detail")?;

    cx.update_window(handle, |_, window, cx| layer.update(cx, |layer, cx| layer.toggle(window, cx))).map_err(|why| why.to_string())?;
    draw(&mut cx, handle, 3)?;
    shoot(&mut cx, "prototype-window-note-layer")?;
    cx.update_window(handle, |_, window, cx| layer.update(cx, |layer, cx| layer.toggle(window, cx))).map_err(|why| why.to_string())?;
    cx.update_window(handle, |_, _, cx| view.update(cx, |view, cx| view.go("list", cx))).map_err(|why| why.to_string())?;
    draw(&mut cx, handle, 3)?;

    // Markup to a drawn frame: the saved file read again (parse, fixtures decoded through ipc, the check
    // of every part), then the frame. A comment appended to the markup makes each run a different save
    // and changes nothing drawn.
    let mut reads = Vec::new();
    let mut frames = Vec::new();
    for run in 0..runs {
        let mut next = source.clone();
        next.markup.push_str(&format!("\n<!-- save {run} -->"));
        let started = Instant::now();
        let mut read = Duration::ZERO;
        cx.update_window(handle, |_, _, cx| {
            view.update(cx, |view, cx| view.set_source(next, cx));
            read = started.elapsed();
        })
        .map_err(|why| why.to_string())?;
        draw(&mut cx, handle, 1)?;
        reads.push(millis(read));
        frames.push(millis(started.elapsed()));
    }
    crate::log::Note::info("a prototype scene drawn headless")
        .field("runs", runs)
        .field("read_ms", list(&reads))
        .field("markup_to_frame_ms", list(&frames))
        .emit();
    Ok(())
}
