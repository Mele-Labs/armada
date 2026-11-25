//! The System block drawn headless at 1400 and 900 wide and written as pictures, so a Job looks at it with no window on the
//! owner's screen: the board at level one, a part selected with its panel, a part zoomed into, a feature's path, a part the
//! librarian added, and the Code tree with a folder open. A function and not a test because the Mac platform is made on the
//! main thread, which a test never runs on (`features/shots.rs` is the same).

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{div, prelude::*, px, size, AppContext, Entity, HeadlessAppContext, Window};

use super::{tree, SystemBlock};
use crate::look::{token, ActiveLook as _};
use crate::project_board::ProjectScreen;

#[derive(Clone, Copy)]
enum State {
    LevelOne,
    BridgeSelected,
    BridgeJobs,
    DashboardPath,
    Librarian,
    CodeTree,
}

const STATES: [(&str, State); 6] = [
    ("a-level-one", State::LevelOne),
    ("b-bridge-selected", State::BridgeSelected),
    ("c-bridge-jobs", State::BridgeJobs),
    ("d-dashboard-path", State::DashboardPath),
    ("e-librarian-bank", State::Librarian),
    ("f-code-tree", State::CodeTree),
];

/// What a window holds around a bare block: the ground the board would give it.
struct Ground(Entity<SystemBlock>);

impl gpui::Render for Ground {
    fn render(&mut self, _: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        crate::labels::label_root(div().size_full().p(look.length(token::SPACE_4)).bg(look.color(token::BG_BASE)).text_color(look.color(token::FG_DEFAULT)).child(self.0.clone()))
    }
}

pub fn shoot(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(crate::assets::Assets), || gpui_platform::current_headless_renderer());
    cx.update(crate::init_app);
    cx.update(|cx| cx.set_reduce_motion(true));
    for wide in [1400., 900.] {
        for (name, state) in STATES {
            let held: Rc<RefCell<Option<Entity<SystemBlock>>>> = Rc::default();
            let keep = held.clone();
            // The librarian's part is in the other project, so that picture is the bare block following it.
            let handle = if matches!(state, State::Librarian) {
                cx.open_window(size(px(wide), px(820.)), move |window, cx| {
                    let _ = window;
                    let block = cx.new(|cx| {
                        let mut block = SystemBlock::mock(cx);
                        block.follow("ynab", "auto-ynab", vec!["repo".into()], cx);
                        block
                    });
                    *keep.borrow_mut() = Some(block.clone());
                    cx.new(|_| Ground(block))
                })
                .map(gpui::AnyWindowHandle::from)
            } else {
                cx.open_window(size(px(wide), px(820.)), move |window, cx| {
                    let screen = cx.new(|cx| ProjectScreen::mock(None, window, cx));
                    *keep.borrow_mut() = Some(screen.read(cx).board.read(cx).system.clone());
                    screen.update(cx, |screen, cx| screen.enter(window, cx));
                    screen
                })
                .map(gpui::AnyWindowHandle::from)
            }
            .map_err(|why| why.to_string())?;
            let block = held.borrow().clone().ok_or("the block was not made")?;
            let draw = |cx: &mut HeadlessAppContext| -> Result<(), String> {
                for _ in 0..4 {
                    cx.run_until_parked();
                    cx.update_window(handle, |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
                }
                Ok(())
            };
            cx.update_window(handle, |_, window, _| window.activate_window()).map_err(|why| why.to_string())?;
            draw(&mut cx)?;
            cx.update_window(handle, |_, window, cx| {
                block.update(cx, |block, cx| match state {
                    State::LevelOne => {}
                    State::BridgeSelected => block.select(Some("bridge".into()), cx),
                    State::BridgeJobs => {
                        block.select(Some("bridge".into()), cx);
                        block.look_inside(window, cx);
                    }
                    State::DashboardPath => {
                        block.select(Some("bridge".into()), cx);
                        block.look_inside(window, cx);
                        block.follow_feature("dashboard", window, cx);
                    }
                    State::Librarian => block.select(Some("bank".into()), cx),
                    State::CodeTree => block.select(Some("bridge".into()), cx),
                })
            })
            .map_err(|why| why.to_string())?;
            draw(&mut cx)?;
            // The second step of a state, once what it asked has come: zoom's pick, the folder that opens.
            cx.update_window(handle, |_, _, cx| {
                block.update(cx, |block, cx| match state {
                    State::BridgeJobs => block.select(Some("b_jobs".into()), cx),
                    State::CodeTree => {
                        let first = block.answers.get("bridge").and_then(super::Read::ready).and_then(|answers| tree::rows(&answers.code, &HashMap::new(), &HashMap::new()).into_iter().next());
                        if let Some(row) = first {
                            block.toggle_folder("bridge", &row, cx);
                        }
                    }
                    _ => {}
                })
            })
            .map_err(|why| why.to_string())?;
            draw(&mut cx)?;
            let picture = cx.capture_screenshot(handle).map_err(|why| why.to_string())?;
            let path = dir.join(format!("native-{name}-{}.png", wide as u32));
            picture.save(&path).map_err(|why| format!("{}: {why}", path.display()))?;
        }
    }
    Ok(())
}
