//! A Job's thread with the contents rail, drawn headless at 1400 wide and written as pictures: the rail at rest with the
//! second ask in view, and the list open beside it. A function and not a test because the Mac platform is made on the main
//! thread, which a test never runs on; `kind_chip::shots` is the same.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{div, prelude::*, px, size, AppContext, Entity, HeadlessAppContext};

use super::Rail;
use crate::job_view::kind_chip::store;
use crate::job_view::JobView;
use crate::look::{token, ActiveLook as _};

const JOB: &str = "mock-02";

/// What the window's shell gives the Job view: the ground it is drawn on.
struct Frame(Entity<JobView>);

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
    for (name, wide, open) in [("rail-rest-1400", 1400., false), ("rail-list-1400", 1400., true), ("rail-rest-820", 820., false), ("rail-list-820", 820., true)] {
        let held: Rc<RefCell<Option<(Entity<JobView>, Entity<crate::jobs::JobsList>)>>> = Rc::default();
        let keep = held.clone();
        let jobs = crate::mock::jobs("job-view")?;
        let threads = crate::mock::threads("job-view")?;
        let handle = cx
            .open_window(size(px(wide), px(900.)), move |window, cx| {
                let list = cx.new(|cx| crate::jobs::JobsList::mock(&jobs, cx));
                let view = cx.new(|cx| JobView::new(list.clone(), Some(threads), window, cx));
                *keep.borrow_mut() = Some((view.clone(), list));
                cx.new(|_| Frame(view))
            })
            .map_err(|why| why.to_string())?;
        let (view, list) = held.borrow().clone().ok_or("the view was not made")?;
        cx.update_window(handle.into(), |_, window, cx| {
            window.activate_window();
            list.update(cx, |list, cx| list.open(JOB.to_string().into(), cx));
        })
        .map_err(|why| why.to_string())?;
        cx.run_until_parked();
        cx.update_window(handle.into(), |_, _, cx| store::shown(JOB, cx)).map_err(|why| why.to_string())?;
        cx.update_window(handle.into(), |_, window, cx| view.update(cx, |view, cx| drop(view.story_of(JOB, window, cx)))).map_err(|why| why.to_string())?;
        let draw = |cx: &mut HeadlessAppContext| -> Result<(), String> {
            for _ in 0..3 {
                cx.run_until_parked();
                cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
            }
            Ok(())
        };
        draw(&mut cx)?;
        // The second ask at the top of the thread, so it is the one in view.
        cx.update_window(handle.into(), |_, _, cx| {
            view.update(cx, |view, cx| {
                let id: gpui::SharedString = JOB.into();
                let (marks, _) = view.rail_marks(&id, cx);
                if let (Some(page), Some(item)) = (view.pages.get(&id), marks.iter().filter_map(|bar| bar.at).nth(1)) {
                    page.list.scroll_to(gpui::ListOffset { item_ix: page.entry(item), offset_in_item: px(0.) });
                }
                cx.notify();
            })
        })
        .map_err(|why| why.to_string())?;
        draw(&mut cx)?;
        {
            cx.update_window(handle.into(), |_, _, cx| {
                cx.global_mut::<Rail>().hover = open;
                view.update(cx, |_, cx| cx.notify());
            })
            .map_err(|why| why.to_string())?;
            draw(&mut cx)?;
        }
        let picture = cx.capture_screenshot(handle.into()).map_err(|why| why.to_string())?;
        let path = dir.join(format!("{name}.png"));
        picture.save(&path).map_err(|why| format!("{}: {why}", path.display()))?;
    }
    Ok(())
}
