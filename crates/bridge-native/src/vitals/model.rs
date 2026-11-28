//! What the Vitals page draws, worked out from a reading and the limits as they stand on the page: each tile's state, its readings
//! and its graph, and what the gate is doing now. Nothing here touches the screen. Once the wire lands Fleet works these out as a
//! view, and this keeps only the drawing's part.

use std::collections::BTreeMap;
use std::rc::Rc;

use ipc::graph::Row;
use ipc::SettingValue;
use serde_json::{json, Value};

use crate::graph::compile::{self, Chart};
use crate::look::{token, Token};

/// Who a change in the log is by, where it was the person.
pub const YOU: &str = "You";

/// How far back the graphs reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Span {
    Quarter,
    Hour,
    Six,
    Day,
}

impl Span {
    pub const ALL: [Span; 4] = [Span::Quarter, Span::Hour, Span::Six, Span::Day];

    pub fn label(self) -> &'static str {
        match self {
            Span::Quarter => "15m",
            Span::Hour => "1h",
            Span::Six => "6h",
            Span::Day => "24h",
        }
    }

    /// As VoiceOver reads it.
    pub fn said(self) -> &'static str {
        match self {
            Span::Quarter => "15 minutes",
            Span::Hour => "1 hour",
            Span::Six => "6 hours",
            Span::Day => "24 hours",
        }
    }

    fn reach(self) -> i64 {
        match self {
            Span::Quarter => 15 * 60_000,
            Span::Hour => 60 * 60_000,
            Span::Six => 6 * 60 * 60_000,
            Span::Day => 24 * 60 * 60_000,
        }
    }

    /// The least time between two readings drawn, so a long span draws as many points as a short one.
    fn every(self) -> i64 {
        match self {
            Span::Quarter | Span::Hour => 0,
            Span::Six => 60_000,
            Span::Day => 5 * 60_000,
        }
    }

    /// The span `vitals_span`'s slot is on.
    pub fn at(slot: u8) -> Option<Span> {
        Span::ALL.get(usize::from(slot)).copied()
    }
}

/// One reading of the machine.
#[derive(Clone, Debug, Default)]
pub struct Sample {
    /// Milliseconds since the epoch.
    pub at: i64,
    pub load1: f64,
    pub load5: f64,
    pub load15: f64,
    /// Cores busy, of the machine's.
    pub busy: f64,
    /// Memory in use and swap used, in GB.
    pub used: f64,
    pub swap: f64,
    pub compressed: f64,
    /// The macOS memory pressure level: 1 normal, 2 warning, 4 critical.
    pub pressure: f64,
    /// Checks running and waiting, and the longest wait in minutes.
    pub running: f64,
    pub waiting: f64,
    pub longest: f64,
    /// The compiler's tokens in use.
    pub tokens: f64,
    pub drones: f64,
    /// GiB free on the slots' volume.
    pub disk: f64,
    /// Fleet's own process, in percent of a core.
    pub fleet: f64,
    /// Cores busy compiling, by who started it, in `WHO`'s order.
    pub by_who: [f64; 4],
}

/// Who started a compile.
pub const WHO: [&str; 4] = ["armada check", "Fleet build", "rust-analyzer", "bare cargo"];

/// A change to a limit, by him or a Job.
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    pub id: u64,
    pub who: String,
    pub key: String,
    pub at: i64,
    pub from: SettingValue,
    pub to: SettingValue,
}

/// What the page reads: the machine over the last day, and the changes made to its limits.
#[derive(Clone, Debug)]
pub struct Reading {
    pub samples: Vec<Sample>,
    pub cores: f64,
    pub memory_gb: f64,
    pub swap_gb: f64,
    pub changes: Vec<Change>,
}

/// The tiles, in the order they stand, two across.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Load,
    Memory,
    Checks,
    Compiler,
    Drones,
    Disk,
    FleetCpu,
    CompileCpu,
    Changes,
    Unread,
}

