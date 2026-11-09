//! The Chrome side of the spike: windowless CEF, the paint callback, and the BGRA -> NV12 hand-off
//! gpui's `surface` needs.

use std::cell::{Cell, RefCell};
use std::path::Path;
use std::ptr;
use std::rc::Rc as StdRc;
use std::time::{Duration, Instant};

use cef::{rc::Rc, *};
use objc2_core_foundation::{CFDictionary, CFRetained, CFType};
use objc2_core_video::{
    kCVPixelBufferIOSurfacePropertiesKey, kCVPixelFormatType_420YpCbCr8BiPlanarFullRange, kCVReturnSuccess, CVPixelBuffer, CVPixelBufferCreate,
    CVPixelBufferGetBaseAddressOfPlane, CVPixelBufferGetBytesPerRowOfPlane, CVPixelBufferLockBaseAddress, CVPixelBufferLockFlags,
    CVPixelBufferUnlockBaseAddress,
};

pub struct Frame {
    pub buf: CFRetained<CVPixelBuffer>,
    pub w: usize,
    pub h: usize,
}

#[derive(Default)]
struct Stats {
    paints: Vec<Instant>,
    alloc_us: Vec<u64>,
    convert_us: Vec<u64>,
    sizes: Vec<(usize, usize)>,
    callback_us: Vec<u64>,
}

pub struct Shared {
    size: Cell<(i32, i32)>,
    scale: f32,
    threads: usize,
    ready: Cell<bool>,
    frame: RefCell<Option<Frame>>,
    stats: RefCell<Stats>,
    /// The newest BGRA frame as CEF gave it, for comparing with what gpui draws.
    raw: RefCell<Option<(usize, usize, Vec<u8>)>>,
    /// What Chrome's accessibility callback delivered: (calls, bytes, "role" keys, saw page text).
    ax: RefCell<(usize, usize, usize, bool)>,
}

impl Shared {
    pub fn new(size: (i32, i32), scale: f32, threads: usize) -> StdRc<Self> {
        StdRc::new(Self {
            size: Cell::new(size),
            scale,
            threads,
            ready: Cell::new(false),
            frame: RefCell::default(),
            stats: RefCell::default(),
            raw: RefCell::default(),
            ax: RefCell::default(),
        })
    }
    pub fn take_frame(&self) -> Option<Frame> {
        self.frame.borrow_mut().take()
    }
    pub fn set_size(&self, size: (i32, i32)) {
        self.size.set(size);
    }
    pub fn paint_count(&self) -> usize {
        self.stats.borrow().paints.len()
    }

    /// Print the numbers the spike doc quotes.
    #[allow(clippy::too_many_arguments)]
    pub fn report(
        &self,
        t_ready: Duration,
        first_frame: Option<Duration>,
        resize_latency: Option<Duration>,
        draw_us: &[u64],
        marks: &[(&str, usize)],
        t_a: f64,
        t_b: f64,
    ) {
        let s = self.stats.borrow();
        let pct = |v: &[u64], p: f64| -> f64 {
            if v.is_empty() {
                return 0.0;
            }
            let mut v = v.to_vec();
            v.sort_unstable();
            v[((v.len() - 1) as f64 * p) as usize] as f64
        };
        let mean = |v: &[u64]| if v.is_empty() { 0.0 } else { v.iter().sum::<u64>() as f64 / v.len() as f64 };
        println!("RESULT cef initialised and browser created: {:.0} ms after process start", t_ready.as_secs_f64() * 1000.0);
        println!("RESULT first frame reached gpui: {:.0} ms after process start", first_frame.map(|d| d.as_secs_f64() * 1000.0).unwrap_or(-1.0));
        println!("RESULT resize 1100x712 -> 900x500 to first new-size frame: {:.0} ms", resize_latency.map(|d| d.as_secs_f64() * 1000.0).unwrap_or(-1.0));
        println!("RESULT paints total: {}", s.paints.len());
        if marks.len() == 2 {
            let n = marks[1].1 - marks[0].1;
            println!("RESULT fps over the {:.0} s animated window: {:.1}", t_b - t_a, n as f64 / (t_b - t_a));
        }
        let intervals: Vec<u64> = s.paints.windows(2).map(|w| (w[1] - w[0]).as_micros() as u64).collect();
        println!("RESULT paint interval us: p50 {:.0} p95 {:.0} max {:.0}", pct(&intervals, 0.5), pct(&intervals, 0.95), pct(&intervals, 1.0));
        println!("RESULT CVPixelBuffer alloc us: mean {:.0} p95 {:.0}", mean(&s.alloc_us), pct(&s.alloc_us, 0.95));
        println!("RESULT BGRA->NV12 convert us ({} threads): mean {:.0} p95 {:.0}", self.threads, mean(&s.convert_us), pct(&s.convert_us, 0.95));
        println!("RESULT whole OnPaint callback us: mean {:.0} p95 {:.0}", mean(&s.callback_us), pct(&s.callback_us, 0.95));
        println!("RESULT gpui update+draw us (headless window): mean {:.0} p95 {:.0}", mean(draw_us), pct(draw_us, 0.95));
        let ax = self.ax.borrow();
        println!("RESULT accessibility tree callbacks: {} calls, {} bytes of JSON, {} role keys, page text present: {}", ax.0, ax.1, ax.2, ax.3);
        let mut sizes = s.sizes.clone();
        sizes.dedup();
        println!("RESULT paint buffer sizes seen: {sizes:?}");
    }

