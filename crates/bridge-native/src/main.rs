//! The native Bridge: one window titled "Armada" on the owner's running Fleet.
//!
//! The body is empty on purpose; screens are drawn on top of this by later Jobs.
//! `BRIDGE_NATIVE_EXIT_AFTER_SECS` closes the app after that many seconds, for a run that must not
//! leave a window on someone's screen.

mod fleet;
mod frames;
mod link;
mod runtime_file;
mod transport;

use std::time::Duration;

use gpui::{
    div, prelude::*, px, size, App, Bounds, Context, Entity, TitlebarOptions, Window, WindowBounds,
    WindowOptions,
};
use gpui_platform::application;

use fleet::FleetConnection;

/// The window's root. It holds the connection so the connection lives as long as the window.
struct Shell {
    _fleet: Entity<FleetConnection>,
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
    }
}

fn main() {
    application().run(|cx: &mut App| {
        gpui_component::init(cx);

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
            |_, cx| {
                let fleet = cx.new(FleetConnection::new);
                cx.new(|_| Shell { _fleet: fleet })
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
