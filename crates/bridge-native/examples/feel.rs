//! How the native Bridge feels and what it costs, measured with no window on screen.
//!
//! An example, not a test, because gpui's Mac platform must be made on the main thread and the
//! test harness runs on another. It re-declares the crate's modules from `src/` (this crate has no
//! library target) and carries copies of `Shell` and `init_app` from `main.rs`, so a change there
//! must be mirrored here. Nothing in the shipped binary depends on it.
//!
//! `cargo build --example feel`, then `feel <measure>` with one of `job_switch_14`,
//! `job_switch_200`, `frame_14`, `frame_200`, `typing`, `process`.
//! A frame here is `Window::draw`: layout, prepaint and paint into a scene on the CPU. The scene
//! is not handed to a display; `render_to_image` (GPU raster and read back) is timed apart.
#![allow(dead_code)]

#[path = "../src/assets.rs"]
mod assets;
#[path = "../src/fleet.rs"]
mod fleet;
#[path = "../src/frames.rs"]
mod frames;
#[path = "../src/jobs.rs"]
mod jobs;
#[path = "../src/keymap.rs"]
mod keymap;
#[path = "../src/labels.rs"]
mod labels;
#[path = "../src/link.rs"]
mod link;
#[path = "../src/log.rs"]
mod log;
#[path = "../src/look/mod.rs"]
mod look;
#[path = "../src/mock.rs"]
mod mock;
#[path = "../src/runtime_file.rs"]
mod runtime_file;
#[path = "../src/transport.rs"]
mod transport;
#[path = "../src/walk/mod.rs"]
mod walk;
#[path = "../src/generated/actions.rs"]
pub mod generated_actions;
#[path = "../src/generated/vocabulary.rs"]
pub mod generated_vocabulary;
mod generated {
    pub use super::generated_actions as actions;
    pub use super::generated_vocabulary as vocabulary;
}

use gpui::{div, prelude::*, App, Context, Window};

use fleet::FleetConnection;
use labels::label_root;
use look::LookController;
use keymap::KeymapController;

pub fn init_app(cx: &mut App) {
    gpui_component::init(cx);
    look::init(cx);
    walk::window::bind(cx);
    keymap::install(&keymap::Rebinding::default(), cx);
}

pub struct Shell {
    screen: gpui::Entity<jobs::JobsScreen>,
    _fleet: Option<(gpui::Entity<FleetConnection>, gpui::Entity<LookController>, gpui::Entity<KeymapController>)>,
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        label_root(div().size_full().child(self.screen.clone()))
    }
}

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::{
    px, size, AppContext, Entity, HeadlessAppContext, Keystroke, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, PlatformInput,
    Point, Role, WindowHandle,
};
use ipc::{SessionId, SessionRecord};

use jobs::{JobsList, JobsScreen};
use labels::LabelBook;
use walk::window::{TakeNote, WalkWindow};

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

/// Median, 95th percentile, mean and worst of `samples`, in milliseconds.
fn line(label: &str, samples: &[f64]) -> String {
    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let at = |share: f64| sorted[(((sorted.len() - 1) as f64) * share).round() as usize];
    let mean = sorted.iter().sum::<f64>() / sorted.len() as f64;
    format!("{label:<44} n={:<4} median {:>8.2}  p95 {:>8.2}  mean {:>8.2}  max {:>8.2}", sorted.len(), at(0.5), at(0.95), mean, sorted[sorted.len() - 1])
}

fn headless() -> HeadlessAppContext {
    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(assets::Assets), || {
        gpui_platform::current_headless_renderer()
    });
    cx.update(init_app);
    cx
}

/// `count` live Jobs: the fixture's, repeated under new ids and titles.
fn jobs_of(count: usize) -> Vec<SessionRecord> {
    let base: Vec<SessionRecord> = mock::jobs("jobs-many").expect("the fixture decodes");
    if count <= base.len() {
        return base;
    }
    (0..count)
        .map(|at| {
            let mut job = base[at % base.len()].clone();
            job.id = SessionId::carried(format!("scaled-{at:03}"));
            job.title = Some(format!("{} {at}", job.title.as_deref().unwrap_or("Job")));
            job
        })
        .collect()
}

struct Jobs {
    cx: HeadlessAppContext,
    handle: WindowHandle<Shell>,
    focus: gpui::FocusHandle,
}