impl Tile {
    pub const ALL: [Tile; 10] = [Tile::Load, Tile::Memory, Tile::Checks, Tile::Compiler, Tile::Drones, Tile::Disk, Tile::FleetCpu, Tile::CompileCpu, Tile::Changes, Tile::Unread];

    pub fn id(self) -> &'static str {
        match self {
            Tile::Load => "load",
            Tile::Memory => "memory",
            Tile::Checks => "checks",
            Tile::Compiler => "compiler",
            Tile::Drones => "drones",
            Tile::Disk => "disk",
            Tile::FleetCpu => "fleet-cpu",
            Tile::CompileCpu => "compile-cpu",
            Tile::Changes => "changes",
            Tile::Unread => "unread",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Tile::Load => "Load",
            Tile::Memory => "Memory",
            Tile::Checks => "Checks",
            Tile::Compiler => "Compiler",
            Tile::Drones => "Drones",
            Tile::Disk => "Disk",
            Tile::FleetCpu => "Fleet CPU",
            Tile::CompileCpu => "Compile CPU by who started it",
            Tile::Changes => "Limit changes",
            Tile::Unread => "Limits with no reading",
        }
    }

    /// A glyph from the icon registry (`packages/icons/icons/`), each a usage proposed for this page.
    pub fn glyph(self) -> &'static str {
        match self {
            Tile::Load => "cpu",
            Tile::Memory => "memory-stick",
            Tile::Checks => "test-tube-diagonal",
            Tile::Compiler => "layers",
            Tile::Drones => "drone",
            Tile::Disk => "hard-drive",
            Tile::FleetCpu => "server",
            Tile::CompileCpu => "users",
            Tile::Changes => "scroll-text",
            Tile::Unread => "ruler",
        }
    }

    /// The settings that limit the tile's reading, drawn in its foot in this order.
    pub fn keys(self) -> &'static [&'static str] {
        match self {
            Tile::Load => &["limits.loadLimit", "limits.loadWindow"],
            Tile::Memory => &["limits.memorySparePercent", "limits.memoryPressureLevel"],
            Tile::Checks => &["limits.checksAtOnce", "limits.checksWhileShort", "limits.checkStartSpacingSeconds"],
            Tile::Compiler => &["limits.compilerCoresKeptBack"],
            Tile::Drones => &["limits.dronesAtOnce"],
            Tile::Disk => &["limits.diskFloorGib", "limits.diskReclaimFloorGib"],
            Tile::FleetCpu | Tile::CompileCpu | Tile::Changes => &[],
            Tile::Unread => &["limits.queueRequeuesPerCommit", "limits.slotBuildTrimAfterDays", "limits.slotBuildCeilingGib", "timeouts.buildSweepSeconds", "timeouts.resourcePollSeconds"],
        }
    }

    /// The tile the arrows reach from this one, two across: none past an edge.
    pub fn moved(at: usize, key: &str) -> Option<usize> {
        let to = match key {
            "left" if at % 2 == 1 => at - 1,
            "right" if at % 2 == 0 => at + 1,
            "up" if at >= 2 => at - 2,
            "down" => at + 2,
            _ => return None,
        };
        (to < Tile::ALL.len()).then_some(to)
    }
}

/// How a counted setting is changed on the page: stepped, typed with its unit, or picked from the gate's three windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Stepper,
    Field,
    Windows,
}

pub fn control_of(key: &str) -> Control {
    match key {
        "limits.checksAtOnce" | "limits.checksWhileShort" | "limits.dronesAtOnce" | "limits.compilerCoresKeptBack" | "limits.memoryPressureLevel" | "limits.queueRequeuesPerCommit" => Control::Stepper,
        "limits.loadWindow" => Control::Windows,
        _ => Control::Field,
    }
}

/// The load averages the gate can read, in minutes.
pub const WINDOWS: [i64; 3] = [1, 5, 15];

/// A reading against its limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Under,
    Near,
    Over,
}

impl State {
    pub fn hue(self) -> Token {
        match self {
            State::Under => token::STATUS_COMPLETED_SUCCESS,
            State::Near => token::STATUS_AWAITING_REVIEW,
            State::Over => token::STATUS_COMPLETED_FAILED,
        }
    }

