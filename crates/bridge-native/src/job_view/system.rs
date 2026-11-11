//! A Job's System tab (`SystemLayer` over `SystemSections`): its time split, the processes it
//! started, what they cost the machine, the bays it holds and the logs Fleet keeps for it.
//!
//! **`GET /sessions/system` is read every 5 s only while the tab is in front of the Job shown**; the
//! loop ends when the tab is hidden, shut or the Job changes. A refused read is a toast in Fleet's
//! words, once a run of refusals, and the sections draw nothing until a read is answered. Fleet's
//! `SystemProcess` carries no CPU, memory or I/O of its own, so Columns offers Port alone.

use std::collections::{HashMap, VecDeque};
use std::time::Duration;

use chrono::Local;
use gpui::{canvas, div, point, prelude::*, px, AnyElement, Bounds, Context, FontWeight, PathBuilder, Pixels, Role, SharedString, Task, Window};
use ipc::{JobSplit, JobTime, RecordSplit, SessionSystem, StopProcess, SystemLog, SystemProcess, SystemProcessState};

use super::pr_panel::{icon_button, stand_ins};
use super::rows::{glyph, mono, tip, Size};
use super::side::Zone;
use super::JobView;
use crate::dropdown::Entry;
use crate::labels::labelled;
use crate::look::{token, Look, Token};

/// How often a tab in front reads again (`POLL_MS`).
const EVERY: Duration = Duration::from_secs(5);
/// How many readings a graph keeps: five minutes at the poll rate.
const HISTORY: usize = 60;
const GB: f64 = 1024. * 1024. * 1024.;
const MB: f64 = 1024. * 1024.;

/// What one Job's tab holds between reads.
#[derive(Default)]
pub(super) struct SystemHeld {
    read: Option<SessionSystem>,
    /// When the last answered read came, `HH:MM:SS`.
    read_at: Option<String>,
    /// The last read was refused: no skeleton, no sections, until one is answered.
    refused: bool,
    cpu: VecDeque<f64>,
    /// In GB.
    memory: VecDeque<f64>,
    /// Processes stopped here, kept as stopped though Fleet no longer lists them.
    stopped: Vec<SystemProcess>,
}

#[derive(Default)]
pub(super) struct Systems {
    held: HashMap<SharedString, SystemHeld>,
    /// The read loop, and the Job it reads for.
    poll: Option<(SharedString, Task<()>)>,
    /// Each Job's split, read once as its tab shows; `None` where Fleet keeps none.
    times: HashMap<SharedString, Option<JobSplit>>,
    /// Columns hid the Port column.
    hide_port: bool,
}

impl Systems {
    /// Stops the read loop: its tab was closed.
    pub(super) fn stop(&mut self) {
        self.poll = None;
    }
}

/// `38%`, or `1.8 of 8 GB`.
fn figure_percent(used: f64) -> String {
    format!("{}%", used.round() as i64)
}

fn figure_gb(used: f64, of: f64) -> String {
    format!("{used:.1} of {} GB", of.round() as i64)
}

/// A size as `sizeOf` draws it: GB or MB to a decimal, else whole KB, never under 1.
pub(super) fn size_of(bytes: u64) -> String {
    let bytes = bytes as f64;
    if bytes >= GB {
        format!("{:.1} GB", bytes / GB)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes / MB)
    } else {
        format!("{} KB", ((bytes / 1024.).round() as i64).max(1))
    }
}

/// The top of the CPU graph: 100%, or the next hundred above where several cores are used.
fn cpu_top(cpu: f64) -> f64 {
    (((cpu / 100.).ceil()) * 100.).max(100.)
}

