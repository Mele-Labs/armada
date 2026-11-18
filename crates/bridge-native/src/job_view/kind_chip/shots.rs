//! A Job's thread with the kind chips, drawn headless at 1400 wide and written as pictures: the chips of each kind, the
//! first menu open, and the second. A function and not a test because the Mac platform is made on the main thread, which a
//! test never runs on; `features::shots` is the same.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{div, prelude::*, px, size, AppContext, Entity, HeadlessAppContext};

use super::model::Choice;
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
    for (name, step) in [("kind-chips-1400", 0), ("kind-menu-1400", 1), ("kind-asks-1400", 2), ("kind-rest-1400", 3)] {
        let held: Rc<RefCell<Option<(Entity<JobView>, Entity<crate::jobs::JobsList>)>>> = Rc::default();
        let keep = held.clone();
        let jobs = crate::mock::jobs("job-view")?;
        let threads = crate::mock::threads("job-view")?;
        let handle = cx
            .open_window(size(px(1400.), px(1500.)), move |window, cx| {
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
        cx.update_window(handle.into(), |_, _, cx| super::store::shown(JOB, cx))
        .map_err(|why| why.to_string())?;
        let draw = |cx: &mut HeadlessAppContext| -> Result<(), String> {
            for _ in 0..3 {
                cx.run_until_parked();
                cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
            }
            Ok(())
        };
        draw(&mut cx)?;
        if step == 3 {
            // The kinds the other pictures do not show: an idea, an answer and a revision, sent as he would send them.
            cx.update_window(handle.into(), |_, _, cx| {
                for (row, choice, ask) in [("m-321", Choice::Ask(ipc::AskKind::Idea), None), ("m-326", Choice::Answer, Some("ask-2")), ("m-331", Choice::Revision, Some("ask-1"))] {
                    if let Some(set) = super::model::request(JOB, row, choice, ask) {
                        super::store::send(JOB, set, None, cx);
                    }
                }
            })
            .map_err(|why| why.to_string())?;
            draw(&mut cx)?;
        }
        if step > 0 && step < 3 {
            // The menu of his newest message, as the key opens it; the second step goes on to the asks of a Revision.
            cx.update_window(handle.into(), |_, window, cx| view.update(cx, |view, cx| view.kind_key(window, cx))).map_err(|why| why.to_string())?;
            draw(&mut cx)?;
        }
        if step == 2 {
            cx.update_window(handle.into(), |_, window, cx| {
                view.update(cx, |view, cx| {
                    let row = "m-331";
                    let at = view.pages.get(JOB).and_then(|page| page.keys.spot_of(&super::spot_key(row))).map(|spot| spot.bounds.get().bottom_left()).unwrap_or_default();
                    view.kind_picked(JOB, row, Choice::Revision.id(), at, None, window, cx)
                })
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
