//! Where an axis puts its ticks and how each number and time is written: round steps of 1, 2 or 5,
//! times at whole minutes in this Mac's clock, and numbers as the dashboard writes them.

use chrono::{DateTime, Local, TimeZone, Timelike};

/// A number as it is read: whole where it is whole, else two places at most. The dashboard's rule
/// (`project_board/metric.rs`, `show_number`), so a graph and a metric block write a number alike.
pub fn show_number(value: f64) -> String {
    if value.fract() == 0. && value.abs() < 1e15 {
        return format!("{value:.0}");
    }
    let text = format!("{value:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// A step of 1, 2 or 5 times a power of ten that cuts `low..high` into about `count` parts.
fn nice_step(low: f64, high: f64, count: usize) -> f64 {
    let span = (high - low).abs();
    if span <= 0. || !span.is_finite() {
        return 1.;
    }
    let raw = span / count.max(1) as f64;
    let power = 10f64.powf(raw.log10().floor());
    let fraction = raw / power;
    let nice = if fraction <= 1. {
        1.
    } else if fraction <= 2. {
        2.
    } else if fraction <= 5. {
        5.
    } else {
        10.
    };
    nice * power
}

/// The domain widened to whole steps and the ticks inside it, for a value axis.
pub fn value_ticks(low: f64, high: f64, count: usize) -> (f64, f64, Vec<f64>) {
    let (low, high) = if high > low { (low, high) } else if low == 0. { (0., 1.) } else { (low.min(0.), low.max(0.) + low.abs().max(1.)) };
    let step = nice_step(low, high, count);
    let start = (low / step).floor() * step;
    let end = (high / step).ceil() * step;
    let ticks = (0..)
        .map(|at| start + step * at as f64)
        .take_while(|tick| *tick <= end + step * 1e-9)
        .map(|tick| if tick.abs() < step * 1e-9 { 0. } else { tick })
        .collect();
    (start, end, ticks)
}

/// The ticks of a time axis from `from` to `to`, in milliseconds: on whole minutes, hours or days
/// in this Mac's clock, about `count` of them, never past either end.
pub fn time_ticks(from: f64, to: f64, count: usize) -> Vec<f64> {
    const MINUTE: f64 = 60_000.;
    let span = (to - from).max(1.);
    let steps = [1., 2., 5., 10., 15., 30., 60., 120., 180., 360., 720., 1440.].map(|minutes| minutes * MINUTE);
    let step = steps.into_iter().find(|step| span / step <= count as f64).unwrap_or(1440. * MINUTE);
    // A step is laid on the local clock: an hour's ticks fall at :00 whatever the zone's offset.
    let offset = local(from).map_or(0., |at| at.offset().local_minus_utc() as f64 * 1000.);
    let first = ((from + offset) / step).ceil() * step - offset;
    (0..).map(|at| first + step * at as f64).take_while(|tick| *tick <= to).collect()
}

pub fn local(ms: f64) -> Option<DateTime<Local>> {
    Local.timestamp_millis_opt(ms as i64).single()
}

/// A time as an axis and a readout write it: `14:05`, or `14:05:30` where the seconds are not whole minutes.
pub fn clock(ms: f64) -> String {
    match local(ms) {
        Some(at) if at.second() != 0 => at.format("%H:%M:%S").to_string(),
        Some(at) => at.format("%H:%M").to_string(),
        None => String::new(),
    }
}

/// A number on an axis: whole steps in full under 10,000, then `12K`, `1.2M`, one style per axis.
pub fn axis_number(value: f64, largest: f64) -> String {
    let compact = |scale: f64, word: &str| format!("{}{word}", show_number(value / scale));
    match largest.abs() {
        big if big >= 1e9 => compact(1e9, "B"),
        big if big >= 1e6 => compact(1e6, "M"),
        big if big >= 1e4 => compact(1e3, "K"),
        _ => grouped(value),
    }
}

/// A number in full, with its thousands grouped: `1,284`, `2.5`.
pub fn grouped(value: f64) -> String {
    let shown = show_number(value);
    let (sign, digits) = shown.strip_prefix('-').map_or(("", shown.as_str()), |rest| ("-", rest));
    let (whole, part) = digits.split_once('.').map_or((digits, None), |(whole, part)| (whole, Some(part)));
    let mut out = String::new();
    for (at, digit) in whole.chars().enumerate() {
        if at > 0 && (whole.len() - at) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    match part {
        Some(part) => format!("{sign}{out}.{part}"),
        None => format!("{sign}{out}"),
    }
}

/// A value with its unit, as a readout writes it: `312 ms`, `2.4%`.
pub fn with_unit(value: f64, unit: Option<&str>) -> String {
    match unit {
        Some("%") => format!("{}%", grouped(value)),
        Some(unit) => format!("{} {unit}", grouped(value)),
        None => grouped(value),
    }
}

/// The unit a title names in its last brackets: `ms` for `p99 latency (ms)`.
pub fn unit_of(title: &str) -> Option<String> {
    let open = title.rfind('(')?;
    let unit = title[open + 1..].strip_suffix(')')?.trim();
    (!unit.is_empty()).then(|| unit.to_string())
}

/// A title without the unit in its last brackets: `p99 latency` for `p99 latency (ms)`.
pub fn bare_title(title: &str) -> &str {
    match unit_of(title) {
        Some(_) => title[..title.rfind('(').unwrap_or(title.len())].trim_end(),
        None => title,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_fall_on_round_steps() {
        let (low, high, ticks) = value_ticks(0., 312., 4);
        assert_eq!((low, high), (0., 400.));
        assert_eq!(ticks, vec![0., 100., 200., 300., 400.]);
        assert_eq!(grouped(12500.), "12,500");
        assert_eq!(axis_number(20000., 40000.), "20K");
        assert_eq!(unit_of("p99 latency (ms)").as_deref(), Some("ms"));
        assert_eq!(bare_title("p99 latency (ms)"), "p99 latency");
    }
}