/// The nine buckets of a split, in seconds, with their names and what each means (`usage-time.ts`).
const BUCKETS: [(&str, &str, &str); 9] = [
    ("model", "Model", "Waiting on the model's answer"),
    ("tools", "Tools", "A tool ran in the main thread"),
    ("checks", "Checks", "A Check queued or ran"),
    ("subagents", "Subagents", "The main thread waited on a subagent"),
    ("slot", "Slot", "Waiting for a slot, or for Fleet to move the Job into one"),
    ("you", "You", "Waiting on the owner: a question, a walk, a plan, a start card"),
    ("pull_request", "Pull request", "CI and the merge queue, with nothing else running"),
    ("stopped", "Stopped", "The Job's agent was not running, or Fleet was down, and nothing was asked"),
    ("idle", "Idle", "Nothing running and nothing asked"),
];

fn seconds(split: &RecordSplit) -> Vec<(&'static str, f64)> {
    split.each().into_iter().map(|(name, ms)| (name, ms as f64 / 1000.)).collect()
}

/// `2h 05m`, `48m` or `<1m`.
fn span(seconds: f64) -> String {
    let minutes = (seconds / 60.).round() as i64;
    if seconds > 0. && minutes < 1 {
        return "<1m".into();
    }
    if minutes < 60 {
        return format!("{minutes}m");
    }
    let rest = minutes % 60;
    if rest == 0 {
        format!("{}h", minutes / 60)
    } else {
        format!("{}h {rest:02}m", minutes / 60)
    }
}

/// The same for a reader: `2 hours 5 minutes`.
fn span_said(seconds: f64) -> String {
    let minutes = (seconds / 60.).round() as i64;
    if minutes < 1 {
        return if seconds > 0. { "under a minute".into() } else { "none".into() };
    }
    let part = |n: i64, unit: &str| format!("{n} {unit}{}", if n == 1 { "" } else { "s" });
    let (hours, rest) = (minutes / 60, minutes % 60);
    let mut said = Vec::new();
    if hours > 0 {
        said.push(part(hours, "hour"));
    }
    if rest > 0 {
        said.push(part(rest, "minute"));
    }
    said.join(" ")
}

/// Far over its usual: twice it and ten minutes beyond.
fn far_over(spent: f64, usual: Option<f64>) -> bool {
    usual.is_some_and(|usual| spent >= usual * 2. && spent - usual >= 600.)
}

/// A bucket's hue (`TimeView.css`): four carry one, five are steps of the ink toward the ground.
fn bucket_hue(look: &Look, bucket: &str) -> gpui::Hsla {
    let ink = |share: f32| look.color(token::BG_SUNKEN).blend(look.color(token::FG_DEFAULT).opacity(share));
    match bucket {
        "model" => look.color(token::ACCENT),
        "tools" => ink(0.42),
        "checks" => look.color(token::TOOL_CHANGE),
        "subagents" => ink(0.95),
        "slot" => ink(0.65),
        "you" => look.color(token::YOU),
        "pull_request" => look.color(token::BG_SUNKEN).blend(look.color(token::OPERATOR).opacity(0.7)),
        "stopped" => ink(0.33),
        _ => look.color(token::BG_HOVER),
    }
}

/// A section (`Section`): its heading in mono, a note after it, something at its end, then its body.
pub(super) fn section(look: &Look, label: &'static str, note: Option<String>, aside: Option<AnyElement>, body: AnyElement) -> AnyElement {
    let heading = div()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_1))
        .px(look.length(token::SPACE_2))
        .font_family(mono(look))
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .text_color(look.color(token::FG_SUBTLE))
        .child(label)
        .children(note.map(|note| div().font_weight(FontWeight::NORMAL).child(SharedString::from(format!("\u{b7} {note}")))))
        .child(div().flex_1())
        .children(aside);
    labelled(div().id(label).flex().flex_col().gap(look.length(token::SPACE_1)).child(heading).child(body), Role::Group, label).into_any_element()
}