    /// Compare the newest raw frame with the region of gpui's picture that holds the pane.
    pub fn save_raw(&self, path: &Path, gpui_img: &image::RgbaImage, y_off: u32, render_ms: f64) {
        let raw = self.raw.borrow();
        let Some((w, h, bgra)) = raw.as_ref() else { return };
        let mut rgba = image::RgbaImage::new(*w as u32, *h as u32);
        for (i, p) in rgba.pixels_mut().enumerate() {
            *p = image::Rgba([bgra[i * 4 + 2], bgra[i * 4 + 1], bgra[i * 4], 255]);
        }
        rgba.save(path).unwrap();
        let (mut sum, mut over16, mut over48, mut n) = (0u64, 0u64, 0u64, 0u64);
        // The page animates, so compare only the static top band (header text and colour chips).
        let band = (*h as u32).min(170 * self.scale as u32 / 2 * 2);
        for y in 0..band {
            for x in 0..(*w as u32) {
                if y + y_off >= gpui_img.height() {
                    continue;
                }
                let a = rgba.get_pixel(x, y).0;
                let b = gpui_img.get_pixel(x, y + y_off).0;
                let d = (0..3).map(|c| a[c].abs_diff(b[c]) as u64).collect::<Vec<_>>();
                let m = *d.iter().max().unwrap();
                sum += d.iter().sum::<u64>();
                over16 += (m > 16) as u64;
                over48 += (m > 48) as u64;
                n += 1;
            }
        }
        println!(
            "RESULT {}: gpui render_to_image {:.0} ms; static band {}x{} px vs raw BGRA: mean channel error {:.2}/255, {:.2}% of pixels off by >16, {:.2}% by >48",
            path.file_name().unwrap().to_string_lossy(),
            render_ms,
            w,
            band,
            sum as f64 / (n * 3) as f64,
            over16 as f64 * 100.0 / n as f64,
            over48 as f64 * 100.0 / n as f64
        );
    }
}

// ---- NV12 hand-off -------------------------------------------------------------------------

fn alloc_nv12(w: usize, h: usize) -> CFRetained<CVPixelBuffer> {
    let key = unsafe { kCVPixelBufferIOSurfacePropertiesKey };
    let attrs = CFDictionary::from_slices(&[key], &[&*CFDictionary::<CFType, CFType>::empty()]);
    let mut out = ptr::null_mut();
    let status = unsafe {
        CVPixelBufferCreate(None, w, h, kCVPixelFormatType_420YpCbCr8BiPlanarFullRange, Some(attrs.as_opaque()), ptr::NonNull::from(&mut out))
    };
    assert_eq!(status, kCVReturnSuccess, "CVPixelBufferCreate");
    unsafe { CFRetained::from_raw(ptr::NonNull::new(out).unwrap()) }
}

#[derive(Clone, Copy)]
struct Planes {
    y: usize,
    y_stride: usize,
    uv: usize,
    uv_stride: usize,
}

