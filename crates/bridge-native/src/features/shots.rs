//! The screen drawn headless at two widths, each written as a picture, so a Job looks at it with no window on
//! the owner's screen. It is a function and not a test because the Mac platform is made on the main thread, which
//! a test never runs on; `walk::check` is the same drawing for a walk.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{px, size, AppContext, HeadlessAppContext};

use super::page::Region;
use super::page_model::{Chip, Section};
use super::FeaturesScreen;

pub fn shoot(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(crate::assets::Assets), || gpui_platform::current_headless_renderer());
    cx.update(crate::init_app);
    cx.update(|cx| cx.set_reduce_motion(true));
    for (name, wide, section) in [("features-bugs-1400", 1400., Section::Bugs), ("features-bugs-900", 900., Section::Bugs), ("features-bugs-820", 820., Section::Bugs), ("features-changes-1400", 1400., Section::Changes), ("features-changes-900", 900., Section::Changes), ("features-ideas-1400", 1400., Section::Ideas)] {
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
        // Enter on the first feature, then the section: the page is open on the feature with the keys on its rows.
        cx.update_window(handle.into(), |_, window, cx| {
            screen.update(cx, |screen, cx| {
                screen.open_cursor(window, cx);
                if let Some(page) = screen.page.clone() {
                    page.update(cx, |page, cx| {
                        page.select(section, cx);
                        if section.is_asks() {
                            page.apply(Chip::All, cx);
                            page.focus_region(Region::Rows, window, cx);
                            page.step_row(2, cx);
                        }
                    });
                }
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

/// The Story tab of the mock Job at 1400 wide: the fixed ask unfolded and two folded, then a feature's page in the
/// overlay over it, then the overlay's Story layer.
pub fn shoot_story(dir: &Path) -> Result<(), String> {
    use crate::job_view::story::{mock::JOB, StoryTab};
    std::fs::create_dir_all(dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(crate::assets::Assets), || gpui_platform::current_headless_renderer());
    cx.update(crate::init_app);
    cx.update(|cx| cx.set_reduce_motion(true));
    for (name, step) in [("story-tab-1400", 0), ("story-overlay-page-1400", 1), ("story-overlay-story-1400", 2)] {
        let held: Rc<RefCell<Option<gpui::Entity<StoryTab>>>> = Rc::default();
        let keep = held.clone();
        let handle = cx
            .open_window(size(px(1400.), px(820.)), move |window, cx| {
                let tab = cx.new(|cx| StoryTab::mock(JOB.into(), false, window, cx));
                *keep.borrow_mut() = Some(tab.clone());
                tab
            })
            .map_err(|why| why.to_string())?;
        let tab = held.borrow().clone().ok_or("the tab was not made")?;
        cx.update_window(handle.into(), |_, window, cx| {
            window.activate_window();
            let focus = tab.read(cx).focus.clone();
            window.focus(&focus, cx);
            tab.update(cx, |tab, cx| {
                tab.put_ask("ask-3".into(), cx);
                if step >= 1 {
                    tab.open_feature("start-a-job".into(), window, cx);
                }
            });
            if step >= 2 {
                if let Some(overlay) = tab.read(cx).overlay().cloned() {
                    overlay.update(cx, |overlay, cx| overlay.push_story(ipc::SessionId::carried(JOB), "ask-3".into(), window, cx));
                }
            }
        })
        .map_err(|why| why.to_string())?;
        for _ in 0..4 {
            cx.run_until_parked();
            cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
        }
        let picture = cx.capture_screenshot(handle.into()).map_err(|why| why.to_string())?;
        let path = dir.join(format!("{name}.png"));
        picture.save(&path).map_err(|why| format!("{}: {why}", path.display()))?;
    }
    Ok(())
}

/// What the annotation layer draws over a screen: the Features tab stands in for a screen with a note on one of its rows.
struct Under {
    screen: gpui::Entity<FeaturesScreen>,
    layer: gpui::Entity<crate::annotate::Layer>,
}

impl gpui::Render for Under {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        use gpui::{div, prelude::*};
        div().size_full().relative().child(self.screen.clone()).child(self.layer.clone())
    }
}

/// A note's card with its feature chips over a screen at 1400 wide, then the feature's page beside it, then the page's
/// Story in the same panel.
pub fn shoot_beside(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(crate::assets::Assets), || gpui_platform::current_headless_renderer());
    cx.update(crate::init_app);
    cx.update(|cx| cx.set_reduce_motion(true));
    for (name, step) in [("note-chips-1400", 0), ("note-beside-1400", 1), ("note-beside-story-1400", 2)] {
        let held: Rc<RefCell<Option<(gpui::Entity<FeaturesScreen>, gpui::Entity<crate::annotate::Layer>)>>> = Rc::default();
        let keep = held.clone();
        let handle = cx
            .open_window(size(px(1400.), px(820.)), move |window, cx| {
                let screen = cx.new(|cx| FeaturesScreen::mock(window, cx));
                let layer = cx.new(|cx| {
                    let mut layer = crate::annotate::Layer::new(Rc::default(), Vec::new(), cx);
                    layer.screen = Some("Jobs".into());
                    layer
                });
                *keep.borrow_mut() = Some((screen.clone(), layer.clone()));
                cx.new(|_| Under { screen, layer })
            })
            .map_err(|why| why.to_string())?;
        let (screen, layer) = held.borrow().clone().ok_or("the screen was not made")?;
        cx.update_window(handle.into(), |_, window, cx| {
            window.activate_window();
            screen.update(cx, |screen, cx| screen.enter(window, cx));
            layer.update(cx, |layer, cx| {
                layer.feature_source(super::overlay::Source::Mock, cx);
                layer.toggle(window, cx);
                let note = serde_json::json!({
                    "id": "20261117-101500-feat", "status": "open", "text": "The row clips its title where a Job has two on it", "component": "JobRow", "owners": ["JobList"],
                    "ownersFrom": "parent", "selector": "div.row", "element": { "tag": "div", "text": "", "label": "Row \"Job list and live status\"" },
                    "screen": "Jobs", "layer": null, "location": "/", "scenario": null,
                    "source": { "file": "apps/desktop/src/renderer/src/jobs/JobRow.tsx", "line": 41 },
                    "box": { "x": 330, "y": 150, "width": 360, "height": 56 }, "window": { "width": 1400, "height": 820 },
                    "createdAt": "2026-11-17T10:15:00.000Z", "updatedAt": "2026-11-17T10:15:00.000Z",
                });
                layer.read_notes(vec![note], cx);
                layer.open_note(1, window, cx);
            });
        })
        .map_err(|why| why.to_string())?;
        for _ in 0..3 {
            cx.run_until_parked();
            cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
        }
        if step >= 1 {
            cx.update_window(handle.into(), |_, window, cx| {
                layer.update(cx, |layer, cx| layer.open_next_feature(window, cx));
            })
            .map_err(|why| why.to_string())?;
            for _ in 0..4 {
                cx.run_until_parked();
                cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
            }
        }
        if step >= 2 {
            cx.update_window(handle.into(), |_, window, cx| {
                if let Some(overlay) = layer.read(cx).features_overlay(cx) {
                    overlay.update(cx, |overlay, cx| overlay.push_story(ipc::SessionId::carried(crate::job_view::story::mock::JOB), "ask-3".into(), window, cx));
                }
            })
            .map_err(|why| why.to_string())?;
            for _ in 0..4 {
                cx.run_until_parked();
                cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
            }
        }
        let picture = cx.capture_screenshot(handle.into()).map_err(|why| why.to_string())?;
        let path = dir.join(format!("{name}.png"));
        picture.save(&path).map_err(|why| format!("{}: {why}", path.display()))?;
    }
    Ok(())
}

/// The whole window at 1400 wide with Features in front on the mock library, reached the way a press on the
/// strip's Features reaches it.
pub fn shoot_mounted(dir: &Path) -> Result<(), String> {
    use crate::jobs::{JobsList, JobsScreen};
    use crate::operator::screen::{Front, OperatorScreen};
    std::fs::create_dir_all(dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(crate::assets::Assets), || gpui_platform::current_headless_renderer());
    cx.update(crate::init_app);
    cx.update(|cx| cx.set_reduce_motion(true));
    let held: Rc<RefCell<Option<gpui::Entity<OperatorScreen>>>> = Rc::default();
    let keep = held.clone();
    let handle = cx
        .open_window(size(px(1400.), px(820.)), move |window, cx| {
            let records = crate::mock::jobs("jobs-many").expect("the fixture decodes");
            let list = cx.new(|cx| JobsList::mock(&records, cx));
            let jobs = cx.new(|cx| JobsScreen::new(list, None, window, cx));
            let screen = cx.new(|cx| OperatorScreen::mock_reviews(jobs, window, cx));
            *keep.borrow_mut() = Some(screen.clone());
            screen
        })
        .map_err(|why| why.to_string())?;
    let screen = held.borrow().clone().ok_or("the screen was not made")?;
    for _ in 0..2 {
        cx.run_until_parked();
        cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
    }
    cx.update_window(handle.into(), |_, window, cx| {
        window.activate_window();
        screen.update(cx, |screen, cx| screen.bring(Some(Front::Features), window, cx));
    })
    .map_err(|why| why.to_string())?;
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
    }
    let picture = cx.capture_screenshot(handle.into()).map_err(|why| why.to_string())?;
    let path = dir.join("features-mounted-1400.png");
    picture.save(&path).map_err(|why| format!("{}: {why}", path.display()))
}

/// The screen at 820 wide, where the list and the page take turns: the list with the row the keys are on, then the
/// page opened from it with its Back row.
pub fn shoot_narrow(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(crate::assets::Assets), || gpui_platform::current_headless_renderer());
    cx.update(crate::init_app);
    cx.update(|cx| cx.set_reduce_motion(true));
    for (name, open) in [("features-narrow-list-820", false), ("features-narrow-page-820", true)] {
        let held: Rc<RefCell<Option<gpui::Entity<FeaturesScreen>>>> = Rc::default();
        let keep = held.clone();
        let handle = cx
            .open_window(size(px(820.), px(820.)), move |window, cx| {
                let screen = cx.new(|cx| FeaturesScreen::mock(window, cx));
                *keep.borrow_mut() = Some(screen.clone());
                screen
            })
            .map_err(|why| why.to_string())?;
        let screen = held.borrow().clone().ok_or("the screen was not made")?;
        cx.update_window(handle.into(), |_, window, cx| {
            window.activate_window();
            screen.update(cx, |screen, cx| {
                screen.enter(window, cx);
                screen.step(2, cx);
                if open {
                    screen.open_cursor(window, cx);
                }
            });
        })
        .map_err(|why| why.to_string())?;
        for _ in 0..4 {
            cx.run_until_parked();
            cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
        }
        let picture = cx.capture_screenshot(handle.into()).map_err(|why| why.to_string())?;
        let path = dir.join(format!("{name}.png"));
        picture.save(&path).map_err(|why| format!("{}: {why}", path.display()))?;
    }
    Ok(())
}

/// The pending row, the Merge into picker, an area of nine and the split sheet at 1400 wide.
pub fn shoot_pending(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(crate::assets::Assets), || gpui_platform::current_headless_renderer());
    cx.update(crate::init_app);
    cx.update(|cx| cx.set_reduce_motion(true));
    for step in ["pending-row", "pending-confirm", "pending-merge-picker", "area-of-nine", "split-sheet"] {
        let held: Rc<RefCell<Option<gpui::Entity<FeaturesScreen>>>> = Rc::default();
        let keep = held.clone();
        let handle = cx
            .open_window(size(px(1400.), px(820.)), move |window, cx| {
                let screen = cx.new(|cx| FeaturesScreen::mock(window, cx));
                *keep.borrow_mut() = Some(screen.clone());
                screen
            })
            .map_err(|why| why.to_string())?;
        let screen = held.borrow().clone().ok_or("the screen was not made")?;
        cx.update_window(handle.into(), |_, window, cx| {
            window.activate_window();
            screen.update(cx, |screen, cx| {
                screen.enter(window, cx);
                screen.library = super::Load::Read(super::mock::library_with(if step.starts_with("pending") { 0 } else { 5 }));
                screen.relist(cx);
                screen.cursor = Some("agent-notes-on-a-diff".into());
            })
        })
        .map_err(|why| why.to_string())?;
        for _ in 0..3 {
            cx.run_until_parked();
            cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
        }
        cx.update_window(handle.into(), |_, window, cx| {
            screen.update(cx, |screen, cx| match step {
                "pending-row" => {
                    let focus = screen.curate.stops["agent-notes-on-a-diff"][0].clone();
                    window.focus(&focus, cx);
                }
                "pending-confirm" => screen.open_confirm("agent-notes-on-a-diff".into(), window, cx),
                "pending-merge-picker" => screen.open_merge("agent-notes-on-a-diff".into(), window, cx),
                "area-of-nine" => {}
                _ => {
                    screen.open_split("running-a-job".into(), window, cx);
                    if let Some(sheet) = &screen.curate.sheet {
                        let name = sheet.name.clone();
                        name.update(cx, |state, cx| state.set_value("Starting Jobs".to_string(), window, cx));
                    }
                    screen.toggle_split(0, cx);
                    screen.toggle_split(4, cx);
                }
            })
        })
        .map_err(|why| why.to_string())?;
        for _ in 0..4 {
            cx.run_until_parked();
            cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
        }
        let picture = cx.capture_screenshot(handle.into()).map_err(|why| why.to_string())?;
        let path = dir.join(format!("{step}-1400.png"));
        picture.save(&path).map_err(|why| format!("{}: {why}", path.display()))?;
    }
    Ok(())
}

