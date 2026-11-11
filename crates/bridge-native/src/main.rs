//! The native Bridge: one window titled "Armada".
//!
//! | Run with | What opens |
//! |---|---|
//! | nothing | The Jobs list on the owner's running Fleet |
//! | `--mock <scenario>` | The same window on mock data (`mock.rs`), with no Fleet |
//! | `--walk <name>` | A native walk in its own window with a walk bar (`walk/`). Fleet adds `--job <id> --url <url> --fleet-port <port>` when the owner presses Open, and `--take <name>=<walk>` for each of its variations |
//! | `--walk <name> --check [--shots <dir>] [--hold <ms>] [--width <points>]` | The walk played with no window on screen, 1100 points wide unless `--width` says; exits 1 naming the first step that failed. `--hold` lets each step's animations run before its picture |
//!
//! `BRIDGE_NATIVE_EXIT_AFTER_SECS` closes the app after that many seconds, for a run that must not
//! leave a window on someone's screen.
//!
//! `ARMADA_BRIDGE_QUIT_MARK` names a file the app writes when it quits after its window opened. `scripts/restart`
//! runs the app under launchd through `scripts/bridge-launch`, which reopens it unless that file is there.

mod annotate;
mod assets;
mod banner;
mod capture;
mod chrome;
mod clash_mark;
mod context_ring;
mod deep_link;
mod dictation;
mod dispatch;
mod dropdown;
mod drafts;
// No screen opens a file yet; the first one that does calls it.
#[allow(dead_code)]
mod editor;
mod fleet;
mod frames;
mod job_view;
mod inspect;
mod jobs;
mod keymap;
mod labels;
mod link;
mod log;
// A screen reads only some of what `look` offers so far.
#[allow(dead_code)]
mod look;
mod menu;
mod merge_strip;
mod mock;
mod notify;
mod operator;
mod palette;
mod point;
mod queued;
mod restart_fleet;
mod room;
mod revert;
mod runtime_file;
mod scene;
mod sketch;
mod sketch_png;
mod titlebar;
mod transport;
mod voice;
mod walk;
mod whisper;

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
    div, point, prelude::*, px, size, App, Bounds, Context, Entity, TitlebarOptions, Window, WindowBounds,
    WindowOptions,
};
use gpui_platform::application;

use fleet::FleetConnection;
use jobs::{JobsList, JobsScreen};
use keymap::KeymapController;
use labels::label_root;
use look::{ActiveLook, LookController};
use menu::MenuController;
use notify::Notifier;
use walk::fleet::WalkFleet;
use walk::window::WalkWindow;

/// What `--help` prints, and what an unread command line prints before it exits.
const USAGE: &str = "\
usage: bridge-native [--mock <scenario>]
       bridge-native --walk <name> [--check] [--shots <dir>] [--hold <ms>] [--width <points>] [--take <name>=<walk>]...
                     [--job <id> --url <url> --fleet-port <port>]

With no flags it opens on the Fleet this Mac runs.
  --mock <scenario>   open on a mock scenario's data instead
  --walk <name>       open the walk of that name in its own window
  --check             play the walk with no window on screen; exit 0 when it plays to its end
  --shots <dir>       with --check, keep a picture of each step in <dir>
  --hold <ms>         with --shots, let each step's animations run this long before its picture
  --width <points>    the walk's window width
  --take <name>=<walk>  a take the walk's bar offers, once for each, in the bar's order
  --job, --url, --fleet-port  walk a live Job's page on a running Fleet; the three come together
  -h, --help          print this and exit
";

/// The flags the command line reads that take a value, and those that stand alone.
const VALUED: [&str; 9] = ["--walk", "--job", "--url", "--fleet-port", "--shots", "--hold", "--width", "--take", "--mock"];
const BARE: [&str; 1] = ["--check"];

/// What the command line asked for.
enum Mode {
    Help,
    Live,
    Mock(String),
    Walk { name: String, check: bool, shots: Option<PathBuf>, hold: Option<u64>, width: Option<f32>, fleet: Option<WalkFleet>, takes: Vec<(String, String)> },
}

