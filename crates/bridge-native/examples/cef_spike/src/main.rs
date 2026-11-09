//! Spike: Chrome (CEF windowless) painting into a gpui `surface`, with no window on screen.
//!
//! CEF hands over BGRA through `OnPaint`; gpui's `surface` takes only a bi-planar 4:2:0 full-range
//! CVPixelBuffer, so each frame is converted. gpui draws it in a headless window and writes the
//! result as a picture, the way `bridge-native --walk <name> --check` draws without a window.
//! Env: CEF_SPIKE_PAGE (file to load), CEF_SPIKE_OUT (picture directory), CEF_SPIKE_SECS (run
//! length), CEF_SPIKE_THREADS (conversion threads), CEF_SPIKE_PORT (remote debugging port).

mod web;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::{div, px, rgb, size, AnyWindowHandle, Entity, HeadlessAppContext, ObjectFit};
use objc2_core_foundation::CFRetained;
use objc2_core_video::CVPixelBuffer;

/// What gpui draws: a header bar, the web pane (one `surface`), and a gpui chip over the pane.
struct Pane {
    frame: Option<(CFRetained<CVPixelBuffer>, usize, usize)>,
    scale: f32,
}

impl Render for Pane {
    fn render(&mut self, _window: &mut gpui::Window, _cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let web = match &self.frame {
            Some((buf, w, h)) => gpui::surface(buf.clone())
                .object_fit(ObjectFit::Fill)
                .w(px(*w as f32 / self.scale))
                .h(px(*h as f32 / self.scale))
                .into_any_element(),
            None => div().into_any_element(),
        };
        div()
            .size_full()
            .bg(rgb(0xf5f5f7))
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(48.))
                    .w_full()
                    .px_4()
                    .flex()
                    .items_center()
                    .text_color(rgb(0x1d1d1f))
                    .child("Armada: a gpui header above a Chrome pane"),
            )
            .child(
                div().relative().child(web).child(
                    div()
                        .absolute()
                        .bottom(px(8.))
                        .right(px(8.))
                        .px_2()
                        .py_1()
                        .bg(rgb(0x1d1d1f))
                        .text_color(rgb(0xffffff))
                        .text_size(px(12.))
                        .child("gpui chip over the web pane"),
                ),
            )
    }
}

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

fn main() {
    let secs: f64 = env_or("CEF_SPIKE_SECS", "14").parse().unwrap();
    let out = PathBuf::from(env_or("CEF_SPIKE_OUT", "/tmp/cef-spike-out"));
    let page = env_or("CEF_SPIKE_PAGE", "");
    let port: i32 = env_or("CEF_SPIKE_PORT", "9333").parse().unwrap();
    let threads: usize = env_or("CEF_SPIKE_THREADS", "4").parse().unwrap();
    std::fs::create_dir_all(&out).unwrap();

    let scale = 2.0f32;
    let (win_w, win_h, header) = (1100.0f32, 760.0f32, 48.0f32);
    let shared = web::Shared::new((win_w as i32, (win_h - header) as i32), scale, threads);

    let t_start = Instant::now();
    let Some(mut host) = web::start(shared.clone(), &page, port, &out) else {
        return; // a helper process: it ran and is done
    };
    let t_ready = t_start.elapsed();
    if env_or("CEF_SPIKE_REAL", "0") == "1" {
        run_real(shared, host, secs, out, scale, (win_w, win_h), header, t_start, t_ready);
        return;
    }

    // gpui: a headless app, a window that is never shown, and the Metal renderer for pictures.
    let platform = gpui_platform::current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(()), || gpui_platform::current_headless_renderer());
    let view_cell: Rc<RefCell<Option<Entity<Pane>>>> = Rc::default();
    let keep = view_cell.clone();
    let handle = cx
        .open_window(size(px(win_w), px(win_h)), move |_, cx| {
            let pane = cx.new(|_| Pane { frame: None, scale });
            *keep.borrow_mut() = Some(pane.clone());
            pane
        })
        .unwrap();
    let pane = view_cell.borrow().clone().unwrap();
    let any: AnyWindowHandle = handle.into();

    let draw = |cx: &mut HeadlessAppContext, pane: &Entity<Pane>, frame: Option<web::Frame>| -> Duration {
        let t = Instant::now();
        cx.update_window(any, |_, window, cx| {
            if let Some(f) = frame {
                pane.update(cx, |p, cx| {
                    p.frame = Some((f.buf, f.w, f.h));
                    cx.notify();
                });
            }
            window.draw(cx).clear(cx);
        })
        .unwrap();
        t.elapsed()
    };

    let mut draw_us: Vec<u64> = Vec::new();
    let snap = |cx: &mut HeadlessAppContext, name: &str| {
        let t = Instant::now();
        let img = cx.capture_screenshot(any).unwrap();
        let render_ms = t.elapsed().as_secs_f64() * 1000.0;
        img.save(out.join(format!("gpui-{name}.png"))).unwrap();
        shared.save_raw(&out.join(format!("raw-{name}.png")), &img, (header * scale) as u32, render_ms);
    };

    let poll_ms: u64 = env_or("CEF_SPIKE_POLL_MS", "2").parse().unwrap();
    let t0 = Instant::now();
    let mut did = [false; 8];
    let mut first_frame_at: Option<Duration> = None;
    let mut resized_at: Option<Instant> = None;
    let mut resize_latency: Option<Duration> = None;
    let mut window_marks: Vec<(&str, usize)> = Vec::new();
    loop {
        web::pump();
        cx.run_until_parked();
        let at = t0.elapsed().as_secs_f64();

        if let Some(f) = shared.take_frame() {
            if first_frame_at.is_none() {
                first_frame_at = Some(t_start.elapsed());
            }
            if let (Some(r), true) = (resized_at, f.w as i32 == (900.0 * scale) as i32) {
                resize_latency.get_or_insert(r.elapsed());
            }
            draw_us.push(draw(&mut cx, &pane, Some(f)).as_micros() as u64);
        }

        // The scripted session, in seconds from the browser being ready.
        if at > 2.0 && !did[0] {
            did[0] = true;
            window_marks.push(("fps window start", shared.paint_count()));
            snap(&mut cx, "first");
        }
        if at > 5.0 && !did[1] {
            did[1] = true;
            window_marks.push(("fps window end", shared.paint_count()));
            host.click(124, 308); // the input
        }
        if at > 5.5 && !did[2] {
            did[2] = true;
            host.type_text("hi");
        }
        if at > 6.5 && !did[3] {
            did[3] = true;
            host.wheel(200, 450, -240);
        }
        if at > 7.5 && !did[4] {
            did[4] = true;
            resized_at = Some(Instant::now());
            shared.set_size((900, 500));
            host.resized();
        }
        if at > secs - 2.0 && !did[5] {
            did[5] = true;
            snap(&mut cx, "after");
        }
        if at > secs {
            break;
        }
        std::thread::sleep(Duration::from_millis(poll_ms));
    }

    web::stop(host);
    shared.report(t_ready, first_frame_at, resize_latency, &draw_us, &window_marks, 2.0, 5.0);
}

