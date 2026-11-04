//! What a thread's context holds, a part at a time, so each turn's burn can be split by what it was
//! spent on. `docs/concepts/session/where-the-tokens-went.md`, *What the burn was spent on*.
//!
//! **An estimate, not a count**: four bytes of text are a token and an image is [`IMAGE_TOKENS`]. A
//! turn keeps only the shares the estimate gives, so the real figures decide the units and the
//! estimate only how they are shared. The text is measured as it is decoded ([`Measured`]) and
//! never kept.

use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};

/// What an image block is taken to cost, whatever its bytes.
pub const IMAGE_TOKENS: u64 = 1_600;
/// A request this long after the one before, whose cache write is larger than its cache read, found
/// the cache cold.
const COLD_AFTER_SECONDS: f64 = 300.0;

/// A part of a thread's input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Part {
    StartUp,
    Reads,
    OwnWords,
    OtherResults,
    Messages,
    Announced,
    ColdCache,
}

/// The parts a context carries from one request to the next, in the order [`Parts`] holds them.
const CARRIED: [Part; 6] = [Part::StartUp, Part::Reads, Part::OwnWords, Part::OtherResults, Part::Messages, Part::Announced];

impl Part {
    pub fn label(self) -> &'static str {
        match self {
            Part::StartUp => "start_up",
            Part::Reads => "reads",
            Part::OwnWords => "own_words",
            Part::OtherResults => "other_results",
            Part::Messages => "messages",
            Part::Announced => "announced",
            Part::ColdCache => "cold_cache",
        }
    }
}

/// Estimated tokens in each carried part.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Parts([u64; 6]);

impl Parts {
    fn add(&mut self, part: Part, tokens: u64) {
        if let Some(at) = CARRIED.iter().position(|one| *one == part) {
            self.0[at] += tokens;
        }
    }

    fn total(&self) -> u64 {
        self.0.iter().sum()
    }

    fn plus(&self, other: &Parts) -> Parts {
        Parts(std::array::from_fn(|at| self.0[at] + other.0[at]))
    }

    /// Each non-empty part's share of the whole, in [`CARRIED`] order; empty where the whole is.
    fn shares(&self) -> Vec<(Part, f64)> {
        let total = self.total();
        if total == 0 {
            return Vec::new();
        }
        CARRIED.iter().zip(self.0).filter(|(_, tokens)| *tokens > 0).map(|(part, tokens)| (*part, tokens as f64 / total as f64)).collect()
    }
}

/// How one turn's input is shared: its cache read by `read`, and its input and cache writes by
/// `write`. Each adds up to one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shares {
    pub read: Vec<(Part, f64)>,
    pub write: Vec<(Part, f64)>,
}

/// A compaction seen, waiting for the summary written after it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Compaction {
    id: String,
    at: String,
    read: u64,
}

/// A compaction's own request, which the transcript does not hold: the context it read and the
/// summary it wrote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackgroundTurn {
    pub id: String,
    pub at: String,
    pub model: String,
    pub cache_read_tokens: u64,
    pub output_tokens: u64,
}

/// What a read of a transcript carries to the next one.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Carry {
    /// The context as the last request held it.
    held: Parts,
    /// What was written after that request.
    fresh: Parts,
    /// A request has been made since the start or the last compaction.
    started: bool,
    /// The last request's message id, so a message whose lines fall in two reads is one request.
    last_id: Option<String>,
    last_at: Option<String>,
    model: Option<String>,
    /// The calls still waiting on a result, and the part each result is.
    calls: BTreeMap<String, Part>,
    compaction: Option<Compaction>,
}

impl Carry {
    /// The carry a read kept, or a new one where it kept none or the text does not decode.
    pub fn resumed(kept: Option<&str>) -> Carry {
        kept.and_then(|text| ipc::decode::<Carry>("transcript carry", text.as_bytes()).ok()).unwrap_or_default()
    }

    pub fn kept(&self) -> Option<String> {
        ipc::encode(self).ok()
    }