/// Open the Jobs screen of `count` Jobs in a headless 1100 by 760 window at scale 2.
fn open_jobs(count: usize) -> Jobs {
    let mut cx = headless();
    let records = jobs_of(count);
    let held: Rc<RefCell<Option<gpui::FocusHandle>>> = Rc::default();
    let keep = held.clone();
    let handle = cx
        .open_window(size(px(1100.), px(760.)), move |window, cx| {
            let list = cx.new(|cx| JobsList::mock(&records, cx));
            let focus = list.read(cx).focus_handle().clone();
            window.focus(&focus, cx);
            *keep.borrow_mut() = Some(focus);
            let screen = cx.new(|_| JobsScreen { list });
            cx.new(|_| Shell { screen, _fleet: None })
        })
        .expect("the window opens");
    let focus = held.borrow().clone().expect("focus");
    Jobs { cx, handle, focus }
}

/// The Jobs screen, drawn once.
fn jobs_window(count: usize) -> Jobs {
    let mut jobs = open_jobs(count);
    jobs.draw();
    jobs
}

impl Jobs {
    fn draw(&mut self) -> Duration {
        let start = Instant::now();
        self.cx.update_window(self.handle.into(), |_, window, cx| window.draw(cx).clear(cx)).expect("draws");
        start.elapsed()
    }

    fn redraw(&mut self) -> Duration {
        let start = Instant::now();
        self.cx
            .update_window(self.handle.into(), |_, window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
            })
            .expect("draws");
        start.elapsed()
    }

    fn key(&mut self, key: &'static str) -> Duration {
        let keystroke = Keystroke::parse(key).expect("a key");
        let focus = self.focus.clone();
        let start = Instant::now();
        self.cx
            .update_window(self.handle.into(), |_, window, cx| {
                window.focus(&focus, cx);
                window.dispatch_keystroke(keystroke, cx)
            })
            .expect("sends");
        start.elapsed()
    }

    fn press(&mut self, at: Point<gpui::Pixels>) -> Duration {
        let start = Instant::now();
        self.cx
            .update_window(self.handle.into(), |_, window, cx| {
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
            })
            .expect("presses");
        start.elapsed()
    }

    fn to_image(&mut self) -> Duration {
        let start = Instant::now();
        self.cx.capture_screenshot(self.handle.into()).expect("renders");
        start.elapsed()
    }

    /// The centre of every Job row drawn now, top to bottom.
    fn rows(&mut self) -> Vec<Point<gpui::Pixels>> {
        let id = self.handle.window_id();
        self.cx.update(|cx| {
            cx.global::<LabelBook>().drawn(id).iter().filter(|drawn| drawn.role == Role::Row).map(|drawn| drawn.bounds.center()).collect()
        })
    }
}

