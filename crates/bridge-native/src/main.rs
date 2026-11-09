//! The native Bridge: one window titled "Armada" on the owner's running Fleet.
//!
//! The body is empty on purpose; screens are drawn on top of this by later Jobs.
//! `BRIDGE_NATIVE_EXIT_AFTER_SECS` closes the app after that many seconds, for a run that must not
//! leave a window on someone's screen.

// No screen is drawn yet, so most of what `assets`, `keymap` and `look` offer is not called.
#[allow(dead_code)]
mod assets;
mod fleet;
mod frames;
#[allow(dead_code)]
mod keymap;
mod link;
mod log;
#[allow(dead_code)]
mod look;
mod runtime_file;
mod transport;

/// Written by `pnpm --filter @armada/desktop codegen` from `crates/core-model/domain/`. Not edited.
mod generated {
    pub mod actions;
    pub mod vocabulary;
}

use std::time::Duration;

use gpui::{
    div, prelude::*, px, size, App, Bounds, Context, Entity, TitlebarOptions, Window, WindowBounds,
    WindowOptions,
};
use gpui_platform::application;

use fleet::FleetConnection;
use keymap::KeymapController;
use look::LookController;

/// The window's root. It holds the connection, and what follows the connection, so each lives as
/// long as the window.
struct Shell {
    _fleet: Entity<FleetConnection>,
    _look: Entity<LookController>,
    _keymap: Entity<KeymapController>,
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
    }
}

fn main() {
    application().with_assets(assets::Assets).run(|cx: &mut App| {
        gpui_component::init(cx);
        look::init(cx);

        cx.on_window_closed(|cx, _window| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let bounds = Bounds::centered(None, size(px(1100.), px(760.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Armada".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                let fleet = cx.new(FleetConnection::new);
                let look = cx.new(|cx| LookController::new(&fleet, window, cx));
                let keymap = cx.new(|cx| KeymapController::new(&fleet, cx));
                cx.new(|_| Shell { _fleet: fleet, _look: look, _keymap: keymap })
            },
        )
        .expect("the window opens");
        cx.activate(true);

        if let Some(seconds) = std::env::var("BRIDGE_NATIVE_EXIT_AFTER_SECS")
            .ok()
            .and_then(|seconds| seconds.parse::<u64>().ok())
        {
            cx.spawn(async move |cx| {
                cx.background_executor().timer(Duration::from_secs(seconds)).await;
                cx.update(|cx| cx.quit());
            })
            .detach();
        }
    });
}
