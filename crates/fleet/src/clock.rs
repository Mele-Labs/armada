//! The one place in the workspace that reads a clock.
//!
//! **Every other crate takes its instant as an argument.** `core-model` says
//! it, `store` says it, `config` says it and the gate says it: a function that
//! reads its own inputs from the process cannot be tested and cannot be
//! replayed. That rule needs somewhere for the reading to actually happen, and
//! this is it — one trait with one method, held by Fleet, passed down as a
//! [`Timestamp`] to everything below. So [`Clock`] is not a convenience but the
//! seam that makes the rule affordable: a test plants a clock answering a fixed
//! string and the whole system below Fleet becomes deterministic, without a
//! single call site growing a `#[cfg(test)]`.
//!
//! **The format is `Timestamp`'s, and it is spelled once.** RFC3339, UTC,
//! millisecond precision — `core_model::Timestamp`'s own contract. It is
//! computed here rather than taken from a date library because the whole
//! computation is the civil-date arithmetic below, and a dependency for it
//! would be a dependency in the crate that spawns Drones.
//!
//! **There is no parsing counterpart and there will not be one here.** A stored
//! instant is compared, ordered and displayed as text; the moment something
//! wants arithmetic on one, the type that holds it needs to change rather than
//! this module growing a reader.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use core_model::Timestamp;

/// How long between two readings of the injected clock.
///
/// Zero where either will not read as an instant. **Never a negative duration**: a clock that
/// went backwards is a machine that is wrong, and reading it as an expired grace would act on it.
pub(crate) fn elapsed(from: &Timestamp, to: &Timestamp) -> Duration {
    match (from.epoch_millis(), to.epoch_millis()) {
        (Some(from), Some(to)) if to > from => Duration::from_millis((to - from) as u64),
        _ => Duration::ZERO,
    }
}

/// What time it is, asked rather than read.
///
/// **One method, and it cannot fail.** A clock that returned a `Result` would
/// put an error path into every transition Fleet writes, and there is nothing
/// a caller could do about a clock that would not answer except stop — which
/// is what the implementation below does, by falling back to the epoch rather
/// than by taking the daemon down mid-Job.
pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
}

impl<C: Clock + ?Sized> Clock for Arc<C> {
    fn now(&self) -> Timestamp {
        (**self).now()
    }
}

/// The machine's clock.
///
/// **The only type in this workspace that calls [`SystemTime::now`].** A grep
/// for it should find this line and nothing else outside a test.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl SystemClock {
    pub fn new() -> SystemClock {
        SystemClock
    }

    /// The same reading as an instant, to hold against a file's own write: the
    /// merge line's *Sent back* bound. A [`Timestamp`] has no arithmetic, by
    /// this module's rule, and a file's mtime is not one.
    pub fn instant(&self) -> SystemTime {
        SystemTime::now()
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            // A clock set before 1970 is a machine that is wrong, not a Job
            // that should stop. The epoch is visibly absurd in a log line,
            // which is the behaviour a silent panic would not have.
            .map(|since| since.as_millis() as i64)
            .unwrap_or(0);
        Timestamp::from_rfc3339(rfc3339_utc(millis))
    }
}

/// Milliseconds since the epoch, as `YYYY-MM-DDTHH:MM:SS.mmmZ`.
///
/// Public to this crate so `mint` can stamp a ULID from the same reading, and
/// so a test can drive the civil-date arithmetic at the boundaries that
/// actually go wrong — a leap day, a century that is not a leap year, the last
/// millisecond of a year.
pub(crate) fn rfc3339_utc(millis_since_epoch: i64) -> String {
    let (days, millis_of_day) = split_day(millis_since_epoch);
    let (year, month, day) = civil_from_days(days);
    let millis = millis_of_day % 1_000;
    let seconds_of_day = millis_of_day / 1_000;
    let (hour, minute, second) = (
        seconds_of_day / 3_600,
        (seconds_of_day / 60) % 60,
        seconds_of_day % 60,
    );
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

/// The instant, in milliseconds since the epoch, that this machine's day holding `millis_since_epoch`
/// began: its local midnight, as its time zone reads. Read again at the midnight found, so a day a
/// clock change falls in still begins at the midnight the machine showed.
pub(crate) async fn local_day_began(millis_since_epoch: i64) -> i64 {
    let first = day_began(millis_since_epoch, local_offset_secs(millis_since_epoch).await);
    day_began(millis_since_epoch, local_offset_secs(first).await)
}

/// One instant as this machine's clock shows it: its date, its time and its ISO week.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LocalReading {
    pub(crate) year: i64,
    pub(crate) month: i64,
    pub(crate) day: i64,
    pub(crate) hour: i64,
    pub(crate) minute: i64,
    /// Monday is 1 and Sunday 7.
    pub(crate) weekday: i64,
    /// The ISO week's own year, which differs from `year` in the first and last days of some years.
    pub(crate) week_year: i64,
    pub(crate) week: i64,
}