/// Job switch: from the act that opens a Job (Enter on the focused row, or a press on a row) to
/// the end of the frame that draws it opened.
fn switch_run(count: usize, rounds: usize) {
    let mut jobs = jobs_window(count);
    let reach = count.min(14) - 1;
    // Warm: text shaping and glyph caches fill on the first frames.
    for _ in 0..5 {
        jobs.key("j");
        jobs.draw();
        jobs.key("enter");
        jobs.draw();
        jobs.key("k");
        jobs.draw();
    }
    // Prove an open changes what is drawn, or the figures time a no-op.
    jobs.key("j");
    jobs.key("j");
    jobs.draw();
    let before = jobs.cx.capture_screenshot(jobs.handle.into()).expect("renders");
    jobs.key("enter");
    jobs.draw();
    let after = jobs.cx.capture_screenshot(jobs.handle.into()).expect("renders");
    assert_ne!(before.as_raw(), after.as_raw(), "Enter opens a Job and the frame shows it");
    jobs.key("k");
    jobs.key("k");
    jobs.draw();
    let (mut dispatch, mut draw, mut total, mut raster) = (vec![], vec![], vec![], vec![]);
    for _ in 0..rounds {
        for step in ["j", "k"] {
            for _ in 0..reach {
                jobs.key(step);
                jobs.draw();
                let start = Instant::now();
                let sent = jobs.key("enter");
                let drawn = jobs.draw();
                total.push(ms(start.elapsed()));
                dispatch.push(ms(sent));
                draw.push(ms(drawn));
                raster.push(ms(jobs.to_image()));
            }
        }
    }
    println!("--- Job switch by Enter, {count} Jobs in the list (debug_assertions {})", cfg!(debug_assertions));
    println!("{}", line("dispatch the open (key, action, effects)", &dispatch));
    println!("{}", line("draw the frame (CPU scene)", &draw));
    println!("{}", line("open to drawn frame (sum)", &total));
    println!("{}", line("  then GPU raster + readback of that frame", &raster));

    // A press on each drawn row in turn, the row bounds read from the last frame.
    let (mut dispatch, mut draw, mut total) = (vec![], vec![], vec![]);
    for _ in 0..rounds * 2 {
        let rows = jobs.rows();
        assert!(rows.len() > 5, "rows are drawn to press");
        let before = jobs.cx.capture_screenshot(jobs.handle.into()).expect("renders");
        jobs.press(rows[rows.len() / 2]);
        jobs.draw();
        let after = jobs.cx.capture_screenshot(jobs.handle.into()).expect("renders");
        assert_ne!(before.as_raw(), after.as_raw(), "a press opens a Job and the frame shows it");
        for at in rows {
            let start = Instant::now();
            let sent = jobs.press(at);
            let drawn = jobs.draw();
            total.push(ms(start.elapsed()));
            dispatch.push(ms(sent));
            draw.push(ms(drawn));
        }
    }
    println!("--- Job switch by a press on a row, {count} Jobs in the list");
    println!("{}", line("dispatch the press (down, up, click)", &dispatch));
    println!("{}", line("draw the frame (CPU scene)", &draw));
    println!("{}", line("press to drawn frame (sum)", &total));
}

fn job_switch_14() {
    switch_run(14, 4);
}

fn job_switch_200() {
    switch_run(200, 4);
}

/// Frame cost of the Jobs screen: the first frame, then a full re-render of the tree each time.
fn frame_run(count: usize) {
    let start = Instant::now();
    let mut jobs = open_jobs(count);
    let after_init = start.elapsed();
    let first = jobs.draw();
    let second = jobs.redraw();
    let first_raster = jobs.to_image();
    let (mut redraw, mut raster) = (vec![], vec![]);
    for _ in 0..300 {
        redraw.push(ms(jobs.redraw()));
    }
    for _ in 0..60 {
        jobs.redraw();
        raster.push(ms(jobs.to_image()));
    }
    println!("--- Frame cost, Jobs screen, {count} Jobs (debug_assertions {})", cfg!(debug_assertions));
    println!("app and renderer up: {:.1} ms; first frame {:.2} ms; second {:.2} ms; first GPU raster {:.1} ms", ms(after_init), ms(first), ms(second), ms(first_raster));
    println!("{}", line("full re-render + draw (refresh, CPU scene)", &redraw));
    println!("{}", line("GPU raster + readback, 2200x1520 px", &raster));
}

fn frame_14() {
    frame_run(14);
}

fn frame_200() {
    frame_run(200);
}