fn mode_of(args: &[String]) -> Result<Mode, String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return Ok(Mode::Help);
    }
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        if VALUED.contains(&arg.as_str()) {
            rest.next();
        } else if !BARE.contains(&arg.as_str()) && !arg.starts_with("-psn_") {
            // `-psn_0_…` is what LaunchServices may hand an app it opens; it is not his to type.
            return Err(format!("{arg} is not a flag this app reads"));
        }
    }
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
        let width = value("--width").transpose()?.map(|wide| wide.parse::<f32>().map_err(|_| format!("--width {wide} is not a number of points"))).transpose()?;
        let hold = value("--hold").transpose()?.map(|ms| ms.parse::<u64>().map_err(|_| format!("--hold {ms} is not a number of milliseconds"))).transpose()?;
        // `--take <name>=<walk>`, once for each take, in the bar's order.
        let takes = args
            .windows(2)
            .filter(|pair| pair[0] == "--take")
            .map(|pair| pair[1].rsplit_once('=').map(|(name, walk)| (name.to_string(), walk.to_string())).ok_or(format!("--take {} is not <name>=<walk>", pair[1])))
            .collect::<Result<_, _>>()?;
        return Ok(Mode::Walk { name, check: args.iter().any(|arg| arg == "--check"), shots, hold, width, fleet, takes });
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
    jobs::bind(cx);
    job_view::bind(cx);
    titlebar::bind(cx);
    merge_strip::bind(cx);
    annotate::bind(cx);
    dispatch::screen::bind(cx);
    keymap::install(&keymap::Rebinding::default(), cx);
    menu::install(cx);
}

