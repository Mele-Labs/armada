//! The screen drawn headless at two widths, each written as a picture, so a Job looks at it with no window on
//! the owner's screen. It is a function and not a test because the Mac platform is made on the main thread, which
//! a test never runs on; `walk::check` is the same drawing for a walk.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{px, size, AppContext, HeadlessAppContext};

use super::FeaturesScreen;

pub fn shoot(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(crate::assets::Assets), || gpui_platform::current_headless_renderer());
    cx.update(crate::init_app);
    cx.update(|cx| cx.set_reduce_motion(true));
    for (name, wide) in [("features-1400", 1400.), ("features-820", 820.)] {
        let held: Rc<RefCell<Option<gpui::Entity<FeaturesScreen>>>> = Rc::default();
        let keep = held.clone();
        let handle = cx
            .open_window(size(px(wide), px(820.)), move |window, cx| {
                let screen = cx.new(|cx| FeaturesScreen::mock(window, cx));
                *keep.borrow_mut() = Some(screen.clone());
                screen
            })
            .map_err(|why| why.to_string())?;
        let screen = held.borrow().clone().ok_or("the screen was not made")?;
        cx.update_window(handle.into(), |_, window, cx| {
            window.activate_window();
            screen.update(cx, |screen, cx| screen.enter(window, cx));
        })
        .map_err(|why| why.to_string())?;
        // Enter on the second feature, then one row down: the page is open on one and the keys are on the next.
        cx.update_window(handle.into(), |_, _, cx| {
            screen.update(cx, |screen, cx| {
                screen.step(1, cx);
                screen.open_cursor(cx);
                screen.step(1, cx);
            })
        })
        .map_err(|why| why.to_string())?;
        for _ in 0..3 {
            cx.run_until_parked();
            cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
        }
        let picture = cx.capture_screenshot(handle.into()).map_err(|why| why.to_string())?;
        let path = dir.join(format!("{name}.png"));
        picture.save(&path).map_err(|why| format!("{}: {why}", path.display()))?;
    }
    Ok(())
}