/// Typing lag: from a keystroke into the walk window's note box to the end of the frame that draws
/// the character.
fn typing() {
    let mut cx = headless();
    let held: Rc<RefCell<Option<Entity<WalkWindow>>>> = Rc::default();
    let keep = held.clone();
    let walk = walk::named("jobs-list").expect("the walk");
    let handle = cx
        .open_window(size(px(1100.), px(760.)), move |window, cx| {
            let walking = cx.new(|cx| WalkWindow::new(walk, None, window, cx));
            *keep.borrow_mut() = Some(walking.clone());
            walking
        })
        .expect("opens");
    let any = handle.into();
    cx.run_until_parked();
    cx.update_window(any, |_, window, cx| window.draw(cx).clear(cx)).unwrap();
    let before = cx.capture_screenshot(any).expect("renders");
    cx.update_window(any, |_, window, cx| window.dispatch_action(Box::new(TakeNote), cx)).unwrap();
    cx.run_until_parked();
    cx.update_window(any, |_, window, cx| window.draw(cx).clear(cx)).unwrap();
    let id = handle.window_id();
    let field = cx.update(|cx| cx.global::<LabelBook>().drawn(id).iter().any(|drawn| drawn.role == Role::TextInput && drawn.name == "Note"));
    assert!(field, "the note box is drawn");
    let empty = cx.capture_screenshot(any).expect("renders");

    const TEXT: [&str; 26] = [
        "t", "h", "e", "space", "m", "e", "r", "g", "e", "space", "l", "i", "n", "e", "space", "c", "h", "i", "p", "space", "i", "s", "space", "o", "f", "f",
    ];
    let (mut dispatch, mut draw, mut total) = (vec![], vec![], vec![]);
    for round in 0..4 {
        for key in TEXT {
            let keystroke = Keystroke::parse(key).expect("a key");
            let start = Instant::now();
            cx.update_window(any, |_, window, cx| window.dispatch_keystroke(keystroke, cx)).unwrap();
            let sent = start.elapsed();
            cx.update_window(any, |_, window, cx| window.draw(cx).clear(cx)).unwrap();
            let all = start.elapsed();
            if round > 0 {
                dispatch.push(ms(sent));
                draw.push(ms(all - sent));
                total.push(ms(all));
            }
        }
    }
    let typed = cx.capture_screenshot(any).expect("renders");
    if let Ok(dir) = std::env::var("FEEL_SHOTS") {
        std::fs::create_dir_all(&dir).unwrap();
        empty.save(format!("{dir}/note-empty.png")).unwrap();
        typed.save(format!("{dir}/note-typed.png")).unwrap();
    }
    assert_ne!(empty.as_raw(), typed.as_raw(), "the typed characters are drawn");
    assert_ne!(before.as_raw(), empty.as_raw(), "the note box is drawn");
    println!("--- Typing into the walk window's note box (debug_assertions {})", cfg!(debug_assertions));
    println!("{}", line("keystroke dispatch (input handler, effects)", &dispatch));
    println!("{}", line("draw the frame (CPU scene)", &draw));
    println!("{}", line("keystroke to drawn frame (sum)", &total));
}

/// Resident memory (KB) and CPU seconds of this process, as `ps` reports them.
fn ps() -> (u64, f64) {
    let out = std::process::Command::new("ps").args(["-o", "rss=,time=", "-p", &std::process::id().to_string()]).output().expect("ps runs");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut parts = text.split_whitespace();
    let rss = parts.next().and_then(|rss| rss.parse().ok()).unwrap_or(0);
    let time = parts.next().unwrap_or("0:00.00");
    let (minutes, seconds) = time.split_once(':').unwrap_or(("0", "0"));
    (rss, minutes.parse::<f64>().unwrap_or(0.0) * 60.0 + seconds.parse::<f64>().unwrap_or(0.0))
}

/// Memory and CPU of the process: before gpui, after the app is up, idle, then drawing 60 frames a
/// second for ten seconds. `FEEL_JOBS` sets how many Jobs are drawn; `FEEL_RASTER=1` also renders
/// each frame on the GPU and reads it back.
fn process() {
    let count: usize = std::env::var("FEEL_JOBS").ok().and_then(|n| n.parse().ok()).unwrap_or(14);
    let raster = std::env::var("FEEL_RASTER").is_ok();
    let (rss_start, _) = ps();
    let mut jobs = jobs_window(count);
    let (rss_up, _) = ps();
    let sample = |label: &str| {
        let (rss, cpu) = ps();
        println!("{label:<34} rss {:>7.1} MB   cpu {cpu:.2} s", rss as f64 / 1024.0);
        (rss, cpu)
    };
    println!("--- Process, {count} Jobs, raster {raster} (debug_assertions {})", cfg!(debug_assertions));
    println!("before gpui {:.1} MB, window up and drawn {:.1} MB", rss_start as f64 / 1024.0, rss_up as f64 / 1024.0);
    let (_, idle_from) = sample("idle start");
    std::thread::sleep(Duration::from_secs(10));
    let (_, idle_to) = sample("idle end, 10 s later");
    println!("idle CPU: {:.2}% of one core", (idle_to - idle_from) / 10.0 * 100.0);
    let (_, busy_from) = sample("drawing start");
    let wall = Instant::now();
    let mut frames = 0u32;
    let mut cost = Duration::ZERO;
    while wall.elapsed() < Duration::from_secs(10) {
        let begin = Instant::now();
        jobs.key(if frames % 24 < 12 { "j" } else { "k" });
        jobs.draw();
        if raster {
            jobs.to_image();
        }
        cost += begin.elapsed();
        frames += 1;
        let due = Duration::from_micros(16_667) * frames;
        if let Some(wait) = due.checked_sub(wall.elapsed()) {
            std::thread::sleep(wait);
        }
    }
    let (_, busy_to) = sample("drawing end");
    println!(
        "drawing CPU: {:.2}% of one core over {} frames at 60 a second; {:.2} ms a frame on average",
        (busy_to - busy_from) / wall.elapsed().as_secs_f64() * 100.0,
        frames,
        ms(cost) / f64::from(frames)
    );
}