/// The last readings as a filled line, newest at the right edge; `top` is the top of the box.
fn graph(look: &Look, name: &'static str, past: &VecDeque<f64>, top: f64) -> AnyElement {
    let points: Vec<f64> = past.iter().copied().collect();
    let (line, area) = (look.color(token::ACCENT), look.color(token::ACCENT).opacity(0.25));
    let count = points.len();
    let drawn = canvas(
        |_, _, _| {},
        move |bounds: Bounds<Pixels>, _, window, _| {
            if count < 2 {
                return;
            }
            let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
            let first = HISTORY - count;
            let at = |n: f64, i: usize| {
                let x = (i as f32 / (HISTORY - 1) as f32) * w;
                let y = h - (n / top).min(1.) as f32 * h;
                point(bounds.origin.x + px(x), bounds.origin.y + px(y))
            };
            let mut fill = PathBuilder::fill();
            fill.move_to(point(at(0., first).x, bounds.origin.y + px(h)));
            for (i, n) in points.iter().enumerate() {
                fill.line_to(at(*n, first + i));
            }
            fill.line_to(point(bounds.origin.x + px(w), bounds.origin.y + px(h)));
            fill.close();
            if let Ok(path) = fill.build() {
                window.paint_path(path, area);
            }
            let mut stroke = PathBuilder::stroke(px(1.5));
            stroke.move_to(at(points[0], first));
            for (i, n) in points.iter().enumerate().skip(1) {
                stroke.line_to(at(*n, first + i));
            }
            if let Ok(path) = stroke.build() {
                window.paint_path(path, line);
            }
        },
    )
    .w_full()
    .h(px(32.));
    labelled(div().id(SharedString::from(format!("graph-{name}"))).px(look.length(token::SPACE_2)).child(drawn), Role::Image, format!("{name}, last {count} readings")).into_any_element()
}

/// A meter (`armada-usage__meter`): the share of `of` used.
fn meter(look: &Look, name: &'static str, used: f64, of: f64, said: String) -> AnyElement {
    let share = if of > 0. { (used / of).clamp(0., 1.) as f32 } else { 0. };
    labelled(
        div().id(SharedString::from(format!("meter-{name}"))).px(look.length(token::SPACE_2)).py(look.length(token::SPACE_2)).child(
            div()
                .w_full()
                .h(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_SM))
                .bg(look.color(token::BG_SUNKEN))
                .child(div().h_full().w(gpui::relative(share)).rounded(look.length(token::RADIUS_SM)).bg(look.color(token::ACCENT))),
        ),
        Role::Meter,
        format!("{name}, {said}"),
    )
    .into_any_element()
}