    pub fn wash(self) -> Token {
        match self {
            State::Under => token::STATUS_COMPLETED_SUCCESS_BG,
            State::Near => token::STATUS_AWAITING_REVIEW_BG,
            State::Over => token::STATUS_COMPLETED_FAILED_BG,
        }
    }

    pub fn said(self) -> &'static str {
        match self {
            State::Under => "Under its limit",
            State::Near => "Near its limit",
            State::Over => "Over its limit",
        }
    }

    /// Over past the limit, near within 15% of it, else under.
    fn of(value: f64, limit: f64) -> State {
        if value > limit {
            State::Over
        } else if value >= limit * 0.85 {
            State::Near
        } else {
            State::Under
        }
    }
}

/// A limit as the page stands on it: its value, and whether that is only typed so far.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limit {
    pub at: f64,
    pub unsaved: bool,
}

pub type Limits = BTreeMap<String, Limit>;

fn limit(limits: &Limits, key: &str) -> Limit {
    limits.get(key).copied().unwrap_or(Limit { at: 0., unsaved: false })
}

/// One key under a graph: a series in its colour slot, or a limit drawn dashed.
#[derive(Clone, Debug, PartialEq)]
pub enum Mark {
    Series { name: String, slot: usize, note: Option<&'static str> },
    Rule { name: String, unsaved: bool },
}

/// One graph of a tile, and the keys under it.
#[derive(Clone, Debug)]
pub struct Graph {
    pub id: &'static str,
    pub chart: Rc<Chart>,
    pub legend: Vec<Mark>,
    /// A limit on it is only typed: its rules are drawn in the accent.
    pub unsaved: bool,
    /// Its height in steps of the largest space.
    pub tall: f32,
    pub name: String,
}

/// What a tile draws.
#[derive(Clone, Debug)]
pub struct Face {
    pub tile: Tile,
    /// None where nothing limits the reading.
    pub state: Option<State>,
    pub big: String,
    pub sub: String,
    pub graphs: Vec<Graph>,
}

/// Why the gate holds Checks now.
#[derive(Clone, Debug, PartialEq)]
pub enum Short {
    Memory { level: i64 },
    Load { load: f64, limit: f64, window: i64 },
}

impl Short {
    pub fn said(&self) -> String {
        match self {
            Short::Memory { level } => format!("Checks held: memory pressure {level}"),
            Short::Load { load, limit, window } => format!("Checks held: load {} over {} ({window} min)", load.round(), limit.round()),
        }
    }
}

fn load_of(sample: &Sample, window: i64) -> f64 {
    match window {
        1 => sample.load1,
        5 => sample.load5,
        _ => sample.load15,
    }
}

/// What the gate reads now: short for memory first, then for load. None where it starts Checks.
pub fn short(reading: &Reading, limits: &Limits) -> Option<Short> {
    let last = reading.samples.last()?;
    let level = limit(limits, "limits.memoryPressureLevel").at;
    if level > 0. && last.pressure >= level {
        return Some(Short::Memory { level: last.pressure as i64 });
    }
    let most = limit(limits, "limits.loadLimit").at;
    let window = limit(limits, "limits.loadWindow").at as i64;
    let load = load_of(last, window);
    (most > 0. && load > most).then_some(Short::Load { load, limit: most, window })
}

/// The readings a span draws: those inside it, thinned so a long span draws about as many as an hour.
fn within(samples: &[Sample], span: Span) -> Vec<&Sample> {
    let Some(end) = samples.last().map(|last| last.at) else { return Vec::new() };
    let from = end - span.reach();
    let mut kept: Vec<&Sample> = Vec::new();
    for one in samples.iter().filter(|one| one.at >= from) {
        if kept.last().is_none_or(|last| one.at - last.at >= span.every()) {
            kept.push(one);
        }
    }
    kept
}

fn row(value: Value) -> Row {
    value.as_object().cloned().unwrap_or_default()
}

/// A figure as the tiles write it: whole where it is whole, else one place.
pub fn figure(value: f64) -> String {
    if (value - value.round()).abs() < 0.05 {
        format!("{}", value.round() as i64)
    } else {
        format!("{value:.1}")
    }
}

/// A series layer: a line, or an area where `fill`; a step where the figure counts things.
fn series_layer(fill: bool, step: bool, sort: &[&str], measure: &str) -> Value {
    let interpolate = if step { "step-after" } else { "monotone" };
    json!({
        "mark": {"type": if fill { "area" } else { "line" }, "interpolate": interpolate},
        "encoding": {
            "x": {"field": "at", "type": "temporal"},
            "y": {"field": "v", "type": "quantitative", "title": measure},
            "color": {"field": "s", "type": "nominal", "sort": sort}
        }
    })
}

fn rule_layer(at: f64) -> Value {
    json!({"mark": {"type": "rule", "strokeDash": [4, 4]}, "encoding": {"y": {"datum": at}}})
}

fn chart_of(layers: Vec<Value>, rows: &[Row]) -> Option<Rc<Chart>> {
    let spec = ipc::graph::read_spec(&json!({ "layer": layers }).to_string()).ok()?;
    compile::compile(&spec, rows, &BTreeMap::<String, Vec<Row>>::new()).ok().map(Rc::new)
}

/// The rows of each named series over the span.
fn rows_of(samples: &[&Sample], series: &[(&str, fn(&Sample) -> f64)]) -> Vec<Row> {
    samples.iter().flat_map(|one| series.iter().map(move |(name, read)| row(json!({"at": one.at, "v": read(one), "s": name})))).collect()
}

/// A graph of `series`, with a dashed rule at each limit given, and its keys.
#[allow(clippy::too_many_arguments)]
fn graph(id: &'static str, name: &str, measure: &str, samples: &[&Sample], series: &[(&str, fn(&Sample) -> f64)], fill: bool, step: bool, rules: &[(String, Limit)], tall: f32, note: Option<(usize, &'static str)>) -> Option<Graph> {
    let names: Vec<&str> = series.iter().map(|(name, _)| *name).collect();
    let mut layers = vec![series_layer(fill, step, &names, measure)];
    layers.extend(rules.iter().map(|(_, limit)| rule_layer(limit.at)));
    let chart = chart_of(layers, &rows_of(samples, series))?;
    let mut legend: Vec<Mark> = chart
        .series
        .iter()
        .enumerate()
        .map(|(at, one)| Mark::Series { name: one.name.clone(), slot: one.slot, note: note.filter(|(which, _)| *which == at).map(|(_, said)| said) })
        .collect();
    legend.extend(rules.iter().map(|(said, limit)| Mark::Rule { name: said.clone(), unsaved: limit.unsaved }));
    Some(Graph { id, chart, legend, unsaved: rules.iter().any(|(_, limit)| limit.unsaved), tall, name: name.to_string() })
}