/// The Dossier page open on the first feature at 1400 wide beside the list, and at 820 wide where the page is drawn alone.
pub fn shoot_dossier(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(crate::assets::Assets), || gpui_platform::current_headless_renderer());
    cx.update(crate::init_app);
    cx.update(|cx| cx.set_reduce_motion(true));
    for (name, wide) in [("dossier-1400", 1400.), ("dossier-820", 820.)] {
        let held: Rc<RefCell<Option<gpui::Entity<FeaturesScreen>>>> = Rc::default();
        let keep = held.clone();
        let handle = cx
            .open_window(size(px(wide), px(836.)), move |window, cx| {
                let screen = cx.new(|cx| FeaturesScreen::mock(window, cx));
                *keep.borrow_mut() = Some(screen.clone());
                screen
            })
            .map_err(|why| why.to_string())?;
        let screen = held.borrow().clone().ok_or("the screen was not made")?;
        cx.update_window(handle.into(), |_, window, cx| {
            window.activate_window();
            screen.update(cx, |screen, cx| {
                screen.enter(window, cx);
                screen.open_cursor(window, cx);
            });
        })
        .map_err(|why| why.to_string())?;
        for _ in 0..4 {
            cx.run_until_parked();
            cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
        }
        let picture = cx.capture_screenshot(handle.into()).map_err(|why| why.to_string())?;
        let path = dir.join(format!("{name}.png"));
        picture.save(&path).map_err(|why| format!("{}: {why}", path.display()))?;
    }
    Ok(())
}

