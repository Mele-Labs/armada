//! The Story tab drawn headless at two widths and written as pictures, so a Job looks at it with no window on the owner's
//! screen. A function and not a test because the Mac platform is made on the main thread, which a test never runs on;
//! `kind_chip::shots` is the same.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{div, prelude::*, px, size, AppContext, Entity, HeadlessAppContext};

use super::mock::{self, JOB};
use super::StoryTab;
use crate::look::{token, ActiveLook as _};

/// What the window's shell gives the tab: the ground it is drawn on.
struct Frame(Entity<StoryTab>);

impl gpui::Render for Frame {
    fn render(&mut self, _: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        crate::labels::label_root(div().size_full().bg(look.color(token::BG_BASE)).text_color(look.color(token::FG_DEFAULT)).child(self.0.clone()))
    }
}

pub fn shoot(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(crate::assets::Assets), || gpui_platform::current_headless_renderer());
    cx.update(crate::init_app);
    cx.update(|cx| cx.set_reduce_motion(true));
    for (name, wide) in [("head-1400", 1400.), ("head-820", 820.)] {
        let held: Rc<RefCell<Option<Entity<StoryTab>>>> = Rc::default();
        let keep = held.clone();
        let handle = cx
            .open_window(size(px(wide), px(900.)), move |window, cx| {
                let tab = cx.new(|cx| StoryTab::mock(JOB.into(), false, window, cx));
                *keep.borrow_mut() = Some(tab.clone());
                cx.new(|_| Frame(tab))
            })
            .map_err(|why| why.to_string())?;
        let tab = held.borrow().clone().ok_or("the tab was not made")?;
        cx.update_window(handle.into(), |_, window, cx| {
            window.activate_window();
            tab.update(cx, |tab, cx| {
                if let super::Load::Read(story) = &mut tab.story {
                    // The owner's ask, with a title that wraps, and his words as they were sent, with their markdown.
                    story.asks[1].ask.title = "Schedule work or set up watcher scripts outside jobs, so that a repository can be looked at on a timetable".into();
                    story.asks[1].ask.said = vec![mock::words("row-long", ipc::MessageKind::Revision, "**Read the whole of the watcher design** and decided that a timetable belongs to the repository, not to the Job that first asked for one", 17)];
                }
                tab.rebuild(cx);
            });
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