// What each series reads off a sample.
fn load1(one: &Sample) -> f64 {
    one.load1
}
fn load5(one: &Sample) -> f64 {
    one.load5
}
fn load15(one: &Sample) -> f64 {
    one.load15
}
fn busy(one: &Sample) -> f64 {
    one.busy
}
fn used(one: &Sample) -> f64 {
    one.used
}
fn swap(one: &Sample) -> f64 {
    one.swap
}
fn pressure(one: &Sample) -> f64 {
    one.pressure
}
fn running(one: &Sample) -> f64 {
    one.running
}
fn waiting(one: &Sample) -> f64 {
    one.waiting
}
fn tokens(one: &Sample) -> f64 {
    one.tokens
}
fn drones(one: &Sample) -> f64 {
    one.drones
}
fn disk(one: &Sample) -> f64 {
    one.disk
}
fn fleet(one: &Sample) -> f64 {
    one.fleet
}
fn who0(one: &Sample) -> f64 {
    one.by_who[0]
}
fn who1(one: &Sample) -> f64 {
    one.by_who[1]
}
fn who2(one: &Sample) -> f64 {
    one.by_who[2]
}
fn who3(one: &Sample) -> f64 {
    one.by_who[3]
}

/// Every tile's face, for the span shown and the limits as the page stands on them.
pub fn faces(reading: &Reading, span: Span, limits: &Limits) -> Vec<Face> {
    let shown = within(&reading.samples, span);
    let last = reading.samples.last().cloned().unwrap_or_default();
    let cores = reading.cores;
    Tile::ALL.iter().map(|tile| face(*tile, reading, &shown, &last, cores, limits)).collect()
}

