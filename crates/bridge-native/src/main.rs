//! The native Bridge: one window titled "Armada".
//!
//! | Run with | What opens |
//! |---|---|
//! | nothing | The Jobs list on the owner's running Fleet |
//! | `--mock <scenario>` | The same window on mock data (`mock.rs`), with no Fleet |
//! | `--walk <name>` | A native walk in its own window with a walk bar (`walk/`). Fleet adds `--job <id> --url <url> --fleet-port <port>` when the owner presses Open |
//! | `--walk <name> --check [--shots <dir>]` | The walk played with no window on screen; exits 1 naming the first step that failed |
//!
//! `BRIDGE_NATIVE_EXIT_AFTER_SECS` closes the app after that many seconds, for a run that must not
//! leave a window on someone's screen.

mod assets;
mod fleet;
mod frames;
mod jobs;
mod keymap;
mod labels;
mod link;
mod log;
// A screen reads only some of what `look` offers so far.
#[allow(dead_code)]
mod look;
mod mock;
mod runtime_file;
mod transport;
mod walk;

/// Written by `pnpm --filter @armada/desktop codegen` from `crates/core-model/domain/`. Not edited.
#[allow(dead_code)]
mod generated {
    pub mod actions;
    pub mod vocabulary;
}

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use gpui::{
    div, prelude::*, px, size, App, Bounds, Context, Entity, TitlebarOptions, Window, WindowBounds,
    WindowOptions,
};
use gpui_platform::application;

use fleet::FleetConnection;
use jobs::{JobsList, JobsScreen};
use keymap::KeymapController;
use labels::label_root;
use look::LookController;
use walk::fleet::WalkFleet;
use walk::window::WalkWindow;

/// What the command line asked for.
enum Mode {
    Live,
    Mock(String),
    Walk { name: String, check: bool, shots: Option<PathBuf>, fleet: Option<WalkFleet> },
}

fn mode_of(args: &[String]) -> Result<Mode, String> {
    let value = |flag: &str| args.iter().position(|arg| arg == flag).map(|at| args.get(at + 1).cloned().ok_or(format!("{flag} needs a value")));
    if let Some(name) = value("--walk").transpose()? {
        let fleet = match (value("--job").transpose()?, value("--url").transpose()?, value("--fleet-port").transpose()?) {
            (Some(job), Some(url), Some(port)) => {
                Some(WalkFleet { job, url, port: port.parse().map_err(|_| format!("--fleet-port {port} is not a port"))? })
            }
            (None, None, None) => None,
            _ => return Err("--job, --url and --fleet-port come together".into()),
        };
        let shots = value("--shots").transpose()?.map(PathBuf::from);
        return Ok(Mode::Walk { name, check: args.iter().any(|arg| arg == "--check"), shots, fleet });
    }
    if let Some(scenario) = value("--mock").transpose()? {
        return Ok(Mode::Mock(scenario));
    }
    Ok(Mode::Live)
}

/// What every window of the app needs before it opens: gpui-component, the look, the keys.
pub fn init_app(cx: &mut App) {
    gpui_component::init(cx);
    look::init(cx);
    walk::window::bind(cx);
    keymap::install(&keymap::Rebinding::default(), cx);
}

/// The window's root on a live Fleet or on mock data.
struct Shell {
    screen: Entity<JobsScreen>,
    _fleet: Option<(Entity<FleetConnection>, Entity<LookController>, Entity<KeymapController>)>,
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        label_root(div().size_full().child(self.screen.clone()))
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = match mode_of(&args) {
        Ok(mode) => mode,
        Err(why) => {
            log::Note::warn("the command line did not read").field("why", why).emit();
            return ExitCode::from(2);
        }
    };
    if let Mode::Walk { name, check: true, shots, .. } = &mode {
        let Some(walk) = walk::named(name) else {
            log::Note::warn("no walk by that name").field("walk", name.as_str()).emit();
            return ExitCode::FAILURE;
        };
        return match walk::check::run(walk, shots.clone()) {
            Ok(()) => {
                log::Note::info("the walk played to its end").field("walk", walk.name).emit();
                ExitCode::SUCCESS
            }
            Err(why) => {
                log::Note::warn("the walk did not play").field("walk", walk.name).field("why", why).emit();
                ExitCode::FAILURE
            }
        };
    }

    application().with_assets(assets::Assets).run(move |cx: &mut App| {
        init_app(cx);
        let bounds = Bounds::centered(None, size(px(1100.), px(760.)), cx);
        let options = |title: String| WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions { title: Some(title.into()), ..Default::default() }),
            ..Default::default()
        };
        let opened = match mode {
            Mode::Live => cx
                .open_window(options("Armada".into()), |window, cx| {
                    let fleet = cx.new(FleetConnection::new);
                    let look = cx.new(|cx| LookController::new(&fleet, window, cx));
                    let keys = cx.new(|cx| KeymapController::new(&fleet, cx));
                    let jobs = cx.new(|cx| JobsList::live(&fleet, cx));
                    let focus = jobs.read(cx).focus_handle().clone();
                    window.focus(&focus, cx);
                    let screen = cx.new(|_| JobsScreen { list: jobs });
                    cx.new(|_| Shell { screen, _fleet: Some((fleet, look, keys)) })
                })
                .map(|_| ()),
            Mode::Mock(scenario) => match mock::jobs(&scenario) {
                Ok(records) => cx
                    .open_window(options("Armada".into()), |window, cx| {
                        let jobs = cx.new(|cx| JobsList::mock(&records, cx));
                        let focus = jobs.read(cx).focus_handle().clone();
                    window.focus(&focus, cx);
                        let screen = cx.new(|_| JobsScreen { list: jobs });
                        cx.new(|_| Shell { screen, _fleet: None })
                    })
                    .map(|_| ()),
                Err(why) => Err(anyhow::anyhow!(why)),
            },
            Mode::Walk { name, fleet, .. } => match walk::named(&name) {
                Some(walk) => {
                    let closing = fleet.clone();
                    cx.on_window_closed(move |_, _| {
                        if let Some(fleet) = &closing {
                            if let Err(why) = fleet.closed() {
                                log::Note::warn("Fleet was not told the walk closed").field("why", why).emit();
                            }
                        }
                    })
                    .detach();
                    cx.open_window(options(format!("{} walk", walk.title)), |window, cx| cx.new(|cx| WalkWindow::new(walk, fleet, window, cx)))
                        .map(|_| ())
                }
                None => Err(anyhow::anyhow!("no walk named {name}")),
            },
        };
        if let Err(why) = opened {
            log::Note::warn("the window did not open").field("why", why.to_string()).emit();
            cx.quit();
            return;
        }
        cx.on_window_closed(|cx, _window| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        cx.activate(true);

        if let Some(seconds) = std::env::var("BRIDGE_NATIVE_EXIT_AFTER_SECS").ok().and_then(|seconds| seconds.parse::<u64>().ok()) {
            cx.spawn(async move |cx| {
                cx.background_executor().timer(Duration::from_secs(seconds)).await;
                cx.update(|cx| cx.quit());
            })
            .detach();
        }
    });
    ExitCode::SUCCESS
}