/// The same thing in a real gpui window and a real `Application` run loop, so the Chrome pump
/// shares gpui's NSApplication. The window is made at coordinates far off every screen and is
/// never ordered in (`show: false`), so nothing appears; its pictures come from `render_to_image`.
#[allow(clippy::too_many_arguments)]
fn run_real(shared: Rc<web::Shared>, host: web::Host, secs: f64, out: PathBuf, scale: f32, win: (f32, f32), header: f32, t_start: Instant, t_ready: Duration) {
    let host_in = Rc::new(RefCell::new(Some(host)));
    let shared_in = shared.clone();
    let draw_us: Rc<RefCell<Vec<u64>>> = Rc::default();
    let draw_in = draw_us.clone();
    let first: Rc<RefCell<Option<Duration>>> = Rc::default();
    let first_in = first.clone();
    gpui_platform::application().run(move |cx: &mut gpui::App| {
        let bounds = gpui::Bounds { origin: gpui::point(px(-6000.), px(-6000.)), size: size(px(win.0), px(win.1)) };
        let handle = cx
            .open_window(
                gpui::WindowOptions { window_bounds: Some(gpui::WindowBounds::Windowed(bounds)), show: false, focus: false, titlebar: None, ..Default::default() },
                |_, cx| cx.new(|_| Pane { frame: None, scale }),
            )
            .unwrap();
        cx.spawn(async move |cx| {
            let t0 = Instant::now();
            loop {
                cx.background_executor().timer(Duration::from_millis(4)).await;
                let done = cx
                    .update(|cx| {
                        web::pump();
                        if let Some(f) = shared_in.take_frame() {
                            first_in.borrow_mut().get_or_insert(t_start.elapsed());
                            let t = Instant::now();
                            let _ = handle.update(cx, |pane, _, cx| {
                                pane.frame = Some((f.buf, f.w, f.h));
                                cx.notify();
                            });
                            let _ = cx.update_window(handle.into(), |_, window, cx| window.draw(cx).clear(cx));
                            draw_in.borrow_mut().push(t.elapsed().as_micros() as u64);
                        }
                        t0.elapsed().as_secs_f64() > secs
                    })
                    ;
                if done {
                    break;
                }
            }
            let _ = cx.update(|cx| {
                let t = Instant::now();
                if let Ok(Ok(img)) = cx.update_window(handle.into(), |_, window, _| window.render_to_image()) {
                    let ms = t.elapsed().as_secs_f64() * 1000.0;
                    img.save(out.join("gpui-real.png")).unwrap();
                    shared_in.save_raw(&out.join("raw-real.png"), &img, (header * scale) as u32, ms);
                }
                // `quit` ends the process from inside the run loop, so report and shut Chrome down first.
                println!("RESULT real window and Application run loop: first frame {:.0} ms after process start", first_in.borrow().map(|d| d.as_secs_f64() * 1000.0).unwrap_or(-1.0));
                shared_in.report(t_ready, *first_in.borrow(), None, &draw_in.borrow(), &[], 2.0, 5.0);
                if let Some(h) = host_in.borrow_mut().take() {
                    web::stop(h);
                }
                cx.quit();
            });
        })
        .detach();
    });
}