/// BT.601 full range, the matrix gpui's `surface_fragment` shader inverts. Chroma is the mean of
/// each 2x2 block.
fn convert_rows(src: &[u8], w: usize, h: usize, p: Planes, pair_rows: std::ops::Range<usize>) {
    for r in pair_rows {
        let y0 = r * 2;
        for dy in 0..2 {
            let y = y0 + dy;
            if y >= h {
                break;
            }
            let row = &src[y * w * 4..(y + 1) * w * 4];
            let dst = unsafe { std::slice::from_raw_parts_mut((p.y + y * p.y_stride) as *mut u8, w) };
            for (x, px) in row.chunks_exact(4).enumerate() {
                let (b, g, r) = (px[0] as i32, px[1] as i32, px[2] as i32);
                dst[x] = ((77 * r + 150 * g + 29 * b + 128) >> 8) as u8;
            }
        }
        let uv = unsafe { std::slice::from_raw_parts_mut((p.uv + r * p.uv_stride) as *mut u8, w.div_ceil(2) * 2) };
        let y1 = (y0 + 1).min(h - 1);
        for c in 0..w.div_ceil(2) {
            let x0 = c * 2;
            let x1 = (x0 + 1).min(w - 1);
            let (mut sr, mut sg, mut sb) = (0i32, 0i32, 0i32);
            for (x, y) in [(x0, y0), (x1, y0), (x0, y1), (x1, y1)] {
                let i = (y * w + x) * 4;
                sb += src[i] as i32;
                sg += src[i + 1] as i32;
                sr += src[i + 2] as i32;
            }
            let (r, g, b) = (sr >> 2, sg >> 2, sb >> 2);
            let cb = (((-43 * r - 85 * g + 128 * b + 128) >> 8) + 128).clamp(0, 255);
            let cr = (((128 * r - 107 * g - 21 * b + 128) >> 8) + 128).clamp(0, 255);
            uv[c * 2] = cb as u8;
            uv[c * 2 + 1] = cr as u8;
        }
    }
}

fn convert(src: &[u8], w: usize, h: usize, buf: &CVPixelBuffer, threads: usize) {
    unsafe {
        assert_eq!(CVPixelBufferLockBaseAddress(buf, CVPixelBufferLockFlags::empty()), kCVReturnSuccess);
        let p = Planes {
            y: CVPixelBufferGetBaseAddressOfPlane(buf, 0) as usize,
            y_stride: CVPixelBufferGetBytesPerRowOfPlane(buf, 0),
            uv: CVPixelBufferGetBaseAddressOfPlane(buf, 1) as usize,
            uv_stride: CVPixelBufferGetBytesPerRowOfPlane(buf, 1),
        };
        let pairs = h.div_ceil(2);
        if threads <= 1 {
            convert_rows(src, w, h, p, 0..pairs);
        } else {
            let per = pairs.div_ceil(threads);
            std::thread::scope(|s| {
                for t in 0..threads {
                    let (a, b) = (t * per, ((t + 1) * per).min(pairs));
                    if a < b {
                        s.spawn(move || convert_rows(src, w, h, p, a..b));
                    }
                }
            });
        }
        assert_eq!(CVPixelBufferUnlockBaseAddress(buf, CVPixelBufferLockFlags::empty()), kCVReturnSuccess);
    }
}

// ---- CEF handlers --------------------------------------------------------------------------

