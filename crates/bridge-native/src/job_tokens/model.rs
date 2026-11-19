//! What a Job's tokens graph draws, worked out from Fleet's `JobTimeline` and nothing here touches the screen: the bars as fractions of
//! the window's width and of the tallest bucket, the context line, the compaction marks, the pauses as bands, where a label fits, the
//! hover cursor and what its tooltip says. Positions are fractions, so the same graph draws at any size.

use chrono::{DateTime, Duration, TimeZone, Timelike};

use super::wire::JobTimeline;
use crate::usage_page::model::{share_said, tokens_said};

/// The share of the tallest bar the bars may use: the rest is room for the label of a compaction, which never lies over a bar.
pub const HEADROOM: f32 = 0.82;

/// Seconds since the epoch of an RFC3339 stamp. None for one that cannot be read.
fn seconds_of(at: &str) -> Option<f64> {
    DateTime::parse_from_rfc3339(at).ok().map(|stamp| stamp.timestamp_millis() as f64 / 1000.)
}

/// One bucket as a column: where it stands across the window, and its two halves as shares of the tallest bucket.
#[derive(Clone, Debug, PartialEq)]
pub struct Bar {
    pub at: f64,
    pub x: f32,
    pub w: f32,
    pub main: f32,
    pub sub: f32,
    pub main_units: f64,
    pub sub_units: f64,
}

/// A pause as a band of the window.
#[derive(Clone, Debug, PartialEq)]
pub struct Band {
    pub from: f64,
    pub to: f64,
    pub x0: f32,
    pub x1: f32,
    /// Fleet's words for it, and how long it lasted.
    pub label: String,
    pub duration: String,
    /// It waited on the person: drawn in the hue that asks for him.
    pub on_person: bool,
    pub cold: bool,
    /// Where the Job woke on the context line, for the dot of a cold wake: across the window, and as a share of the peak.
    pub wake: Option<(f32, f32)>,
}

/// The tooltip of a bucket under the cursor, a line each.
#[derive(Clone, Debug, PartialEq)]
pub struct Tip {
    pub time: String,
    pub share: String,
    pub split: String,
    pub context: Option<String>,
    pub pause: Option<String>,
}