fn face(tile: Tile, reading: &Reading, shown: &[&Sample], last: &Sample, cores: f64, limits: &Limits) -> Face {
    let none = |big: String, sub: String| Face { tile, state: None, big, sub, graphs: Vec::new() };
    match tile {
        Tile::Load => {
            let most = limit(limits, "limits.loadLimit");
            let window = limit(limits, "limits.loadWindow").at as i64;
            let read = load_of(last, window);
            let mut series: Vec<(&str, fn(&Sample) -> f64)> = vec![("1 min", load1 as fn(&Sample) -> f64)];
            if window == 5 {
                series.push(("5 min", load5));
            }
            series.push(("15 min", load15));
            let read_at = series.iter().position(|(name, _)| *name == format!("{window} min"));
            let rules = if most.at > 0. { vec![(format!("Limit {}", figure(most.at)), most)] } else { Vec::new() };
            let busy = graph("busy", "Busy cores", "Cores", shown, &[("Busy cores", busy as fn(&Sample) -> f64)], true, false, &[], 2., None);
            let graphs = graph("load", "Load", "Load", shown, &series, false, false, &rules, 3.5, read_at.map(|at| (at, "Gate reads"))).into_iter().chain(busy).collect();
            Face {
                tile,
                state: (most.at > 0.).then(|| State::of(read, most.at)),
                big: format!("{read:.1}"),
                sub: format!("{window} min load, {:.1} of {} cores busy", last.busy, figure(cores)),
                graphs,
            }
        }
        Tile::Memory => {
            let spare = limit(limits, "limits.memorySparePercent");
            let level = limit(limits, "limits.memoryPressureLevel");
            let most = Limit { at: reading.memory_gb * (1. - spare.at / 100.), unsaved: spare.unsaved };
            let free = (reading.memory_gb - last.used) / reading.memory_gb * 100.;
            let short = (level.at > 0. && last.pressure >= level.at) || free < spare.at;
            let state = if short {
                State::Over
            } else if free < spare.at + 5. {
                State::Near
            } else {
                State::Under
            };
            let series: [(&str, fn(&Sample) -> f64); 2] = [("In use", used), ("Swap", swap)];
            let used = graph("memory", "Memory in use and swap", "GB", shown, &series, false, false, &[(format!("In use limit {} GB", figure(most.at)), most)], 3.5, None);
            let pressure_rules = if level.at > 0. { vec![(format!("Level {}", figure(level.at)), level)] } else { Vec::new() };
            let pressure = graph("pressure", "Memory pressure", "Level", shown, &[("Pressure", pressure as fn(&Sample) -> f64)], false, true, &pressure_rules, 2., None);
            Face {
                tile,
                state: Some(state),
                big: format!("Pressure {}", figure(last.pressure)),
                sub: format!("{} of {} GB, {} GB compressed, swap {} of {} GB", figure(last.used), figure(reading.memory_gb), figure(last.compressed), figure(last.swap), figure(reading.swap_gb)),
                graphs: used.into_iter().chain(pressure).collect(),
            }
        }
        Tile::Checks => {
            let at_once = limit(limits, "limits.checksAtOnce");
            let while_short = limit(limits, "limits.checksWhileShort");
            let series: [(&str, fn(&Sample) -> f64); 2] = [("Running", running), ("Waiting", waiting)];
            let rules = [(format!("At once {}", figure(at_once.at)), at_once), (format!("While short {}", figure(while_short.at)), while_short)];
            let state = if last.running > at_once.at {
                State::Over
            } else if last.waiting > 0. {
                State::Near
            } else {
                State::Under
            };
            let sub = if last.waiting > 0. { format!("{} waiting, longest {} min", figure(last.waiting), last.longest.round() as i64) } else { "None waiting".into() };
            Face { tile, state: Some(state), big: format!("{} running", figure(last.running)), sub, graphs: graph("checks", "Checks running and waiting", "Checks", shown, &series, false, true, &rules, 3.5, None).into_iter().collect() }
        }
        Tile::Compiler => {
            let kept = limit(limits, "limits.compilerCoresKeptBack");
            let pool = Limit { at: (cores - kept.at).max(0.), unsaved: kept.unsaved };
            let series: [(&str, fn(&Sample) -> f64); 1] = [("Tokens in use", tokens)];
            Face {
                tile,
                state: Some(State::of(last.tokens, pool.at)),
                big: format!("{} of {}", figure(last.tokens), figure(pool.at)),
                sub: format!("Pool: {} cores less {} kept back", figure(cores), figure(kept.at)),
                graphs: graph("compiler", "Compiler tokens in use", "Tokens", shown, &series, true, true, &[(format!("Pool {}", figure(pool.at)), pool)], 3.5, None).into_iter().collect(),
            }
        }
        Tile::Drones => {
            let most = limit(limits, "limits.dronesAtOnce");
            let state = if last.drones > most.at {
                State::Over
            } else if last.drones >= most.at {
                State::Near
            } else {
                State::Under
            };
            let series: [(&str, fn(&Sample) -> f64); 1] = [("Running", drones)];
            Face {
                tile,
                state: Some(state),
                big: format!("{} of {}", figure(last.drones), figure(most.at)),
                sub: "Drones running".into(),
                graphs: graph("drones", "Drones running", "Drones", shown, &series, false, true, &[(format!("At once {}", figure(most.at)), most)], 3.5, None).into_iter().collect(),
            }
        }
        Tile::Disk => {
            let floor = limit(limits, "limits.diskFloorGib");
            let reclaim = limit(limits, "limits.diskReclaimFloorGib");
            let state = if last.disk < floor.at {
                State::Over
            } else if last.disk < reclaim.at {
                State::Near
            } else {
                State::Under
            };
            let series: [(&str, fn(&Sample) -> f64); 1] = [("Free", disk)];
            let rules = [(format!("Keep free {} GiB", figure(floor.at)), floor), (format!("Reclaim under {} GiB", figure(reclaim.at)), reclaim)];
            Face { tile, state: Some(state), big: format!("{} GiB", figure(last.disk)), sub: "Free on the slots' volume".into(), graphs: graph("disk", "Disk free", "GiB", shown, &series, true, false, &rules, 3.5, None).into_iter().collect() }
        }
        Tile::FleetCpu => {
            let series: [(&str, fn(&Sample) -> f64); 1] = [("Fleet", fleet)];
            Face { tile, state: None, big: format!("{}%", last.fleet.round() as i64), sub: "Fleet's own process".into(), graphs: graph("fleet-cpu", "Fleet CPU", "Percent of a core", shown, &series, true, false, &[], 3.5, None).into_iter().collect() }
        }
        Tile::CompileCpu => {
            let series: [(&str, fn(&Sample) -> f64); 4] = [(WHO[0], who0), (WHO[1], who1), (WHO[2], who2), (WHO[3], who3)];
            let total: f64 = last.by_who.iter().sum();
            let most = last.by_who.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).map(|(at, cores)| format!("Most: {} {:.1}", WHO[at], cores)).unwrap_or_default();
            Face { tile, state: None, big: format!("{total:.1} cores"), sub: most, graphs: graph("compile-cpu", "Compile CPU by who started it", "Cores", shown, &series, true, false, &[], 3.5, None).into_iter().collect() }
        }
        Tile::Changes | Tile::Unread => none(String::new(), String::new()),
    }
}
