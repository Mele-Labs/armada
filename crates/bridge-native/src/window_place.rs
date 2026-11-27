//! Where the live window was when the app last ran: its size, its place, and whether it was zoomed or
//! full screen, so a restart opens it as he left it. The file is read once, before the window opens;
//! a move or resize is written a moment after the last one, and once more as the app quits. A place no
//! display still shows, or no file at all, opens the window centred as before.

use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui::{point, px, size, App, Bounds, Context, Global, Pixels, Task, Window, WindowBounds};
use serde::{Deserialize, Serialize};

use crate::log::Note;

/// How long after the last move the file is written.
const QUIET: Duration = Duration::from_millis(500);

/// How much of the window's top, from its left edge, must be on a display for the kept place to stand:
/// enough of the title row to take hold of and drag back.
const GRIP: f32 = 44.;

/// The size the window opens at when nothing is kept.
pub const FIRST: (f32, f32) = (1100., 760.);

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum State {
    #[default]
    Windowed,
    Zoomed,
    FullScreen,
}

/// What the file holds. The bounds are the windowed ones, which a zoomed or full-screen window goes back to.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct Place {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    #[serde(default)]
    state: State,
}

impl Place {
    fn of(bounds: WindowBounds) -> Self {
        let (state, at) = match bounds {
            WindowBounds::Windowed(at) => (State::Windowed, at),
            WindowBounds::Maximized(at) => (State::Zoomed, at),
            WindowBounds::Fullscreen(at) => (State::FullScreen, at),
        };
        Place { x: at.origin.x.into(), y: at.origin.y.into(), width: at.size.width.into(), height: at.size.height.into(), state }
    }

    fn bounds(&self) -> Bounds<Pixels> {
        Bounds::new(point(px(self.x), px(self.y)), size(px(self.width), px(self.height)))
    }

    fn window_bounds(&self) -> WindowBounds {
        match self.state {
            State::Windowed => WindowBounds::Windowed(self.bounds()),
            State::Zoomed => WindowBounds::Maximized(self.bounds()),
            State::FullScreen => WindowBounds::Fullscreen(self.bounds()),
        }
    }

    /// Whether some display shows enough of the title row to grab.
    fn on_any(&self, displays: &[Bounds<Pixels>]) -> bool {
        if !(self.width > 0. && self.height > 0. && self.x.is_finite() && self.y.is_finite()) {
            return false;
        }
        let grip = Bounds::new(point(px(self.x), px(self.y)), size(px(self.width.min(GRIP * 4.)), px(GRIP.min(self.height))));
        displays.iter().filter(|display| grip.intersects(display)).any(|display| {
            let shown = grip.intersect(display);
            f32::from(shown.size.width) >= GRIP.min(self.width) && f32::from(shown.size.height) >= GRIP.min(self.height) / 2.
        })
    }
}

struct Kept {
    file: PathBuf,
    last: Option<Place>,
    unwritten: bool,
    wait: Option<Task<()>>,
}

impl Global for Kept {}

/// The file the live app keeps its window's place in.
pub fn file() -> Option<PathBuf> {
    crate::jobs::deck_file().map(|deck| deck.with_file_name("native-window.json"))
}

/// How the window opens: as the file keeps it where a display still shows it, else centred at the first size.
pub fn opening(file: Option<&Path>, cx: &App) -> WindowBounds {
    let displays: Vec<Bounds<Pixels>> = cx.displays().iter().map(|display| display.bounds()).collect();
    match file.and_then(read).filter(|place| place.on_any(&displays)) {
        Some(place) => place.window_bounds(),
        None => WindowBounds::Windowed(Bounds::centered(None, size(px(FIRST.0), px(FIRST.1)), cx)),
    }
}

/// Keep the window's place in `file` from here on, as it moves and as the app quits.
pub fn follow<T: 'static>(file: PathBuf, window: &mut Window, cx: &mut Context<T>) {
    cx.set_global(Kept { file, last: None, unwritten: false, wait: None });
    cx.observe_window_bounds(window, |_, window, cx| moved(Place::of(window.window_bounds()), cx)).detach();
    cx.on_app_quit(|_, cx| {
        let due = cx.try_global::<Kept>().filter(|kept| kept.unwritten).and_then(|kept| kept.last.map(|place| (kept.file.clone(), place)));
        async move {
            if let Some((file, place)) = due {
                write(&file, &place);
            }
        }
    })
    .detach();
}