/// The floor plan card on the first feature's page at 1400 wide: the mock's plan, then a page with no plan, then the same plan with
/// no room lit and no door.
pub fn shoot_floor_plan(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(crate::assets::Assets), || gpui_platform::current_headless_renderer());
    cx.update(crate::init_app);
    cx.update(|cx| cx.set_reduce_motion(true));
    for (name, step) in [("floor-plan-1400", 0), ("floor-plan-none-1400", 1), ("floor-plan-unlit-1400", 2)] {
        let held: Rc<RefCell<Option<gpui::Entity<FeaturesScreen>>>> = Rc::default();
        let keep = held.clone();
        let handle = cx
            .open_window(size(px(1400.), px(1100.)), move |window, cx| {
                let screen = cx.new(|cx| FeaturesScreen::mock(window, cx));
                *keep.borrow_mut() = Some(screen.clone());
                screen
            })
            .map_err(|why| why.to_string())?;
        let screen = held.borrow().clone().ok_or("the screen was not made")?;
        cx.update_window(handle.into(), |_, window, cx| {
            window.activate_window();
            screen.update(cx, |screen, cx| {
                screen.enter(window, cx);
                screen.open_cursor(window, cx);
                if let Some(page) = screen.page.clone() {
                    page.update(cx, |page, cx| {
                        if let super::page::Load::Read(read) = &mut page.page {
                            match step {
                                1 => read.floor_plan = None,
                                2 => {
                                    if let Some(plan) = read.floor_plan.as_mut() {
                                        plan.doors.clear();
                                        for room in &mut plan.rooms {
                                            room.lit = false;
                                            for file in &mut room.files {
                                                file.lit = false;
                                                file.changing = false;
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        page.rebuild(cx);
                    });
                }
            })
        })
        .map_err(|why| why.to_string())?;
        for _ in 0..4 {
            cx.run_until_parked();
            cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
        }
        let picture = cx.capture_screenshot(handle.into()).map_err(|why| why.to_string())?;
        let path = dir.join(format!("{name}.png"));
        picture.save(&path).map_err(|why| format!("{}: {why}", path.display()))?;
    }
    Ok(())
}

/// The librarian's card over the list in each state it can be in, and the chips row with another project chosen, at 1000 wide.
pub fn shoot_librarian(dir: &Path) -> Result<(), String> {
    use super::librarian::Press;
    use super::mock::{self, Reading};
    std::fs::create_dir_all(dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(crate::assets::Assets), || gpui_platform::current_headless_renderer());
    cx.update(crate::init_app);
    cx.update(|cx| cx.set_reduce_motion(true));
    let refusal = "Fleet holds no project named armada";
    let states: [(&str, Reading, Press, Option<&str>); 8] = [
        ("librarian-never", Reading::Never, Press::Idle, None),
        ("librarian-current", Reading::Current, Press::Idle, None),
        ("librarian-behind", Reading::Behind, Press::Idle, None),
        ("librarian-behind-long", Reading::FarBehind, Press::Idle, None),
        ("librarian-working", Reading::Behind, Press::Working, None),
        ("librarian-running", Reading::Running, Press::Idle, None),
        ("librarian-failed", Reading::Behind, Press::Failed(refusal.into()), None),
        ("chips-pocket", Reading::Current, Press::Idle, Some("pocket")),
    ];
    for (name, reading, press, project) in states {
        let held: Rc<RefCell<Option<gpui::Entity<FeaturesScreen>>>> = Rc::default();
        let keep = held.clone();
        let handle = cx
            .open_window(size(px(1000.), px(560.)), move |window, cx| {
                let screen = cx.new(|cx| FeaturesScreen::mock(window, cx));
                *keep.borrow_mut() = Some(screen.clone());
                screen
            })
            .map_err(|why| why.to_string())?;
        let screen = held.borrow().clone().ok_or("the screen was not made")?;
        cx.update_window(handle.into(), |_, window, cx| {
            window.activate_window();
            screen.update(cx, |screen, cx| {
                screen.enter(window, cx);
                if let Some(project) = project {
                    screen.choose(project.to_string(), cx);
                } else if let super::Load::Read(library) = &mut screen.library {
                    library.librarian = mock::librarian(reading);
                    let library = library.clone();
                    screen.run_read(&library);
                }
                screen.press = press;
                cx.notify();
            })
        })
        .map_err(|why| why.to_string())?;
        for _ in 0..4 {
            cx.run_until_parked();
            cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
        }
        let picture = cx.capture_screenshot(handle.into()).map_err(|why| why.to_string())?;
        let path = dir.join(format!("{name}.png"));
        picture.save(&path).map_err(|why| format!("{}: {why}", path.display()))?;
    }
    Ok(())
}

/// The project's Overview and Since you looked pages beside the list at 1400 wide: the Overview as the first read leaves it, with
/// terms in the glossary, and with no README sentence; Since you looked with its paragraph, and with nothing since.
pub fn shoot_project(dir: &Path) -> Result<(), String> {
    use super::page::Load;
    use super::project_model::ProjectPage;
    std::fs::create_dir_all(dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(crate::assets::Assets), || gpui_platform::current_headless_renderer());
    cx.update(crate::init_app);
    cx.update(|cx| cx.set_reduce_motion(true));
    let terms = || {
        vec![
            ipc::GlossaryTerm { term: "Job".into(), meaning: "One agent working one task in a slot, from a brief to a merged pull request.".into() },
            ipc::GlossaryTerm { term: "Slot".into(), meaning: "A warm worktree leased to a Job and given back when it lands.".into() },
            ipc::GlossaryTerm { term: "Story".into(), meaning: "The thread of a Job's asks and answers, read in order.".into() },
        ]
    };
    let steps: [(&str, ProjectPage, u8); 5] = [
        ("project-overview-1400", ProjectPage::Overview, 0),
        ("project-overview-glossary-1400", ProjectPage::Overview, 1),
        ("project-overview-no-readme-1400", ProjectPage::Overview, 2),
        ("project-since-1400", ProjectPage::Since, 0),
        ("project-since-nothing-1400", ProjectPage::Since, 3),
    ];
    for (name, which, step) in steps {
        let held: Rc<RefCell<Option<gpui::Entity<FeaturesScreen>>>> = Rc::default();
        let keep = held.clone();
        let handle = cx
            .open_window(size(px(1400.), px(836.)), move |window, cx| {
                let screen = cx.new(|cx| FeaturesScreen::mock(window, cx));
                *keep.borrow_mut() = Some(screen.clone());
                screen
            })
            .map_err(|why| why.to_string())?;
        let screen = held.borrow().clone().ok_or("the screen was not made")?;
        cx.update_window(handle.into(), |_, window, cx| {
            window.activate_window();
            screen.update(cx, |screen, cx| {
                screen.enter(window, cx);
                screen.open_project(which, window, cx);
                if let Some(page) = screen.project_page.clone() {
                    page.update(cx, |page, cx| {
                        if let Load::Read(read) = &mut page.overview {
                            match step {
                                1 => read.glossary = terms(),
                                2 => read.what_it_is = None,
                                _ => {}
                            }
                        }
                        if let (3, Load::Read(read)) = (step, &mut page.since) {
                            read.paragraph.clear();
                            read.features.clear();
                        }
                        cx.notify();
                    });
                }
            })
        })
        .map_err(|why| why.to_string())?;
        for _ in 0..4 {
            cx.run_until_parked();
            cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
        }
        let picture = cx.capture_screenshot(handle.into()).map_err(|why| why.to_string())?;
        let path = dir.join(format!("{name}.png"));
        picture.save(&path).map_err(|why| format!("{}: {why}", path.display()))?;
    }
    Ok(())
}

/// Who's on what beside the list at 1400 wide, in each of its three layouts, with the keys on a pull request, and with nothing to say.
pub fn shoot_who(dir: &Path) -> Result<(), String> {
    use super::page::Load;
    use super::project_model::ProjectPage;
    use super::who_model::WhoView;
    std::fs::create_dir_all(dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(crate::assets::Assets), || gpui_platform::current_headless_renderer());
    cx.update(crate::init_app);
    cx.update(|cx| cx.set_reduce_motion(true));
    // `(name, layout, keys on the page at this place, nothing to say)`.
    let steps: [(&str, WhoView, Option<usize>, bool); 5] = [
        ("who-grid-1400", WhoView::Grid, None, false),
        ("who-grid-keys-1400", WhoView::Grid, Some(2), false),
        ("who-by-feature-1400", WhoView::ByFeature, None, false),
        ("who-by-person-1400", WhoView::ByPerson, None, false),
        ("who-empty-1400", WhoView::Grid, None, true),
    ];
    for (name, view, at, empty) in steps {
        let held: Rc<RefCell<Option<gpui::Entity<FeaturesScreen>>>> = Rc::default();
        let keep = held.clone();
        let handle = cx
            .open_window(size(px(1400.), px(836.)), move |window, cx| {
                let screen = cx.new(|cx| FeaturesScreen::mock(window, cx));
                *keep.borrow_mut() = Some(screen.clone());
                screen
            })
            .map_err(|why| why.to_string())?;
        let screen = held.borrow().clone().ok_or("the screen was not made")?;
        cx.update_window(handle.into(), |_, window, cx| {
            window.activate_window();
            screen.update(cx, |screen, cx| {
                screen.enter(window, cx);
                screen.open_project(ProjectPage::Who, window, cx);
                if let Some(page) = screen.project_page.clone() {
                    page.update(cx, |page, cx| {
                        page.set_view(view, cx);
                        if empty {
                            page.who = Load::Read(ipc::WhosOnWhat { project: super::mock::PROJECT.into(), people: Vec::new(), features: Vec::new() });
                        }
                        if at.is_some() {
                            page.focus_body(window, cx);
                        }
                        page.at = at;
                        cx.notify();
                    });
                }
            })
        })
        .map_err(|why| why.to_string())?;
        for _ in 0..4 {
            cx.run_until_parked();
            cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx)).map_err(|why| why.to_string())?;
        }
        let picture = cx.capture_screenshot(handle.into()).map_err(|why| why.to_string())?;
        let path = dir.join(format!("{name}.png"));
        picture.save(&path).map_err(|why| format!("{}: {why}", path.display()))?;
    }
    Ok(())
}

/// The start panel of a project with no feature of its own at 1400 and 1100 wide: as it opens, with the keys on Draft as
/// Pending, and after the press with the drafted rows in the list.
pub fn shoot_start(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(crate::assets::Assets), || gpui_platform::current_headless_renderer());
    cx.update(crate::init_app);
    cx.update(|cx| cx.set_reduce_motion(true));
    for (name, wide, step) in [("features-start-1400", 1400., 0), ("features-start-keys-1400", 1400., 1), ("features-start-drafted-1400", 1400., 2), ("features-start-1100", 1100., 0)] {
        let held: Rc<RefCell<Option<gpui::Entity<FeaturesScreen>>>> = Rc::default();
        let keep = held.clone();
        let handle = cx
            .open_window(size(px(wide), px(820.)), move |window, cx| {
                let screen = cx.new(|cx| FeaturesScreen::mock_holding(super::mock::bare(), window, cx));
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
        cx.run_until_parked();
        cx.update_window(handle.into(), |_, window, cx| {
            screen.update(cx, |screen, cx| {
                if step > 0 {
                    screen.show_start(window, cx);
                }
                if let (2, Some(start)) = (step, screen.start.clone()) {
                    start.update(cx, |start, cx| start.draft(cx));
                }
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