impl JobView {
    /// The tab came in front, or the Job changed: read the split once, and start the read loop for
    /// this Job where none runs. A tab not in front ends the loop.
    pub(super) fn want_system(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let front = self.shown.clone().filter(|_| self.zone_shown() == Some(Zone::System));
        let Some(job) = front else {
            self.panes.systems.poll = None;
            return;
        };
        if !self.panes.systems.times.contains_key(&job) {
            self.read_time(job.clone(), window, cx);
        }
        if self.panes.systems.poll.as_ref().is_some_and(|(polled, _)| *polled == job) {
            return;
        }
        if self.fleet.is_none() {
            self.mock_system(job, cx);
            return;
        }
        let Some(port) = self.port(cx) else { return };
        let path = format!("/sessions/system?session_id={}", super::wire::encoded(&job));
        let reading = job.clone();
        let task = cx.spawn(async move |this, cx| loop {
            let path = path.clone();
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::get(port, &path))?;
                    if !(200..300).contains(&status) {
                        return Err(super::wire::reason(status, &bytes));
                    }
                    ipc::decode::<SessionSystem>("SessionSystem", &bytes).map_err(|why| why.to_string())
                })
                .await;
            let still = this.update(cx, |this, cx| {
                let front = this.shown.as_ref() == Some(&reading) && this.zone_shown() == Some(Zone::System);
                if front {
                    this.take_system(&reading, answer, cx);
                } else if this.panes.systems.poll.as_ref().is_some_and(|(polled, _)| *polled == reading) {
                    // A loop that ends leaves no handle, so the tab shown again starts another.
                    this.panes.systems.poll = None;
                }
                front
            });
            if !still.unwrap_or(false) {
                return;
            }
            cx.background_executor().timer(EVERY).await;
        });
        self.panes.systems.poll = Some((job, task));
    }

    /// An answer: the reading and the graphs' next points, or a refusal said once.
    fn take_system(&mut self, job: &SharedString, answer: Result<SessionSystem, String>, cx: &mut Context<Self>) {
        let held = self.panes.systems.held.entry(job.clone()).or_default();
        match answer {
            Ok(system) => {
                if let Some(cpu) = system.cpu_percent {
                    held.cpu.push_back(cpu as f64);
                }
                if let Some(memory) = system.memory_bytes {
                    held.memory.push_back(memory as f64 / GB);
                }
                while held.cpu.len() > HISTORY {
                    held.cpu.pop_front();
                }
                while held.memory.len() > HISTORY {
                    held.memory.pop_front();
                }
                held.read = Some(system);
                held.read_at = Some(Local::now().format("%H:%M:%S").to_string());
                held.refused = false;
            }
            Err(why) => {
                let first = !held.refused;
                held.read = None;
                held.read_at = None;
                held.refused = true;
                if first {
                    cx.defer(move |cx| crate::toasts::tell(why, cx));
                }
            }
        }
        cx.notify();
    }

    /// The mock's Fleet: its reading for the Job, after the wait the scenario names, with the graphs'
    /// past readings, or its refusal.
    fn mock_system(&mut self, job: SharedString, cx: &mut Context<Self>) {
        if self.panes.systems.held.get(&job).is_some_and(|held| held.read.is_some() || held.refused) {
            return;
        }
        let Some(seed) = crate::mock::system(&job) else { return };
        let wait = Duration::from_millis(seed.first_read_ms);
        let reading = job.clone();
        let task = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(wait).await;
            let _ = this.update(cx, |this, cx| {
                let answer = match (seed.system, seed.refused) {
                    (_, Some(why)) => Err(why),
                    (Some(system), None) => Ok(system),
                    (None, None) => return,
                };
                // The seeded past runs up to this reading, which the answer adds.
                let held = this.panes.systems.held.entry(reading.clone()).or_default();
                held.cpu = seed.cpu.iter().copied().collect();
                held.memory = seed.memory.iter().copied().collect();
                held.cpu.pop_back();
                held.memory.pop_back();
                this.take_system(&reading, answer, cx);
            });
        });
        self.panes.systems.poll = Some((job, task));
    }

    /// `GET /sessions/time`: the Job's split, read once. None kept, nothing drawn.
    fn read_time(&mut self, job: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        if self.fleet.is_none() {
            let split = crate::mock::job_time(&job).and_then(|time| time.time);
            self.panes.systems.times.insert(job, split);
            return;
        }
        let path = format!("/sessions/time?session_id={}", super::wire::encoded(&job));
        self.get(
            path,
            window,
            cx,
            Box::new(move |this, answer, _, cx| {
                let split = answer.ok().and_then(|bytes| ipc::decode::<JobTime>("JobTime", &bytes).ok()).and_then(|time| time.time);
                this.panes.systems.times.insert(job, split);
                cx.notify();
            }),
        );
    }

    /// Stop one process the Job started (`POST /sessions/stop_process`). It stays in the list as
    /// stopped; a refusal is a toast.
    fn stop_process(&mut self, pid: u32, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let of = job.clone();
        let stopped = move |this: &mut JobView, cx: &mut Context<JobView>| {
            let held = this.panes.systems.held.entry(of.clone()).or_default();
            if let Some(read) = held.read.as_mut() {
                if let Some(at) = read.processes.iter().position(|one| one.pid == pid) {
                    let mut one = read.processes.remove(at);
                    one.state = SystemProcessState::Stopped;
                    held.stopped.push(one);
                }
            }
            cx.notify();
        };
        if self.fleet.is_none() {
            stopped(self, cx);
            return;
        }
        let body = ipc::encode(&StopProcess { session_id: Some(ipc::SessionId::carried(job.to_string())), pid: Some(pid), port: None });
        self.post(
            "/sessions/stop_process".into(),
            body,
            window,
            cx,
            Box::new(move |this, answer, _, cx| match answer {
                Ok(_) => stopped(this, cx),
                Err(why) => cx.defer(move |cx| crate::toasts::tell(why, cx)),
            }),
        );
    }

    fn pick_columns(&mut self, at: gpui::Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let entries = vec![Entry::Choice { id: "column:port", label: "Port".into(), on: !self.panes.systems.hide_port }];
        let back = window.focused(cx);
        self.open_button_menu("Columns", entries, at, back, |this, picked, _, cx| {
            if picked == "column:port" {
                this.panes.systems.hide_port = !this.panes.systems.hide_port;
                cx.notify();
            }
        }, window, cx);
    }

    /// System: the sections in Classic's order, each left out where its figure is absent.
    pub(super) fn pane_system(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let held = self.panes.systems.held.get(id);
        let time = self.panes.systems.times.get(id).cloned().flatten().map(|split| {
            let title = self.record(id, cx).map_or_else(|| id.to_string(), |job| job.title.clone().unwrap_or_else(|| id.to_string()));
            section(look, "Time", None, None, self.job_split(&title, &split, look))
        });
        let mut sections: Vec<AnyElement> = time.into_iter().collect();
        let Some(read) = held.and_then(|held| held.read.as_ref()) else {
            if held.is_some_and(|held| held.refused) {
                return (!sections.is_empty()).then(|| self.system_frame(sections, look));
            }
            sections.push(labelled(div().id("system-reading").flex().flex_col().gap(look.length(token::SPACE_4)).children(
                [("Processes", 5), ("CPU", 1), ("Memory", 1), ("Disk", 1)].into_iter().map(|(label, rows)| {
                    let widths: Vec<f32> = (0..rows).map(|at| 0.9 - at as f32 * 0.07).collect();
                    section(look, label, None, None, div().px(look.length(token::SPACE_2)).py(look.length(token::SPACE_1)).child(stand_ins(look, &format!("system-{label}"), &widths)).into_any_element())
                }),
            ), Role::Group, "Reading the system").into_any_element());
            return Some(self.system_frame(sections, look));
        };
        let held = held?;
        let columns = labelled(
            icon_button(look, "system-columns", "sliders-horizontal", "Columns".into()).on_click(cx.listener(|this, event: &gpui::ClickEvent, window, cx| {
                this.pick_columns(event.position(), window, cx);
            })),
            Role::Button,
            "Columns",
        )
        .into_any_element();
        sections.push(section(look, "Processes", held.read_at.clone().map(|at| format!("read {at}")), Some(columns), self.process_table(read, &held.stopped, look, cx)));
        if let Some(cpu) = read.cpu_percent {
            let used = cpu as f64;
            sections.push(section(look, "CPU", None, Some(div().child(SharedString::from(figure_percent(used))).into_any_element()), graph(look, "CPU", &held.cpu, cpu_top(used))));
        }
        if let (Some(used), Some(of)) = (read.memory_bytes, read.memory_of_bytes) {
            let (used, of) = (used as f64 / GB, (of as f64 / GB).round());
            sections.push(section(look, "Memory", None, Some(div().child(SharedString::from(figure_gb(used, of))).into_any_element()), graph(look, "Memory", &held.memory, of)));
        }
        if let (Some(used), Some(of)) = (read.disk_bytes, read.disk_of_bytes) {
            let (used, of) = (used as f64 / GB, (of as f64 / GB).round());
            let said = figure_gb(used, of);
            sections.push(section(look, "Disk", None, Some(div().child(SharedString::from(said.clone())).into_any_element()), meter(look, "Disk", used, of, said)));
        }
        let bays = self.held_bays(id, cx);
        if !bays.is_empty() {
            let rows: Vec<AnyElement> = bays.iter().map(|bay| self.berth_row(bay, "system", look, cx)).collect();
            sections.push(section(look, "Worktrees", None, None, div().flex().flex_col().gap(look.length(token::SPACE_2)).children(rows).into_any_element()));
        }
        if !read.logs.is_empty() {
            let rows: Vec<AnyElement> = read.logs.iter().map(|one| self.log_row(one, look, cx)).collect();
            sections.push(section(look, "Logs", None, None, div().flex().flex_col().children(rows).into_any_element()));
        }
        Some(self.system_frame(sections, look))
    }

    fn system_frame(&self, sections: Vec<AnyElement>, look: &Look) -> AnyElement {
        div()
            .id("pane-system")
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_SM))
            .children(sections)
            .into_any_element()
    }

    /// The process table: a fixed box of eight rows that scrolls, under a header that stays.
    fn process_table(&self, read: &SessionSystem, stopped: &[SystemProcess], look: &Look, cx: &Context<Self>) -> AnyElement {
        let port = !self.panes.systems.hide_port;
        let row_h = look.length(token::H_ROW);
        let cells = |command: AnyElement, port_cell: Option<AnyElement>, pid: AnyElement, mark: AnyElement, act: AnyElement| {
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .min_h(row_h)
                .px(look.length(token::SPACE_2))
                .py(look.length(token::SPACE_1))
                .child(div().flex_1().min_w_0().child(command))
                .children(port_cell.map(|cell| div().flex_none().w(look.length(token::SPACE_8) + look.length(token::SPACE_2)).child(cell)))
                .child(div().flex_none().w(look.length(token::SPACE_12)).child(pid))
                .child(div().flex_none().w(look.length(token::SPACE_6)).flex().justify_center().child(mark))
                .child(div().flex_none().w(look.length(token::SPACE_8)).flex().justify_center().child(act))
        };
        let head_word = |said: &'static str| div().text_size(look.length(token::TEXT_2XS)).font_family(mono(look)).text_color(look.color(token::FG_SUBTLE)).child(said).into_any_element();
        let header = cells(head_word("Command"), port.then(|| head_word("Port")), head_word("PID"), div().into_any_element(), div().into_any_element())
            .bg(look.color(token::BG_RAISED))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE));
        let figure = |text: String| div().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(text)).into_any_element();
        let rows = read.processes.iter().chain(stopped.iter()).map(|one| {
            let running = one.state == SystemProcessState::Running;
            let (icon, said, ink): (&'static str, &'static str, Token) = if running { ("circle-dot", "Running", token::STEP_RUNNING) } else { ("power", "Stopped", token::FG_MUTED) };
            let mark = labelled(div().id(SharedString::from(format!("process-mark:{}", one.pid))).tooltip(tip(said)).child(glyph(look, icon, Size::Small, look.color(ink))), Role::Image, said).into_any_element();
            let pid = one.pid;
            let act = if running {
                labelled(
                    icon_button(look, SharedString::from(format!("process-stop:{pid}")), "square", format!("Stop {}", one.command).into())
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_DEFAULT))
                        .on_click(cx.listener(move |this, _, window, cx| this.stop_process(pid, window, cx))),
                    Role::Button,
                    format!("Stop {}", one.command),
                )
                .into_any_element()
            } else {
                div().into_any_element()
            };
            let command = div().id(SharedString::from(format!("process-command:{}", one.pid))).tooltip(tip(one.command.clone())).truncate().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).text_color(look.color(if running { token::FG_DEFAULT } else { token::FG_MUTED })).child(SharedString::from(one.command.clone())).into_any_element();
            labelled(
                cells(command, port.then(|| figure(one.port.map(|port| format!(":{port}")).unwrap_or_default())), figure(pid.to_string()), mark, act).id(SharedString::from(format!("process:{pid}"))),
                Role::ListItem,
                one.command.clone(),
            )
            .into_any_element()
        });
        div()
            .flex()
            .flex_col()
            .h(row_h * 9.)
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .overflow_hidden()
            .child(header)
            .child(labelled(div().id("process-rows").flex_1().min_h_0().overflow_y_scroll().flex().flex_col().children(rows), Role::List, "Processes"))
            .into_any_element()
    }

    /// A log Fleet keeps: its name and size. It opens in the file panel once Fleet serves a log's
    /// text; until then only the mock's panel reads one.
    fn log_row(&self, one: &SystemLog, look: &Look, cx: &Context<Self>) -> AnyElement {
        let opens = self.fleet.is_none();
        let path = one.path.clone();
        let (ground, ink) = (look.color(token::BG_HOVER), look.color(token::FG_DEFAULT));
        let row = div()
            .id(SharedString::from(format!("log:{}", one.path)))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .min_h(look.length(token::H_ROW))
            .px(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .text_color(look.color(token::FG_MUTED))
            .child(div().flex_1().min_w_0().truncate().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).child(SharedString::from(one.name.clone())))
            .child(div().flex_none().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(size_of(one.bytes))))
            .when(opens, |row| row.cursor_pointer().hover(move |style| style.bg(ground).text_color(ink)).on_click(cx.listener(move |this, _, window, cx| this.open_log(path.clone(), window, cx))));
        labelled(row, if opens { Role::Button } else { Role::ListItem }, format!("Open {}", one.name)).into_any_element()
    }

    /// One Job's split (`JobSplit` with `flush`): its title and total, then each bucket, most first,
    /// against its workflow's usual where the bucket is far over it, else as a share of the whole.
    fn job_split(&self, title: &str, split: &JobSplit, look: &Look) -> AnyElement {
        let spent = seconds(&split.spent);
        let usual: Option<HashMap<&str, f64>> = split.usual.as_ref().map(|usual| seconds(&usual.split).into_iter().collect());
        let total: f64 = spent.iter().map(|(_, s)| s).sum();
        let against = !split.workflow.is_empty();
        let mut order: Vec<(usize, &'static str, f64)> = spent.iter().enumerate().filter(|(_, (_, s))| *s > 0.).map(|(at, (name, s))| (at, *name, *s)).collect();
        order.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal).then(a.0.cmp(&b.0)));
        let fig = |text: String, said: String, over: bool| {
            div()
                .id(SharedString::from(format!("split-fig:{said}")))
                .flex_none()
                .w(look.length(token::SPACE_12))
                .flex()
                .justify_end()
                .font_family(mono(look))
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(if over { token::STATUS_AWAITING_REVIEW } else { token::FG_MUTED }))
                .tooltip(tip(said))
                .child(SharedString::from(text))
        };
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_2))
            .child(div().id("split-title").flex_1().min_w_0().truncate().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).tooltip(tip(title.to_string())).child(SharedString::from(title.to_string())))
            .child(fig(span(total), format!("Total time, {}", split.workflow), false));
        let heads = div()
            .flex()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_2))
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_2XS))
            .text_color(look.color(token::FG_SUBTLE))
            .child(div().flex_1())
            .child(div().w(look.length(token::SPACE_12)).flex().justify_end().child("Time"))
            .child(div().w(look.length(token::SPACE_12)).flex().justify_end().child(if against { "Usual" } else { "Share" }));
        let rows = order.into_iter().map(|(_, bucket, s)| {
            let (_, label, means) = BUCKETS.iter().find(|(id, _, _)| *id == bucket).copied().unwrap_or((bucket, bucket, ""));
            let usual_s = usual.as_ref().and_then(|usual| usual.get(bucket).copied());
            let over = against && far_over(s, usual_s);
            let share = if total > 0. { (s / total) as f32 } else { 0. };
            let hue = bucket_hue(look, bucket);
            let name = div()
                .id(SharedString::from(format!("bucket:{bucket}")))
                .flex_none()
                .w(look.length(token::SPACE_12) * 2.)
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .tooltip(tip(means))
                .child(div().flex_none().size(px(10.)).rounded(px(2.)).bg(hue).when(bucket == "idle", |swatch| swatch.border(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_DEFAULT))))
                .child(div().text_size(look.length(token::TEXT_XS)).child(label));
            let track = div()
                .flex_1()
                .h(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_SM))
                .bg(look.color(token::BG_SUNKEN))
                .relative()
                .child(div().h_full().w(gpui::relative(share)).rounded(look.length(token::RADIUS_SM)).bg(hue))
                .children(over.then(|| {
                    let at = usual_s.map_or(0., |u| if total > 0. { (u / total) as f32 } else { 0. });
                    div().absolute().top_0().bottom_0().left(gpui::relative(at)).w(px(2.)).bg(look.color(token::FG_DEFAULT))
                }));
            let second = if against {
                if over {
                    let u = usual_s.unwrap_or_default();
                    fig(span(u), format!("Usual for {}: the median Job, {}", split.workflow, span_said(u)), false).into_any_element()
                } else {
                    div().w(look.length(token::SPACE_12)).into_any_element()
                }
            } else {
                fig(format!("{}%", (share * 100.).round() as i64), "Share of all time".into(), false).into_any_element()
            };
            let said = format!("{label}, {}{}", span_said(s), if over { format!(", usual {}", span_said(usual_s.unwrap_or_default())) } else { String::new() });
            labelled(
                div()
                    .id(SharedString::from(format!("split-row:{bucket}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .min_h(look.length(token::H_ROW))
                    .px(look.length(token::SPACE_2))
                    .child(name)
                    .child(track)
                    .child(fig(span(s), format!("{label}, {}", span_said(s)), over))
                    .child(second),
                Role::ListItem,
                said,
            )
            .into_any_element()
        });
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .child(head)
            .child(heads)
            .child(labelled(div().id("split-rows").flex().flex_col().children(rows), Role::List, if against { "Job split" } else { "Split" }))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_size_reads_in_the_largest_unit_with_a_decimal_and_never_under_a_kilobyte() {
        assert_eq!(size_of(3 * 1024 * 1024 * 1024 / 2), "1.5 GB");
        assert_eq!(size_of(310 * 1024 * 1024), "310.0 MB");
        assert_eq!(size_of(40 * 1024), "40 KB");
        assert_eq!(size_of(12), "1 KB");
    }

    #[test]
    fn the_cpu_graph_tops_at_a_hundred_or_the_next_hundred_above() {
        assert_eq!(cpu_top(38.), 100.);
        assert_eq!(cpu_top(140.), 200.);
    }

    #[test]
    fn a_span_reads_as_classic_draws_it() {
        assert_eq!(span(20.), "<1m");
        assert_eq!(span(48. * 60.), "48m");
        assert_eq!(span(2. * 3600. + 5. * 60.), "2h 05m");
        assert_eq!(span(3600.), "1h");
        assert_eq!(span_said(2. * 3600. + 5. * 60.), "2 hours 5 minutes");
        assert_eq!(span_said(0.), "none");
    }

    #[test]
    fn a_bucket_is_far_over_past_twice_its_usual_and_ten_minutes_beyond() {
        assert!(far_over(1500., Some(600.)));
        assert!(!far_over(1100., Some(600.)));
        assert!(!far_over(300., Some(100.)));
        assert!(!far_over(5000., None));
    }
}