    pub(super) fn is_request(&self, id: &str) -> bool {
        self.last_id.as_deref() != Some(id)
    }

    /// Text written into the thread.
    pub(super) fn wrote(&mut self, part: Part, tokens: u64) {
        let part = if part == Part::Announced && !self.started { Part::StartUp } else { part };
        self.fresh.add(part, tokens);
    }

    pub(super) fn called(&mut self, id: String, result: Part) {
        self.calls.insert(id, result);
    }

    /// The part a call's result is, the call forgotten.
    pub(super) fn answered(&mut self, id: &str) -> Part {
        self.calls.remove(id).unwrap_or(Part::OtherResults)
    }

    /// A new request, `context` its input, cache writes and cache read: how its input is shared.
    pub(super) fn request(&mut self, id: &str, at: &str, model: &str, cache_write: u64, cache_read: u64, context: u64) -> Option<Shares> {
        let shares = if !self.started {
            let shown = self.fresh.total();
            self.fresh.add(Part::StartUp, context.saturating_sub(shown));
            let all = self.fresh.shares();
            Shares { read: all.clone(), write: all }
        } else {
            let cold = cache_write > cache_read && gap(self.last_at.as_deref(), at).is_some_and(|gap| gap > COLD_AFTER_SECONDS);
            let (held, fresh) = (self.held.shares(), self.fresh.shares());
            let read = if held.is_empty() { fresh.clone() } else { held.clone() };
            let write = if cold {
                vec![(Part::ColdCache, 1.0)]
            } else if fresh.is_empty() {
                held
            } else {
                fresh
            };
            Shares { read, write }
        };
        self.held = self.held.plus(&self.fresh);
        self.fresh = Parts::default();
        self.started = true;
        self.last_id = Some(id.to_string());
        self.last_at = Some(at.to_string());
        self.model = Some(model.to_string());
        (!shares.read.is_empty() || !shares.write.is_empty()).then_some(shares)
    }

    /// A compaction: the context starts again, and its request waits for the summary.
    pub(super) fn compacted(&mut self, id: String, at: String, read: u64) {
        self.held = Parts::default();
        self.fresh = Parts::default();
        self.started = false;
        self.calls.clear();
        self.compaction = Some(Compaction { id, at, read });
    }

    /// The summary after a compaction, `tokens` long: the compaction's request, where one waits.
    pub(super) fn summarised(&mut self, tokens: u64) -> Option<BackgroundTurn> {
        self.wrote(Part::StartUp, tokens);
        let compaction = self.compaction.take()?;
        Some(BackgroundTurn {
            id: format!("compaction-{}", compaction.id),
            at: compaction.at,
            model: self.model.clone().unwrap_or_default(),
            cache_read_tokens: compaction.read,
            output_tokens: tokens,
        })
    }
}

/// Seconds from `before` to `after`, both `YYYY-MM-DDTHH:MM:SS[.fff]Z` as the CLI writes them.
fn gap(before: Option<&str>, after: &str) -> Option<f64> {
    Some(seconds(after)? - seconds(before?)?)
}

fn seconds(stamp: &str) -> Option<f64> {
    let stamp = stamp.strip_suffix('Z')?;
    let (date, time) = stamp.split_once('T')?;
    let mut ymd = date.splitn(3, '-').map(|one| one.parse::<i64>().ok());
    let (y, m, d) = (ymd.next()??, ymd.next()??, ymd.next()??);
    let mut hms = time.splitn(3, ':');
    let (h, min) = (hms.next()?.parse::<f64>().ok()?, hms.next()?.parse::<f64>().ok()?);
    let s = hms.next()?.parse::<f64>().ok()?;
    // Days from the civil date, Howard Hinnant's algorithm.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days as f64 * 86_400.0 + h * 3_600.0 + min * 60.0 + s)
}

/// The part a message's text is: a reminder the CLI wrote, or what was said to the agent.
pub(super) fn said(text: &str) -> Part {
    if text.trim_start().starts_with(REMINDER) {
        Part::Announced
    } else {
        Part::Messages
    }
}