wrap_render_handler! {
    struct SpikeRender {
        shared: StdRc<Shared>,
        ax: AccessibilityHandler,
    }

    impl RenderHandler {
        fn accessibility_handler(&self) -> Option<AccessibilityHandler> {
            Some(self.ax.clone())
        }

        fn view_rect(&self, _browser: Option<&mut Browser>, rect: Option<&mut Rect>) {
            if let Some(rect) = rect {
                let (w, h) = self.shared.size.get();
                rect.width = w;
                rect.height = h;
            }
        }

        fn screen_info(&self, _browser: Option<&mut Browser>, info: Option<&mut ScreenInfo>) -> ::std::os::raw::c_int {
            let Some(info) = info else { return false as _ };
            let (w, h) = self.shared.size.get();
            info.device_scale_factor = self.shared.scale;
            info.depth = 24;
            info.depth_per_component = 8;
            info.rect = Rect { x: 0, y: 0, width: w, height: h };
            info.available_rect = info.rect.clone();
            true as _
        }

        fn on_paint(
            &self,
            _browser: Option<&mut Browser>,
            type_: PaintElementType,
            _dirty_rects: Option<&[Rect]>,
            buffer: *const u8,
            width: ::std::os::raw::c_int,
            height: ::std::os::raw::c_int,
        ) {
            if type_ != PaintElementType::VIEW || buffer.is_null() || width <= 0 || height <= 0 {
                return;
            }
            let t_cb = Instant::now();
            let (w, h) = (width as usize, height as usize);
            let src = unsafe { std::slice::from_raw_parts(buffer, w * h * 4) };
            let t = Instant::now();
            let buf = alloc_nv12(w, h);
            let alloc = t.elapsed();
            let t = Instant::now();
            convert(src, w, h, &buf, self.shared.threads);
            let conv = t.elapsed();
            // Keep a copy of the source for the picture comparison only when asked: 12 MB a frame.
            if std::env::var_os("CEF_SPIKE_KEEP_RAW").is_some() || self.shared.stats.borrow().paints.len() % 30 == 0 {
                *self.shared.raw.borrow_mut() = Some((w, h, src.to_vec()));
            }
            *self.shared.frame.borrow_mut() = Some(Frame { buf, w, h });
            let mut s = self.shared.stats.borrow_mut();
            s.paints.push(t_cb);
            s.alloc_us.push(alloc.as_micros() as u64);
            s.convert_us.push(conv.as_micros() as u64);
            s.sizes.push((w, h));
            s.callback_us.push(t_cb.elapsed().as_micros() as u64);
        }
    }
}

wrap_accessibility_handler! {
    struct SpikeAx {
        shared: StdRc<Shared>,
    }

    impl AccessibilityHandler {
        fn on_accessibility_tree_change(&self, value: Option<&mut Value>) {
            let Some(v) = value else { return };
            let json = CefString::from(&write_json(Some(v), JsonWriterOptions::DEFAULT)).to_string();
            let mut ax = self.shared.ax.borrow_mut();
            ax.0 += 1;
            ax.1 += json.len();
            ax.2 += json.matches("\"role\"").count();
            if json.contains("Chrome inside the Bridge window") && !ax.3 {
                ax.3 = true;
                if let Some(dir) = std::env::var_os("CEF_SPIKE_OUT") {
                    let _ = std::fs::write(std::path::Path::new(&dir).join("ax-tree.json"), &json);
                }
            }
        }
    }
}

wrap_client! {
    struct SpikeClient {
        render: RenderHandler,
    }

    impl Client {
        fn render_handler(&self) -> Option<RenderHandler> {
            Some(self.render.clone())
        }
    }
}

wrap_browser_process_handler! {
    struct SpikeProcessHandler {
        shared: StdRc<Shared>,
    }

    impl BrowserProcessHandler {
        fn on_context_initialized(&self) {
            self.shared.ready.set(true);
        }
    }
}

wrap_app! {
    struct SpikeApp {
        shared: StdRc<Shared>,
    }

    impl App {
        fn on_before_command_line_processing(&self, _process_type: Option<&CefString>, command_line: Option<&mut CommandLine>) {
            let Some(cl) = command_line else { return };
            // A mock keychain: without it Chrome asks the macOS keychain for its safe-storage key,
            // which puts a password dialog on the screen.
            cl.append_switch(Some(&"use-mock-keychain".into()));
            cl.append_switch(Some(&"noerrdialogs".into()));
            cl.append_switch(Some(&"hide-crash-restore-bubble".into()));
            cl.append_switch_with_value(Some(&"remote-allow-origins".into()), Some(&"*".into()));
        }

        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(SpikeProcessHandler::new(self.shared.clone()))
        }
    }
}

pub struct Host {
    browser: Browser,
}