impl Tip {
    /// What a screen reader says for it.
    pub fn said(&self) -> String {
        let mut parts = vec![self.time.clone(), self.share.clone(), self.split.clone()];
        parts.extend(self.context.clone());
        parts.extend(self.pause.clone());
        parts.join(", ")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Graph {
    pub t0: f64,
    pub t1: f64,
    bucket_seconds: f64,
    pub bars: Vec<Bar>,
    /// The context line: across the window, and as a share of the peak.
    pub line: Vec<(f32, f32)>,
    context: Vec<(f64, u64)>,
    /// Where each compaction stands across the window.
    pub marks: Vec<f32>,
    pub bands: Vec<Band>,
    pub live: bool,
    total_units: f64,
    week_units: f64,
    peak_context: u64,
    cold_wakes: u32,
    subagent_share: f64,
}

impl Graph {
    pub fn new(timeline: &JobTimeline) -> Graph {
        let bucket_seconds = f64::from(timeline.bucket_seconds.max(1));
        let t0 = seconds_of(&timeline.started).unwrap_or(0.);
        let buckets: Vec<(f64, &super::wire::Bucket)> = timeline.buckets.iter().filter_map(|bucket| Some((seconds_of(&bucket.at)?, bucket))).collect();
        let context: Vec<(f64, u64)> = timeline.context.iter().filter_map(|point| Some((seconds_of(&point.at)?, point.tokens))).collect();
        let last = buckets.iter().map(|(at, _)| at + bucket_seconds).chain(context.iter().map(|(at, _)| *at)).fold(t0, f64::max);
        let t1 = timeline.ended.as_deref().and_then(seconds_of).unwrap_or(last);
        let t1 = if t1 > t0 { t1 } else { t0 + bucket_seconds * buckets.len().max(1) as f64 };
        let span = t1 - t0;
        let across = |at: f64| (((at - t0) / span) as f32).clamp(0., 1.);

        let tallest = buckets.iter().map(|(_, bucket)| bucket.main_units + bucket.subagent_units).fold(0., f64::max);
        let tallest = if tallest > 0. { tallest } else { 1. };
        let bars = buckets
            .iter()
            .map(|(at, bucket)| {
                let x = across(*at);
                Bar {
                    at: *at,
                    x,
                    w: ((bucket_seconds / span) as f32).min(1. - x),
                    main: (bucket.main_units / tallest) as f32,
                    sub: (bucket.subagent_units / tallest) as f32,
                    main_units: bucket.main_units,
                    sub_units: bucket.subagent_units,
                }
            })
            .collect();

        let peak = context.iter().map(|(_, tokens)| *tokens).chain([timeline.peak_context]).max().unwrap_or(0).max(1);
        let line = context.iter().map(|(at, tokens)| (across(*at), (*tokens as f64 / peak as f64) as f32)).collect();
        let marks = timeline.compactions.iter().filter_map(|at| seconds_of(at)).map(across).collect();
        let bands = timeline
            .pauses
            .iter()
            .filter_map(|pause| {
                let (from, to) = (seconds_of(&pause.from)?, seconds_of(&pause.to)?);
                let wake = pause.cold.then(|| {
                    let at = context_at(&context, to).map_or(0., |tokens| (tokens as f64 / peak as f64) as f32);
                    (across(to), at)
                });
                Some(Band { from, to, x0: across(from), x1: across(to), label: pause.label.clone(), duration: long_said(to - from), on_person: pause.label == "Waited on you", cold: pause.cold, wake })
            })
            .collect();
        Graph {
            t0,
            t1,
            bucket_seconds,
            bars,
            line,
            context,
            marks,
            bands,
            live: timeline.ended.is_none(),
            total_units: timeline.total_units,
            week_units: timeline.week_units,
            peak_context: timeline.peak_context,
            cold_wakes: timeline.cold_wakes,
            subagent_share: timeline.subagent_share,
        }
    }

    // ---- sizes ---------------------------------------------------------------------------------------------

    /// Each bar in pixels for a plot `width` by `height`: its left edge, its width, and the heights of its two halves. A bar is never
    /// narrower than a pixel, and leaves a pixel between it and the next where it is wide enough to spare one.
    pub fn bar_rects(&self, width: f32, height: f32) -> Vec<BarRect> {
        self.bars
            .iter()
            .map(|bar| {
                let wide = bar.w * width;
                let gap = if wide > 4. { 1. } else { 0. };
                BarRect { x: bar.x * width, w: (wide - gap).max(1.), main_h: bar.main * height * HEADROOM, sub_h: bar.sub * height * HEADROOM }
            })
            .collect()
    }

    // ---- the cursor ----------------------------------------------------------------------------------------

    /// Left and right, held at either end. The first move from nowhere lands on the first bucket, or on the last going left.
    pub fn step(&self, cursor: Option<usize>, by: isize) -> Option<usize> {
        let count = self.bars.len();
        if count == 0 {
            return None;
        }
        Some(match cursor {
            Some(at) => (at as isize + by).clamp(0, count as isize - 1) as usize,
            None if by > 0 => 0,
            None => count - 1,
        })
    }

    /// The bucket under a pointer at `across` the plot, 0 to 1.
    pub fn bucket_at(&self, across: f32) -> Option<usize> {
        let at = self.t0 + f64::from(across.clamp(0., 1.)) * (self.t1 - self.t0);
        self.bars.iter().rposition(|bar| bar.at <= at).or(if self.bars.is_empty() { None } else { Some(0) })
    }

    /// The middle of a bucket across the plot, where its rule and its tooltip stand.
    pub fn centre_of(&self, at: usize) -> Option<f32> {
        self.bars.get(at).map(|bar| (bar.x + bar.w / 2.).min(1.))
    }

    // ---- words ---------------------------------------------------------------------------------------------

    /// `09:12 to 15:40, 6h 28m, 4.1% of your week`, the times on this Mac's clock.
    pub fn header_in<Z: TimeZone>(&self, zone: &Z) -> String
    where
        Z::Offset: std::fmt::Display,
    {
        let end = if self.live { "now".to_string() } else { clock_in(self.t1, zone) };
        let mut said = format!("{} to {}, {}", clock_in(self.t0, zone), end, long_said(self.t1 - self.t0));
        if self.week_units > 0. {
            said.push_str(&format!(", {} of your week", share_fine(self.total_units / self.week_units)));
        }
        said
    }

    /// The four figures under the chart.
    pub fn figures(&self) -> [(&'static str, String); 4] {
        let hours = ((self.t1 - self.t0) / 3600.).max(1. / 60.);
        let hourly = if self.week_units > 0. { share_fine(self.total_units / self.week_units / hours) } else { "0%".into() };
        [("Burn an hour", hourly), ("Peak context", tokens_said(self.peak_context)), ("Cold wakes", self.cold_wakes.to_string()), ("In subagents", share_said(self.subagent_share))]
    }

    /// The tooltip of the bucket `at`.
    pub fn tip_in<Z: TimeZone>(&self, at: usize, zone: &Z) -> Option<Tip>
    where
        Z::Offset: std::fmt::Display,
    {
        let bar = self.bars.get(at)?;
        let week = |units: f64| if self.week_units > 0. { share_fine(units / self.week_units) } else { "0%".into() };
        let middle = bar.at + self.bucket_seconds / 2.;
        let pause = self.bands.iter().find(|band| band.from <= middle && middle < band.to).map(|band| {
            let cold = if band.cold { ". The cache went cold" } else { "" };
            format!("{}, {}{cold}", band.label, band.duration)
        });
        Some(Tip {
            time: format!("{} to {}", clock_in(bar.at, zone), clock_in(bar.at + self.bucket_seconds, zone)),
            share: format!("{} of your week", week(bar.main_units + bar.sub_units)),
            split: format!("Main thread {}, subagents {}", week(bar.main_units), week(bar.sub_units)),
            context: context_at(&self.context, middle).map(|tokens| format!("Context {}", tokens_said(tokens))),
            pause,
        })
    }

    /// The whole hours the window spans on this Mac's clock, across the plot, for the axis: every hour of a short window, every
    /// second of a longer one and every sixth of a day-long one.
    pub fn ticks_in<Z: TimeZone>(&self, zone: &Z) -> Vec<(f32, String)>
    where
        Z::Offset: std::fmt::Display,
    {
        let hours = (self.t1 - self.t0) / 3600.;
        let every = if hours <= 8. { 1 } else if hours <= 16. { 2 } else { 6 };
        let Some(start) = zone.timestamp_opt(self.t0.ceil() as i64, 0).single() else { return Vec::new() };
        let Some(mut tick) = start.with_minute(0).and_then(|at| at.with_second(0)).and_then(|at| at.with_nanosecond(0)) else { return Vec::new() };
        if tick.timestamp() < self.t0.ceil() as i64 {
            tick += Duration::hours(1);
        }
        while tick.hour() % every != 0 {
            tick += Duration::hours(1);
        }
        let mut ticks = Vec::new();
        while (tick.timestamp() as f64) <= self.t1 {
            ticks.push((((tick.timestamp() as f64 - self.t0) / (self.t1 - self.t0)) as f32, clock_in(tick.timestamp() as f64, zone)));
            tick += Duration::hours(i64::from(every));
        }
        ticks
    }
}

/// A bucket's column in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarRect {
    pub x: f32,
    pub w: f32,
    pub main_h: f32,
    pub sub_h: f32,
}

/// The context at an instant: the last point at or before it, or the first where none is.
fn context_at(points: &[(f64, u64)], at: f64) -> Option<u64> {
    points.iter().rev().find(|(when, _)| *when <= at).or(points.first()).map(|(_, tokens)| *tokens)
}

/// `09:12` on a clock.
pub fn clock_in<Z: TimeZone>(secs: f64, zone: &Z) -> String
where
    Z::Offset: std::fmt::Display,
{
    zone.timestamp_opt(secs.round() as i64, 0).single().map(|at| at.format("%H:%M").to_string()).unwrap_or_default()
}

/// A length of time: `52 min`, `6h 28m`, `2h`.
pub fn long_said(secs: f64) -> String {
    let minutes = (secs.max(0.) / 60.).round() as u64;
    match (minutes / 60, minutes % 60) {
        (0, minutes) => format!("{minutes} min"),
        (hours, 0) => format!("{hours}h"),
        (hours, minutes) => format!("{hours}h {minutes}m"),
    }
}

/// A share of the week, finer than the Usage page's because one bucket is a sliver of it: `4.1%`, `0.63%`, `0.04%`.
pub fn share_fine(share: f64) -> String {
    let percent = (share * 100.).max(0.);
    if percent == 0. {
        "0%".into()
    } else if percent < 1. {
        format!("{percent:.2}%")
    } else {
        share_said(share)
    }
}

/// The text of a label that fits `room`: the whole of it, else as much as leaves room for an ellipsis. None where not even a letter
/// and an ellipsis fit, so a band too narrow for words draws none rather than words that spill over its edge.
pub fn fit(text: &str, room: f32, measure: &dyn Fn(&str) -> f32) -> Option<String> {
    if measure(text) <= room {
        return Some(text.to_string());
    }
    let letters: Vec<char> = text.chars().collect();
    (1..letters.len()).rev().map(|keep| format!("{}…", letters[..keep].iter().collect::<String>().trim_end())).find(|cut| measure(cut) <= room)
}
