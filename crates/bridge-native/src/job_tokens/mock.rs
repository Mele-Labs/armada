//! A Job's timeline on mock data: six hours from 09:12 to 15:40 on this Mac's clock, a subagent in bursts, two compactions, a 14 min pause
//! on a command and a 52 min pause on the person that found the cache cold.

use chrono::{DateTime, Duration, Local, TimeZone};
use ipc::{JobTimeline, TimelineBucket, TimelineContext, TimelinePause};

const BUCKET: i64 = 300;

fn at(hour: u32, minute: u32) -> DateTime<Local> {
    Local.with_ymd_and_hms(2026, 11, 18, hour, minute, 0).single().expect("a clock time that exists")
}

fn stamp(when: DateTime<Local>) -> String {
    when.to_rfc3339()
}

/// A steady stand-in for chance: 0 to 1, the same for the same step.
fn wobble(step: i64) -> f64 {
    let mixed = (step as u64).wrapping_mul(2_654_435_761) >> 12;
    (mixed % 100) as f64 / 100.
}

/// Six hours, 28 minutes.
pub fn timeline() -> JobTimeline {
    let (start, end) = (at(9, 12), at(15, 40));
    let pauses = [(at(10, 41), at(10, 55), "command", "Command ran", false), (at(12, 20), at(13, 12), "you", "Waited on you", true)];
    let compactions = [at(11, 30), at(14, 10)];
    let paused = |when: DateTime<Local>| pauses.iter().any(|(from, to, ..)| *from <= when && when < *to);

    let mut buckets = Vec::new();
    let mut context = Vec::new();
    let (mut total, mut sub_total) = (0., 0.);
    let (mut held, mut step) = (28_000_u64, 0);
    let mut when = start;
    while when < end {
        let quiet = paused(when);
        let burst = (11 * 3600 + 5 * 60..11 * 3600 + 25 * 60).contains(&seconds_of_day(when)) || (14 * 3600 + 20 * 60..14 * 3600 + 55 * 60).contains(&seconds_of_day(when)) || (9 * 3600 + 50 * 60..10 * 3600).contains(&seconds_of_day(when));
        let size = 0.35 + held as f64 / 200_000. * 0.9;
        let main = if quiet { 0. } else { 30_000. * size * (0.6 + wobble(step)) };
        let sub = if quiet || !burst { 0. } else { 55_000. * (0.5 + wobble(step + 91)) };
        total += main + sub;
        sub_total += sub;
        buckets.push(TimelineBucket { at: stamp(when), main_units: main, subagent_units: sub });
        if !quiet {
            if compactions.iter().any(|mark| *mark <= when && when < *mark + Duration::seconds(BUCKET)) {
                held = 24_000;
            } else {
                held += 3_000 + (wobble(step + 17) * 4_000.) as u64;
            }
            context.push(TimelineContext { at: stamp(when), tokens: held });
        }
        when += Duration::seconds(BUCKET);
        step += 1;
    }
    let peak = context.iter().map(|point| point.tokens).max().unwrap_or(0);
    JobTimeline {
        session_id: "mock-02".into(),
        started: stamp(start),
        ended: Some(stamp(end)),
        bucket_seconds: BUCKET as u32,
        total_units: total,
        week_units: total / 0.041,
        buckets,
        context,
        compactions: compactions.iter().map(|when| stamp(*when)).collect(),
        pauses: pauses
            .iter()
            .map(|(from, to, waited_on, label, cold)| TimelinePause { from: stamp(*from), to: stamp(*to), waited_on: (*waited_on).into(), label: (*label).into(), cold: *cold })
            .collect(),
        peak_context: peak,
        cold_wakes: 1,
        subagent_share: sub_total / total,
    }
}

fn seconds_of_day(when: DateTime<Local>) -> i64 {
    use chrono::Timelike;
    i64::from(when.num_seconds_from_midnight())
}

/// A Job that spent nothing: Fleet answers one bucket of zero.
#[cfg(test)]
pub fn empty() -> JobTimeline {
    let start = at(9, 12);
    JobTimeline {
        session_id: "mock-02".into(),
        started: stamp(start),
        ended: None,
        bucket_seconds: 60,
        total_units: 0.,
        week_units: 0.,
        buckets: vec![TimelineBucket { at: stamp(start), main_units: 0., subagent_units: 0. }],
        context: Vec::new(),
        compactions: Vec::new(),
        pauses: Vec::new(),
        peak_context: 0,
        cold_wakes: 0,
        subagent_share: 0.,
    }
}
