//! The Vitals page on fixtures: the day up to tonight's stall, 28 Nov 2026 04:00 to 04:45Z, the settings that limit it, and the
//! changes made to them. The stall is a reading every 10 s, its last three minutes the load gate Job's own samples; the day before it
//! is a reading a minute. Replaced by Fleet's reading when the route lands (`wire.rs`).

use chrono::NaiveDate;
use ipc::{Setting, SettingApplies, SettingKind, SettingValue, SettingsList};

use super::model::{Change, Reading, Sample};

/// The scenarios that open on this page.
pub const STALL: &str = "vitals-stall";
pub const LOADING: &str = "vitals-loading";

/// Whether a mock scenario of this name opens on the Vitals page.
pub fn is_vitals(scenario: &str) -> bool {
    matches!(scenario, STALL | LOADING)
}

/// This Mac: 18 cores, 36 GB of memory and 8 GB of swap.
const CORES: f64 = 18.;
const MEMORY_GB: f64 = 36.;
const SWAP_GB: f64 = 8.;

/// The load gate Job's samples, 28 Nov 04:41:29 to 04:44:51Z (`/tmp/armada-red/loadgate/samples2.tsv`): the time, the 1, 5 and
/// 15 minute loads, the busy cores and the Checks holding a slot. The 15 minute load dips under 24 at 04:42:12 and seven Checks start.
const MEASURED: [(&str, f64, f64, f64, f64, f64); 19] = [
    ("04:41:29", 17.16, 21.34, 24.30, 7.7, 1.),
    ("04:41:39", 17.11, 21.19, 24.21, 7.5, 1.),
    ("04:41:50", 16.27, 20.87, 24.06, 7.6, 1.),
    ("04:42:01", 18.30, 21.11, 24.09, 7.4, 1.),
    ("04:42:12", 17.94, 20.94, 23.99, 8.2, 7.),
    ("04:42:22", 18.65, 21.00, 23.98, 9.2, 8.),
    ("04:42:33", 21.49, 21.52, 24.13, 9.5, 8.),
    ("04:42:44", 22.87, 21.81, 24.20, 9.0, 7.),
    ("04:42:55", 26.33, 22.61, 24.46, 9.4, 7.),
    ("04:43:07", 27.83, 23.12, 24.60, 9.7, 6.),
    ("04:43:18", 28.97, 23.52, 24.73, 9.1, 6.),
    ("04:43:29", 30.59, 24.08, 24.91, 9.1, 6.),
    ("04:43:41", 31.21, 24.41, 25.02, 9.1, 6.),
    ("04:43:54", 36.59, 25.96, 25.57, 10.1, 6.),
    ("04:44:04", 38.48, 26.73, 25.84, 9.4, 5.),
    ("04:44:15", 35.74, 26.53, 25.78, 9.0, 5.),
    ("04:44:27", 36.21, 27.09, 25.99, 9.8, 5.),
    ("04:44:38", 43.05, 28.82, 26.62, 8.7, 5.),
    ("04:44:51", 42.04, 29.07, 26.73, 8.6, 5.),
];

/// Milliseconds since the epoch of a time on 28 Nov 2026, UTC.
fn tonight(h: u32, m: u32, s: u32) -> i64 {
    NaiveDate::from_ymd_opt(2026, 11, 28).and_then(|day| day.and_hms_opt(h, m, s)).map_or(0, |at| at.and_utc().timestamp_millis())
}

fn clock(said: &str) -> i64 {
    let mut parts = said.split(':').map(|part| part.parse::<u32>().unwrap_or(0));
    let (h, m, s) = (parts.next().unwrap_or(0), parts.next().unwrap_or(0), parts.next().unwrap_or(0));
    tonight(h, m, s)
}

/// A steady wobble between -1 and 1, the same each run.
fn wobble(seed: u32, at: f64) -> f64 {
    let seed = f64::from(seed);
    ((at * (0.37 + seed * 0.11)).sin() + (at * (0.13 + seed * 0.07) + seed).cos()) / 2.
}