/// Tokens for `bytes` of text.
pub(super) fn tokens(bytes: u64) -> u64 {
    bytes.div_ceil(4)
}

/// Any JSON value, measured as it is decoded: the bytes of its text, an image at
/// [`IMAGE_TOKENS`], and of a map the `type` and `command` it names at the top.
#[derive(Debug, Default)]
pub(super) struct Measured {
    pub bytes: u64,
    pub kind: Option<String>,
    pub command: Option<String>,
    /// A string that is a reminder the CLI wrote into the thread.
    pub reminder: bool,
}

const REMINDER: &str = "<system-reminder>";

impl<'de> Deserialize<'de> for Measured {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(Measuring)
    }
}

struct Measuring;

impl<'de> Visitor<'de> for Measuring {
    type Value = Measured;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("any value")
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<Measured, E> {
        Ok(Measured { bytes: text.len() as u64, reminder: text.trim_start().starts_with(REMINDER), ..Measured::default() })
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<Measured, E> {
        Ok(Measured { bytes: 4, ..Measured::default() })
    }

    fn visit_i64<E: de::Error>(self, _: i64) -> Result<Measured, E> {
        Ok(Measured { bytes: 4, ..Measured::default() })
    }

    fn visit_u64<E: de::Error>(self, _: u64) -> Result<Measured, E> {
        Ok(Measured { bytes: 4, ..Measured::default() })
    }

    fn visit_f64<E: de::Error>(self, _: f64) -> Result<Measured, E> {
        Ok(Measured { bytes: 4, ..Measured::default() })
    }

    fn visit_unit<E: de::Error>(self) -> Result<Measured, E> {
        Ok(Measured::default())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Measured, A::Error> {
        let mut bytes = 0;
        while let Some(one) = seq.next_element::<Measured>()? {
            bytes += one.bytes;
        }
        Ok(Measured { bytes, ..Measured::default() })
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Measured, A::Error> {
        let mut measured = Measured::default();
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                // An image's bytes are its encoding, not what it costs.
                "source" => {
                    map.next_value::<de::IgnoredAny>()?;
                    measured.bytes += IMAGE_TOKENS * 4;
                }
                "type" | "command" => {
                    let Named(value, text) = map.next_value::<Named>()?;
                    measured.bytes += value.bytes;
                    if key == "type" {
                        measured.kind = text;
                    } else {
                        measured.command = text;
                    }
                }
                _ => measured.bytes += map.next_value::<Measured>()?.bytes,
            }
        }
        Ok(measured)
    }
}

/// A value measured, and its text where it is a string.
struct Named(Measured, Option<String>);

impl<'de> Deserialize<'de> for Named {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(Naming)
    }
}

struct Naming;

impl<'de> Visitor<'de> for Naming {
    type Value = Named;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("any value")
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<Named, E> {
        Ok(Named(Measured { bytes: text.len() as u64, ..Measured::default() }, Some(text.to_string())))
    }

    fn visit_bool<E: de::Error>(self, one: bool) -> Result<Named, E> {
        Measuring.visit_bool(one).map(|measured| Named(measured, None))
    }

    fn visit_i64<E: de::Error>(self, one: i64) -> Result<Named, E> {
        Measuring.visit_i64(one).map(|measured| Named(measured, None))
    }

    fn visit_u64<E: de::Error>(self, one: u64) -> Result<Named, E> {
        Measuring.visit_u64(one).map(|measured| Named(measured, None))
    }

    fn visit_f64<E: de::Error>(self, one: f64) -> Result<Named, E> {
        Measuring.visit_f64(one).map(|measured| Named(measured, None))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Named, E> {
        Ok(Named(Measured::default(), None))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Named, A::Error> {
        Measuring.visit_seq(seq).map(|measured| Named(measured, None))
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Named, A::Error> {
        Measuring.visit_map(map).map(|measured| Named(measured, None))
    }
}