fn moved(place: Place, cx: &mut App) {
    let Some(kept) = cx.try_global::<Kept>() else { return };
    if kept.last == Some(place) {
        return;
    }
    let kept = cx.global_mut::<Kept>();
    kept.last = Some(place);
    kept.unwritten = true;
    // A newer move replaces the wait, which is the debounce.
    let wait = cx.spawn(async move |cx| {
        cx.background_executor().timer(QUIET).await;
        let due = cx.update(|cx| {
            let kept = cx.global_mut::<Kept>();
            kept.unwritten = false;
            kept.last.map(|place| (kept.file.clone(), place))
        });
        if let Some((file, place)) = due {
            cx.background_executor().spawn(async move { write(&file, &place) }).await;
        }
    });
    cx.global_mut::<Kept>().wait = Some(wait);
}

fn read(path: &Path) -> Option<Place> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return None,
        Err(why) => {
            Note::warn("the window's kept place was not read").field("why", why.to_string()).emit();
            return None;
        }
    };
    ipc::decode::<Place>("the window's kept place", &bytes)
        .map_err(|why| Note::warn("the window's kept place was not understood").field("why", why.to_string()).emit())
        .ok()
}

fn write(path: &Path, place: &Place) {
    let result = ipc::encode(place).map_err(|why| why.to_string()).and_then(|text| crate::crash::write_atomic(path, &text).map_err(|why| why.to_string()));
    if let Err(why) = result {
        Note::warn("the window's place was not written").field("why", why).emit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("armada-window-place-{name}-{}.json", std::process::id()))
    }

    fn display(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        Bounds::new(point(px(x), px(y)), size(px(w), px(h)))
    }

    fn place(x: f32, y: f32, state: State) -> Place {
        Place { x, y, width: 1200., height: 800., state }
    }

    #[test]
    fn a_kept_place_survives_the_file() {
        let file = scratch("kept");
        let kept = place(120., 80., State::Zoomed);
        write(&file, &kept);
        assert_eq!(read(&file), Some(kept));
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn no_file_or_a_broken_one_keeps_nothing() {
        let file = scratch("broken");
        let _ = std::fs::remove_file(&file);
        assert_eq!(read(&file), None);
        std::fs::write(&file, "not json").unwrap();
        assert_eq!(read(&file), None);
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn each_state_opens_as_it_was_kept() {
        let at = place(10., 20., State::Windowed).bounds();
        assert_eq!(place(10., 20., State::Windowed).window_bounds(), WindowBounds::Windowed(at));
        assert_eq!(place(10., 20., State::Zoomed).window_bounds(), WindowBounds::Maximized(at));
        assert_eq!(place(10., 20., State::FullScreen).window_bounds(), WindowBounds::Fullscreen(at));
        assert_eq!(Place::of(WindowBounds::Fullscreen(at)), place(10., 20., State::FullScreen));
    }

    #[test]
    fn a_place_on_a_display_stands() {
        let screens = [display(0., 0., 1512., 982.), display(1512., 0., 2560., 1440.)];
        assert!(place(100., 40., State::Windowed).on_any(&screens));
        assert!(place(2000., 300., State::Windowed).on_any(&screens), "on the second display");
        assert!(place(1400., 40., State::Windowed).on_any(&screens), "across both");
    }

    #[test]
    fn a_place_no_display_shows_falls_back() {
        let laptop = [display(0., 0., 1512., 982.)];
        assert!(!place(2000., 300., State::Windowed).on_any(&laptop), "the second display was unplugged");
        assert!(!place(100., -900., State::Windowed).on_any(&laptop), "title row above the display");
        assert!(!place(1500., 40., State::Windowed).on_any(&laptop), "only a sliver left on the right");
        assert!(!Place { width: 0., ..place(0., 0., State::Windowed) }.on_any(&laptop));
    }
}