/// The day before the stall, a reading a minute from 27 Nov 04:45 to 28 Nov 04:00Z: work through the day, quiet in the small hours,
/// then the queue filling from 03:30.
fn day() -> Vec<Sample> {
    let start = tonight(4, 0, 0) - (23 * 60 + 15) * 60_000;
    (0..23 * 60 + 15)
        .map(|minute| {
            let at = start + minute * 60_000;
            let t = minute as f64;
            let hours = t / 60.;
            // Busy from about 09:00 to 01:00 his time, a bump through the day.
            let busy_day = ((hours - 3.) / 20. * std::f64::consts::PI).sin().max(0.);
            let late = ((hours - 22.75) / 0.5).clamp(0., 1.);
            let load15 = 6. + 9. * busy_day + 18. * late + 1.2 * wobble(1, t / 9.);
            let load5 = load15 + 2.5 * wobble(2, t / 4.);
            let load1 = (load5 + 4. * wobble(3, t / 1.3)).max(1.);
            let running = (1.5 + 2.5 * busy_day + 1.5 * wobble(4, t / 17.)).round().max(0.) * (1. - late) + late;
            let waiting = if late > 0. { (9. * late).round() } else { (2. * busy_day * wobble(5, t / 23.)).round().max(0.) };
            let used = 21. + 9. * busy_day + 4. * hours / 23. + 4. * late + 0.8 * wobble(6, t / 30.);
            let swap = 0.6 + 5.6 * hours / 23.25;
            let pressure = if hours > 22.2 { 2. } else { 1. };
            let tokens = (2. + 9. * busy_day + 3. * late + 2. * wobble(7, t / 7.)).round().clamp(0., 14.);
            let drones = (1. + 2. * busy_day + wobble(8, t / 41.)).round().clamp(1., 3.);
            let disk = 142. - 64. * hours / 23.25 + 3. * wobble(9, t / 120.);
            let fleet = 8. + 10. * busy_day + 4. * wobble(10, t / 5.);
            let check = running * 1.4;
            let by_who = [check, (1.2 * busy_day * (wobble(11, t / 60.) + 1.)).max(0.), 1.2 + 1.3 * busy_day + 0.4 * wobble(12, t / 3.), (0.8 * busy_day * wobble(13, t / 25.)).max(0.)];
            Sample { at, load1, load5, load15, busy: (load1 / 2.3).min(CORES), used, swap, compressed: 8. + 14. * hours / 23.25, pressure, running, waiting, longest: if late > 0. { 30. * late } else { 0. }, tokens, drones, disk, fleet, by_who }
        })
        .collect()
}

/// The stall: a reading every 10 s from 04:00Z. One Check runs and nine wait from 04:07, held on memory pressure; Fleet's own
/// process sits near 90% from 04:06; then the measured three minutes, where seven Checks start at once.
fn stall() -> Vec<Sample> {
    let start = tonight(4, 0, 0);
    let first_measured = clock(MEASURED[0].0);
    let mut out: Vec<Sample> = (0..)
        .map(|n: i64| start + n * 10_000)
        .take_while(|at| *at < first_measured)
        .map(|at| {
            let t = ((at - start) / 10_000) as f64;
            let minutes = t / 6.;
            let load15 = 24.45 + 0.35 * wobble(21, t / 30.);
            let load5 = 21.6 + 0.9 * wobble(22, t / 9.);
            let load1 = 18.5 + 1.5 * wobble(23, t / 2.);
            let held = minutes >= 7.;
            let fleet = if minutes >= 6. { 89. + 3. * wobble(24, t / 3.) } else { 20. + 11. * minutes };
            Sample {
                at,
                load1,
                load5,
                load15,
                busy: 7.55 + 0.15 * wobble(25, t),
                used: 33.9 + 0.3 * wobble(26, t / 20.),
                swap: 6.2 + 0.7 * minutes / 41.,
                compressed: 22. + minutes / 41.,
                pressure: 2.,
                running: 1.,
                waiting: if held { 9. } else { 9. - (7. - minutes).ceil().clamp(0., 3.) },
                longest: if held { minutes - 7. } else { 0. },
                tokens: (13. + wobble(27, t / 4.)).round().clamp(0., 14.),
                drones: 3.,
                disk: 78.4 - 0.05 * minutes,
                fleet,
                by_who: [1.6, 0.3 + 0.2 * wobble(28, t / 8.), 2.5 + 0.4 * wobble(29, t / 3.), 0.8],
            }
        })
        .collect();
    for (said, load1, load5, load15, busy, holders) in MEASURED {
        let at = clock(said);
        let started = holders > 1.;
        let minutes = (at - start) as f64 / 60_000.;
        out.push(Sample {
            at,
            load1,
            load5,
            load15,
            busy,
            used: 34.2,
            swap: 6.9,
            compressed: 23.,
            pressure: 2.,
            running: holders,
            waiting: if at >= clock("04:42:12") { 2. } else { 9. },
            longest: if started { minutes - 20. } else { minutes - 7. },
            tokens: if started { 14. } else { 13. },
            drones: 3.,
            disk: 76.3,
            fleet: 90. + 2. * wobble(30, minutes),
            by_who: [holders * 1.3, 0.2, 2.4, 0.8],
        });
    }
    out
}

/// Tonight's reading: the day, then the stall, and the changes made to the limits.
pub fn reading() -> Reading {
    let mut samples = day();
    samples.extend(stall());
    Reading { samples, cores: CORES, memory_gb: MEMORY_GB, swap_gb: SWAP_GB, changes: changes() }
}