/// The same screen in a real gpui window on the Mac platform: AppKit, a Metal layer and the app's
/// run loop. The window is made with `show: false` and a frame far off every screen, so it is never
/// ordered front and nothing is drawn to a display. A hidden window gets no display link, so no
/// frame is requested for it; this measures what a real platform costs to hold, and the loop draws
/// (CPU scene only, no present) 60 times a second itself.
fn window() {
    use gpui::{point, App, Bounds, WindowBounds, WindowOptions};
    let count: usize = std::env::var("FEEL_JOBS").ok().and_then(|n| n.parse().ok()).unwrap_or(14);
    let (rss_start, _) = ps();
    gpui_platform::application().with_assets(assets::Assets).run(move |cx: &mut App| {
        init_app(cx);
        let records = jobs_of(count);
        let bounds = Bounds::new(point(px(-20000.), px(-20000.)), size(px(1100.), px(760.)));
        let handle = cx
            .open_window(
                WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), show: false, focus: false, ..Default::default() },
                move |window, cx| {
                    let list = cx.new(|cx| JobsList::mock(&records, cx));
                    let focus = list.read(cx).focus_handle().clone();
                    window.focus(&focus, cx);
                    let screen = cx.new(|_| JobsScreen { list });
                    cx.new(|_| Shell { screen, _fleet: None })
                },
            )
            .expect("the window opens");
        cx.spawn(async move |cx| {
            let wait = |ms: u64| cx.background_executor().timer(Duration::from_millis(ms));
            let draw = |cx: &gpui::AsyncApp| {
                let start = Instant::now();
                cx.update(|cx| {
                    cx.update_window(handle.into(), |_, window, cx| {
                        window.refresh();
                        window.draw(cx).clear(cx);
                    })
                    .ok();
                });
                start.elapsed()
            };
            draw(&cx);
            wait(3000).await;
            let (rss_up, idle_from) = ps();
            wait(10_000).await;
            let (rss_idle, idle_to) = ps();
            println!("--- Real window, hidden and off screen, {count} Jobs (debug_assertions {})", cfg!(debug_assertions));
            println!("before gpui {:.1} MB, window up and drawn {:.1} MB, after idle {:.1} MB", rss_start as f64 / 1024.0, rss_up as f64 / 1024.0, rss_idle as f64 / 1024.0);
            println!("idle CPU: {:.2}% of one core", (idle_to - idle_from) / 10.0 * 100.0);
            let begin = Instant::now();
            let (mut frames, mut cost) = (0u32, Duration::ZERO);
            while begin.elapsed() < Duration::from_secs(10) {
                cost += draw(&cx);
                frames += 1;
                wait(16).await;
            }
            let (rss_busy, busy_to) = ps();
            println!(
                "drawing CPU: {:.2}% of one core over {} frames; {:.2} ms a frame; rss {:.1} MB",
                (busy_to - idle_to) / begin.elapsed().as_secs_f64() * 100.0,
                frames,
                ms(cost) / f64::from(frames),
                rss_busy as f64 / 1024.0
            );
            cx.update(|cx| cx.quit());
        })
        .detach();
    });
}

fn main() {
    let which = std::env::args().nth(1).unwrap_or_default();
    match which.as_str() {
        "job_switch_14" => job_switch_14(),
        "job_switch_200" => job_switch_200(),
        "frame_14" => frame_14(),
        "frame_200" => frame_200(),
        "typing" => typing(),
        "process" => process(),
        "window" => window(),
        other => {
            eprintln!("no measure named {other:?}");
            std::process::exit(2);
        }
    }
}