impl Host {
    fn host(&self) -> BrowserHost {
        self.browser.host().unwrap()
    }
    pub fn click(&mut self, x: i32, y: i32) {
        let h = self.host();
        h.set_focus(1);
        let ev = MouseEvent { x, y, modifiers: 0 };
        h.send_mouse_move_event(Some(&ev), 0);
        h.send_mouse_click_event(Some(&ev), MouseButtonType::LEFT, 0, 1);
        h.send_mouse_click_event(Some(&ev), MouseButtonType::LEFT, 1, 1);
    }
    pub fn type_text(&mut self, text: &str) {
        let h = self.host();
        for ch in text.chars() {
            let vk = ch.to_ascii_uppercase() as i32;
            let mk = |t: KeyEventType| KeyEvent {
                type_: t,
                windows_key_code: if t == KeyEventType::CHAR { ch as i32 } else { vk },
                native_key_code: 0,
                character: ch as u16,
                unmodified_character: ch as u16,
                focus_on_editable_field: 1,
                ..Default::default()
            };
            h.send_key_event(Some(&mk(KeyEventType::RAWKEYDOWN)));
            h.send_key_event(Some(&mk(KeyEventType::CHAR)));
            h.send_key_event(Some(&mk(KeyEventType::KEYUP)));
        }
    }
    pub fn wheel(&mut self, x: i32, y: i32, dy: i32) {
        let ev = MouseEvent { x, y, modifiers: 0 };
        self.host().send_mouse_move_event(Some(&ev), 0);
        self.host().send_mouse_wheel_event(Some(&ev), 0, dy);
    }
    pub fn resized(&mut self) {
        self.host().was_resized();
    }
}

thread_local! {
    static SHARED: RefCell<Option<StdRc<Shared>>> = const { RefCell::new(None) };
    static LOADER: RefCell<Option<library_loader::LibraryLoader>> = const { RefCell::new(None) };
}

pub fn pump() {
    do_message_loop_work();
}

/// Load Chrome, start it windowless, and make one browser. `None` means this process was a helper.
pub fn start(shared: StdRc<Shared>, page: &str, port: i32, out: &Path) -> Option<Host> {
    let loader = library_loader::LibraryLoader::new(&std::env::current_exe().unwrap(), false);
    assert!(loader.load(), "the Chrome framework did not load");
    LOADER.with(|l| *l.borrow_mut() = Some(loader));
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);

    let args = args::Args::new();
    let mut app = SpikeApp::new(shared.clone());
    let ret = execute_process(Some(args.as_main_args()), Some(&mut app), ptr::null_mut());
    if ret >= 0 {
        return None;
    }
    let profile = out.join("armada-profile");
    let settings = Settings {
        no_sandbox: 1,
        windowless_rendering_enabled: 1,
        external_message_pump: 1,
        remote_debugging_port: port,
        root_cache_path: profile.to_string_lossy().as_ref().into(),
        cache_path: profile.to_string_lossy().as_ref().into(),
        log_file: out.join("chrome.log").to_string_lossy().as_ref().into(),
        ..Default::default()
    };
    assert_eq!(initialize(Some(args.as_main_args()), Some(&settings), Some(&mut app), ptr::null_mut()), 1);
    SHARED.with(|s| *s.borrow_mut() = Some(shared.clone()));
    while !shared.ready.get() {
        do_message_loop_work();
        std::thread::sleep(Duration::from_millis(2));
    }

    let window_info = WindowInfo { windowless_rendering_enabled: 1, ..Default::default() };
    let browser_settings = BrowserSettings { windowless_frame_rate: 60, background_color: 0xFFFF_FFFF, ..Default::default() };
    let mut client = SpikeClient::new(SpikeRender::new(shared.clone(), SpikeAx::new(shared.clone())));
    let url = if page.is_empty() { "about:blank".to_string() } else { format!("file://{page}") };
    let browser = browser_host_create_browser_sync(Some(&window_info), Some(&mut client), Some(&url.as_str().into()), Some(&browser_settings), None, None)?;
    browser.host()?.set_accessibility_state(State::ENABLED);
    println!("READY pid={} port={port} url={url}", std::process::id());
    Some(Host { browser })
}

pub fn stop(host: Host) {
    host.host().close_browser(1);
    let until = Instant::now() + Duration::from_millis(800);
    while Instant::now() < until {
        do_message_loop_work();
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(host);
    shutdown();
}