/// Three changes, newest first: the load gate Job's, then two of his.
fn changes() -> Vec<Change> {
    vec![
        Change { id: 3, who: "Load gate".into(), key: "limits.loadLimit".into(), at: tonight(3, 58, 0), from: SettingValue::Integer(0), to: SettingValue::Integer(24) },
        Change { id: 2, who: super::model::YOU.into(), key: "limits.memorySparePercent".into(), at: tonight(1, 20, 0), from: SettingValue::Integer(10), to: SettingValue::Integer(15) },
        Change { id: 1, who: super::model::YOU.into(), key: "limits.dronesAtOnce".into(), at: tonight(1, 20, 0) - 4 * 3_600_000 - 5 * 60_000, from: SettingValue::Integer(2), to: SettingValue::Integer(3) },
    ]
}

#[allow(clippy::too_many_arguments)]
fn setting(key: &str, section: &str, title: &str, description: &str, kind: SettingKind, default: i64, now: i64, applies: SettingApplies) -> Setting {
    Setting {
        key: key.into(),
        group: "Fleet".into(),
        section: section.into(),
        title: title.into(),
        description: description.into(),
        tip: None,
        kind,
        default: SettingValue::Integer(default),
        value: SettingValue::Integer(now),
        saved: (now != default).then_some(SettingValue::Integer(now)),
        applies,
        pending_restart: false,
        overridden_by_env: None,
    }
}

fn count(min: i64, max: i64, unit: Option<&str>) -> SettingKind {
    SettingKind::Integer { min, max, unit: unit.map(String::from) }
}

/// The settings the page draws, as Fleet would list them. The ones Settings › Limits already holds are its mock's rows; the load
/// gate's, the compiler's and the timeouts are written here, as the load gate Job named them.
pub fn settings() -> SettingsList {
    use SettingApplies::{AtRestart, Live};
    let held = crate::settings::mock::settings();
    let mut settings: Vec<Setting> = [
        "limits.dronesAtOnce",
        "limits.checksAtOnce",
        "limits.memorySparePercent",
        "limits.diskFloorGib",
        "limits.diskReclaimFloorGib",
        "limits.slotBuildTrimAfterDays",
        "limits.slotBuildCeilingGib",
    ]
    .iter()
    .filter_map(|key| held.settings.iter().find(|one| one.key == *key).cloned())
    .collect();
    // He set it from 10 to 15 at 01:20, which is what ships.
    if let Some(one) = settings.iter_mut().find(|one| one.key == "limits.memorySparePercent") {
        one.saved = Some(SettingValue::Integer(15));
    }
    settings.extend([
        setting("limits.loadLimit", "Limits", "Load to hold Checks at", "The load over which Fleet starts no Check, read over the window below. Zero reads no load.", count(0, 200, None), 0, 24, Live),
        setting("limits.loadWindow", "Limits", "Load window", "Which load average the gate reads: over 1, 5 or 15 minutes.", count(1, 15, Some("minutes")), 1, 1, Live),
        setting("limits.memoryPressureLevel", "Limits", "Memory pressure to hold Checks at", "The macOS memory pressure level at which Fleet starts no Check. Zero is never short for memory.", count(0, 4, None), 2, 2, Live),
        setting("limits.checksWhileShort", "Limits", "Checks while short", "How many Checks run while the machine is short of memory or over its load.", count(1, 8, Some("Checks")), 1, 1, Live),
        setting("limits.checkStartSpacingSeconds", "Limits", "Time between Check starts", "How long Fleet waits after starting one Check before it starts the next. Zero starts them together.", SettingKind::Seconds { min: 0, max: 600 }, 60, 60, Live),
        setting("limits.compilerCoresKeptBack", "Limits", "Cores kept from the compiler", "How many cores the compiler's token pool leaves for everything else. The pool is the cores less these.", count(0, 17, Some("cores")), 4, 4, Live),
        setting("limits.queueRequeuesPerCommit", "Limits", "Queue again after a runner failure", "How many times Fleet puts a pull request back in the merge queue when a job got no runner or hit its time limit. After that the merge line shows the queue as stuck.", count(0, 5, Some("times")), 2, 2, Live),
        setting("timeouts.buildSweepSeconds", "Timeouts", "Build output sweep", "How often Fleet trims every checkout's build output.", SettingKind::Seconds { min: 60, max: 86_400 }, 3_600, 3_600, AtRestart),
        setting("timeouts.resourcePollSeconds", "Timeouts", "Machine reading", "How stale a reading of the machine's memory and disk may be before it is taken again.", SettingKind::Seconds { min: 1, max: 300 }, 5, 5, Live),
    ]);
    SettingsList { path: held.path, refused: None, settings }
}