/// `millis_since_epoch` as this machine's time zone reads it.
pub(crate) async fn local_reading(millis_since_epoch: i64) -> LocalReading {
    reading_at(millis_since_epoch, local_offset_secs(millis_since_epoch).await)
}

/// `millis_since_epoch` on a clock `offset_secs` ahead of UTC.
pub(crate) fn reading_at(millis_since_epoch: i64, offset_secs: i64) -> LocalReading {
    let (days, millis_of_day) = split_day(millis_since_epoch + offset_secs * 1_000);
    let (year, month, day) = civil_from_days(days);
    // The epoch's day was a Thursday.
    let weekday = (days + 3).rem_euclid(7) + 1;
    // An ISO week belongs to the year its Thursday falls in.
    let thursday = days - (weekday - 1) + 3;
    let (week_year, _, _) = civil_from_days(thursday);
    let week = (thursday - days_from_civil(week_year, 1, 1)) / 7 + 1;
    let minutes = millis_of_day / 60_000;
    LocalReading { year, month, day, hour: minutes / 60, minute: minutes % 60, weekday, week_year, week }
}

/// Howard Hinnant's `days_from_civil`, the inverse of [`civil_from_days`].
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The day holding `millis_since_epoch` began at, on a clock `offset_secs` ahead of UTC.
pub(crate) fn day_began(millis_since_epoch: i64, offset_secs: i64) -> i64 {
    const DAY: i64 = 86_400_000;
    let offset = offset_secs * 1_000;
    let local = millis_since_epoch + offset;
    local - local.rem_euclid(DAY) - offset
}

/// How far the machine's time zone is ahead of UTC at that instant, in seconds, as `date` reads it:
/// asked rather than read through `libc`, which this crate may not call outside its listed sites.
/// Zero where it will not read, which is UTC's day.
pub(crate) async fn local_offset_secs(millis_since_epoch: i64) -> i64 {
    let seconds = millis_since_epoch.div_euclid(1_000).to_string();
    // macOS's `date` takes the instant as `-r`; GNU's, on a Linux build machine, as `-d @`.
    for args in [vec!["-r".to_string(), seconds.clone()], vec!["-d".to_string(), format!("@{seconds}")]] {
        let said = tokio::process::Command::new("date").args(&args).arg("+%z").stdin(std::process::Stdio::null()).output().await;
        if let Some(offset) = said.ok().filter(|said| said.status.success()).and_then(|said| zone_offset(&String::from_utf8_lossy(&said.stdout))) {
            return offset;
        }
    }
    0
}

/// `+hhmm` or `-hhmm`, in seconds.
fn zone_offset(said: &str) -> Option<i64> {
    let said = said.trim();
    let (sign, digits) = match said.split_at_checked(1)? {
        ("+", digits) => (1, digits),
        ("-", digits) => (-1, digits),
        _ => return None,
    };
    if digits.len() != 4 || !digits.bytes().all(|one| one.is_ascii_digit()) {
        return None;
    }
    let hours: i64 = digits[..2].parse().ok()?;
    let minutes: i64 = digits[2..].parse().ok()?;
    Some(sign * (hours * 3_600 + minutes * 60))
}

/// Days since the epoch, and the milliseconds inside that day.
///
/// Floor division, not truncation: a negative reading is a machine whose clock
/// is before 1970, and truncating would put it in the wrong day rather than
/// the wrong century — which is the harder mistake to notice.
fn split_day(millis_since_epoch: i64) -> (i64, i64) {
    const DAY: i64 = 86_400_000;
    let days = millis_since_epoch.div_euclid(DAY);
    (days, millis_since_epoch.rem_euclid(DAY))
}

/// Howard Hinnant's `civil_from_days`, which is the algorithm every date
/// library uses for this and is exact for the whole range an `i64` of days can
/// express. Carried rather than depended on: the whole of it is below.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    // Shift the era so that the leap-day irregularity falls at the end of a
    // 400-year cycle rather than inside one.
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    (if month <= 2 { year + 1 } else { year }, month, day)
}
