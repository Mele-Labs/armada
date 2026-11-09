//! **A stand-in for the Job head.** The Job head is another Job's, and it is not on this branch, so
//! `context_ring::ContextRing` is mounted nowhere in the app yet. This draws the ring on the Jobs
//! screen's centre, in a bar the size of a head's tools row, so the walk shows the component at its
//! real size where the head will carry it. Nothing here ships: it is a walk's stage only.
//!
//! Its scenes set what a head would hand the ring, as the title bar's scenes set the bar's face.

use std::rc::Rc;

use gpui::{div, prelude::*, AnyView, App, Context, Entity, Global, Role, SharedString, WeakEntity, Window};
use ipc::{CompactHow, CompactOff, CompactOffer, ContextBreakdown, SessionUsage};

use crate::context_ring::{ContextRing, Fill};
use crate::labels::{labelled, LabelBook};
use crate::look::{token, ActiveLook};

const JOB: &str = "mock-02";

fn breakdown() -> ContextBreakdown {
    let json = r#"{
        "categories":[
            {"name":"System prompt","tokens":8100,"kind":"used"},
            {"name":"System tools","tokens":14200,"kind":"used"},
            {"name":"MCP tools","tokens":5300,"kind":"used"},
            {"name":"Memory files","tokens":6400,"kind":"used"},
            {"name":"Messages","tokens":92000,"kind":"used"},
            {"name":"Free space","tokens":41000,"kind":"free"},
            {"name":"Autocompact buffer","tokens":33000,"kind":"buffer"},
            {"name":"Deferred tools","tokens":12000,"kind":"deferred"}
        ],
        "memory_files":[
            {"path":"/Users/nick/Development/armada/CLAUDE.md","tokens":2900},
            {"path":"/Users/nick/.claude/CLAUDE.md","tokens":1100},
            {"path":"/Users/nick/.claude/projects/armada/memory/MEMORY.md","tokens":1700},
            {"path":"/Users/nick/Development/armada/crates/bridge-native/CLAUDE.md","tokens":700}
        ],
        "mcp_tools":[
            {"name":"list_jobs","server":"armada-fleet","tokens":1400},
            {"name":"show_window","server":"armada-fleet","tokens":1200},
            {"name":"record_plan","server":"armada-fleet","tokens":900},
            {"name":"query","server":"gitnexus","tokens":800},
            {"name":"resolve-library-id","server":"context7","tokens":500}
        ],
        "total_tokens":126000,
        "max_tokens":200000,
        "autocompact_threshold":167000
    }"#;
    ipc::decode::<ContextBreakdown>("ContextBreakdown", json.as_bytes()).unwrap_or_else(|why| panic!("the scenario's breakdown does not decode: {why}"))
}

/// The figures Fleet reports for a Job at `tokens` of a 200k window, with its percent and band as
/// Fleet reads them.
fn usage(tokens: u64, percent: u8, band: &str) -> SessionUsage {
    let json = format!(r#"{{"context_tokens":{tokens},"context_window":200000,"context_percent":{percent},"context_band":"{band}"}}"#);
    ipc::decode::<SessionUsage>("SessionUsage", json.as_bytes()).unwrap_or_else(|why| panic!("the scenario's usage does not decode: {why}"))
}

fn fill(tokens: u64, percent: u8, band: &str, breakdown: Option<ContextBreakdown>, compact: Option<CompactOffer>) -> Fill {
    Fill::of(&usage(tokens, percent, band), breakdown.as_ref(), compact).expect("a window to fill")
}

fn offered() -> Option<CompactOffer> {
    Some(CompactOffer::Offered { how: CompactHow::Message })
}

/// What a head would hand the ring in a named scene.
fn scene(name: &str) -> Option<Fill> {
    Some(match name {
        "ring-normal" => fill(126_000, 63, "normal", Some(breakdown()), offered()),
        "ring-warning" => fill(168_000, 84, "warning", Some(breakdown()), offered()),
        "ring-danger" => fill(191_000, 96, "danger", Some(breakdown()), offered()),
        "ring-off" => fill(126_000, 63, "normal", Some(breakdown()), Some(CompactOffer::Off { why: CompactOff::Ended })),
        "ring-refused" => Fill { refused: Some("Compact not sent: the agent is not taking messages".into()), ..fill(126_000, 63, "normal", Some(breakdown()), offered()) },
        "ring-popover" => fill(126_000, 63, "normal", None, offered()),
        "ring-tip" => fill(126_000, 63, "normal", None, None),
        _ => return None,
    })
}

/// The window's one stand-in, for the walk's scenes.
struct Live(WeakEntity<JobHeadStandIn>);

impl Global for Live {}

pub struct JobHeadStandIn {
    screen: AnyView,
    fill: Option<Fill>,
}

impl JobHeadStandIn {
    pub fn new(screen: AnyView, cx: &mut Context<Self>) -> JobHeadStandIn {
        cx.set_global(Live(cx.entity().downgrade()));
        JobHeadStandIn { screen, fill: None }
    }
}

/// Move the stand-in to a named scene: what the head hands the ring.
pub fn apply(name: &str, cx: &mut App) -> Result<(), String> {
    let next = scene(name).ok_or_else(|| format!("no ring scene named {name}"))?;
    let held: Entity<JobHeadStandIn> = cx.try_global::<Live>().and_then(|live| live.0.upgrade()).ok_or("no stand-in for the Job head is drawn yet")?;
    held.update(cx, |stand_in, cx| {
        stand_in.fill = Some(next);
        cx.notify();
    });
    Ok(())
}

impl Render for JobHeadStandIn {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        // The head sits at the centre's start: the right edge of the list's handle, as it was last drawn.
        let from = cx
            .try_global::<LabelBook>()
            .and_then(|book| book.find(window.window_handle().window_id(), gpui::Role::Splitter, "Resize the Jobs list"))
            .map(|handle| handle.bounds.origin.x + handle.bounds.size.width + look.length(token::SPACE_2));
        let ring = self.fill.clone().zip(from).map(|(fill, left)| {
            let hold: Rc<dyn Fn(CompactHow, &mut Window, &mut App)> = Rc::new(|_, _, _| {});
            div()
                .absolute()
                .top(look.length(token::H_TITLE_BAR) + look.length(token::SPACE_3))
                .left(left)
                .w(look.length(token::W_DIALOG))
                .child(labelled(
                    div()
                        .id("job-head-stand-in")
                        .flex()
                        .items_center()
                        .justify_end()
                        .h(look.length(token::H_CONTROL_LG) + look.length(token::SPACE_2))
                        .px(look.length(token::SPACE_3))
                        .rounded(look.length(token::RADIUS_LG))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_DEFAULT))
                        .bg(look.color(token::BG_RAISED))
                        .child(ContextRing::new(SharedString::from(JOB), Some(fill)).on_compact(hold)),
                    Role::Group,
                    "Job head",
                ))
        });
        div().relative().size_full().child(self.screen.clone()).children(ring)
    }
}