/// The window's root on a live Fleet or on mock data.
struct Shell {
    screen: gpui::AnyView,
    _fleet: Option<(Entity<FleetConnection>, Entity<LookController>, Entity<KeymapController>, Entity<MenuController>, Entity<Notifier>)>,
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // What a library widget inherits; gpui's own text colour is black.
        let text = cx.look().color(look::token::FG_DEFAULT);
        label_root(div().size_full().text_color(text).child(self.screen.clone()))
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = match mode_of(&args) {
        Ok(mode) => mode,
        Err(why) => {
            eprintln!("{why}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    if let Mode::Help = mode {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    if let Mode::Walk { name, check: true, shots, hold, width, takes, .. } = &mode {
        let Some(walk) = walk::named(name) else {
            log::Note::warn("no walk by that name").field("walk", name.as_str()).emit();
            return ExitCode::FAILURE;
        };
        return match walk::check::run(walk, walk::window::takes_of(takes), shots.clone(), *hold, *width) {
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

    let app = application().with_assets(assets::Assets);
    // macOS hands links over before the app is ready and on its own thread of control, so they cross a
    // channel to a task that reads them where the app can be reached. Nothing registers the scheme with
    // the OS, so none arrives unless something hands this app one (`deep_link.rs`).
    let (links, arrived) = async_channel::unbounded::<String>();
    app.on_open_urls(move |addresses| {
        for address in addresses {
            let _ = links.try_send(address);
        }
    });
    app.run(move |cx: &mut App| {
        init_app(cx);
        cx.spawn(async move |cx| {
            while let Ok(address) = arrived.recv().await {
                match deep_link::arrive(&address) {
                    Some(link) => cx.update(|cx| deep_link::deliver(&link, cx)),
                    None => log::Note::info("an address this app does not open was handed to it").field("address", address).emit(),
                }
            }
        })
        .detach();
        let bounds = Bounds::centered(None, size(px(1100.), px(760.)), cx);
        let options = |title: String| WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            // Frameless, as Electron's `titleBarStyle: "hiddenInset"`: the app draws its own title row, and the
            // traffic lights sit where Electron puts them, 16 down a 44px row (`apps/desktop/src/main/index.ts`).
            titlebar: Some(TitlebarOptions {
                title: Some(title.into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(18.), px(16.))),
            }),
            ..Default::default()
        };
        let opened = match mode {
            // Printed and gone before the app starts.
            Mode::Help => Ok(()),
            Mode::Live => cx
                .open_window(options("Armada".into()), |window, cx| {
                    let fleet = cx.new(FleetConnection::new);
                    let look = cx.new(|cx| LookController::new(&fleet, window, cx));
                    let keys = cx.new(|cx| KeymapController::new(&fleet, cx));
                    let menu = cx.new(|cx| MenuController::new(&fleet, cx));
                    MenuController::handle_revert(&menu, cx);
                    let jobs = cx.new(|cx| JobsList::live(&fleet, cx));
                    let focus = jobs.read(cx).focus_handle().clone();
                    window.focus(&focus, cx);
                    let notifier = cx.new(|cx| Notifier::new(&fleet, &jobs, window.window_handle(), cx));
                    let jobs_screen = cx.new(|cx| JobsScreen::new(jobs, None, window, cx));
                    let operator: gpui::AnyView = cx.new(|cx| operator::OperatorScreen::live(jobs_screen.clone(), &fleet, window, cx)).into();
                    // The sheet over the Operator's page: ⌘N and `n` open it, and its Start goes to this Fleet.
                    let screen = cx.new(|cx| dispatch::DispatchScreen::new(jobs_screen, &[], window, cx).over(operator));
                    cx.new(|_| Shell { screen: screen.into(), _fleet: Some((fleet, look, keys, menu, notifier)) })
                })
                .map(|_| ()),
            Mode::Mock(scenario) => match mock::jobs(&scenario).and_then(|records| Ok((records, mock::threads(&scenario)?))) {
                Ok((records, threads)) => cx
                    .open_window(options("Armada".into()), |window, cx| {
                        let jobs = cx.new(|cx| JobsList::mock(&records, cx));
                        let focus = jobs.read(cx).focus_handle().clone();
                    window.focus(&focus, cx);
                        let screen = cx.new(|cx| JobsScreen::new(jobs, Some(threads), window, cx)).into();
                        cx.new(|_| Shell { screen, _fleet: None })
                    })
                    .map(|_| ()),
                Err(why) => Err(anyhow::anyhow!(why)),
            },
            Mode::Walk { name, fleet, takes, .. } => match walk::named(&name) {
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
                    cx.open_window(options(format!("{} walk", walk.title)), |window, cx| cx.new(|cx| WalkWindow::new(walk, fleet, walk::window::takes_of(&takes), window, cx)))
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
        // A quit the person meant leaves this mark, so `scripts/bridge-launch` can tell it from a crash
        // or a kill and have launchd reopen the app only then. Not written when the window did not open.
        if let Some(mark) = std::env::var_os("ARMADA_BRIDGE_QUIT_MARK") {
            cx.on_app_quit(move |_| {
                let _ = std::fs::write(&mark, "");
                std::future::ready(())
            })
            .detach();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_walk_reads_each_take_as_a_name_and_a_walk_in_the_bars_order() {
        let args: Vec<String> =
            ["--walk", "jobs-list", "--take", "List=jobs-list", "--take", "Hull tint=jobs-lamps"].map(String::from).to_vec();
        let Ok(Mode::Walk { takes, .. }) = mode_of(&args) else { panic!("a walk") };
        assert_eq!(takes, vec![("List".into(), "jobs-list".into()), ("Hull tint".into(), "jobs-lamps".into())]);
        let walks = walk::window::takes_of(&takes);
        assert_eq!(walks.iter().map(|take| take.walk.name).collect::<Vec<_>>(), ["jobs-list", "jobs-lamps"]);

        let bad: Vec<String> = ["--walk", "jobs-list", "--take", "Pennant"].map(String::from).to_vec();
        assert!(mode_of(&bad).is_err(), "a take with no walk");
    }

    #[test]
    fn help_is_read_anywhere_and_an_unknown_flag_is_refused() {
        let args = |line: &[&str]| line.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        assert!(matches!(mode_of(&args(&["--help"])), Ok(Mode::Help)));
        assert!(matches!(mode_of(&args(&["--walk", "jobs-list", "-h"])), Ok(Mode::Help)));
        assert_eq!(mode_of(&args(&["--walks", "jobs-list"])).err().as_deref(), Some("--walks is not a flag this app reads"));
        assert!(mode_of(&args(&["--walk", "jobs-list", "--check", "--width", "800"])).is_ok(), "a value is not read as a flag");
        assert!(matches!(mode_of(&args(&["-psn_0_123456"])), Ok(Mode::Live)), "LaunchServices' own argument passes");
    }
}
