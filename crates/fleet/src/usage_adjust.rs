//! The settings the Usage page changes in place, what each would save, and the words for each change.
//! `docs/concepts/session/where-the-tokens-went.md`, *Settings that change it*; the wire is
//! `docs/practices/protocol.md`, *Settings that change the burn*.
//!
//! A setting is added by a line in [`TRACKED`], its choices and its labels here, and the estimate that
//! prices it in the store.

use config::settings::Resolved;
use ipc::{AdjustChange, AdjustChoice, AdjustSetting, SettingValue, UsageAdjustments};
use store::settings_file::SettingValue as Held;
use store::{CacheEstimate, ChangeRead, CompactionEstimate, WasteKind};

/// The window `get_usage_adjustments` reads where a caller names none.
pub(crate) const DEFAULT_WINDOW: &str = "week";

/// The point the page suggests compacting a Job at, where it is set past it.
pub(crate) const SUGGESTED_COMPACTION: u64 = 200_000;
const COMPACTION_CHOICES: [i64; 5] = [150_000, 200_000, 250_000, 300_000, 400_000];
const QUIET_CHOICES: [i64; 4] = [300, 600, 1_800, 3_300];

/// A setting the page changes, and the kind of waste it cuts.
struct Tracked {
    key: &'static str,
    label: &'static str,
    cuts: WasteKind,
}

const TRACKED: [Tracked; 3] = [
    Tracked { key: "tokens.subagentCacheTtl", label: "Subagent cache length", cuts: WasteKind::ColdCache },
    Tracked { key: "tokens.compactionWindow", label: "Compact a Job at", cuts: WasteKind::MainContext },
    Tracked { key: "timeouts.sessionQuietSeconds", label: "Stop an idle Job after", cuts: WasteKind::FreshStarts },
];

/// The kind a key cuts, for a key the page changes.
pub(crate) fn cuts_of(key: &str) -> Option<WasteKind> {
    TRACKED.iter().find(|one| one.key == key).map(|one| one.cuts)
}

/// What each tracked key holds in `resolved`, as the text the store keeps: a number as its digits, a word as itself.
fn held(resolved: &Resolved) -> Vec<(&'static str, String)> {
    TRACKED
        .iter()
        .map(|one| {
            let text = match resolved.effective(one.key) {
                Held::Integer(n) => n.to_string(),
                Held::Text(word) => word,
                _ => String::new(),
            };
            (one.key, text)
        })
        .collect()
}

/// The tracked keys whose value differs between `before` and `after`, with what each was and became.
pub(crate) fn changed(before: &Resolved, after: &Resolved) -> Vec<(&'static str, String, String)> {
    held(before)
        .into_iter()
        .zip(held(after))
        .filter(|((_, was), (_, now))| was != now)
        .map(|((key, was), (_, now))| (key, was, now))
        .collect()
}

/// A count of seconds as a person says it: `5 min`, `1 hour`.
fn spoken_seconds(seconds: i64) -> String {
    match seconds {
        s if s % 3_600 == 0 && s >= 3_600 => format!("{} hour{}", s / 3_600, if s == 3_600 { "" } else { "s" }),
        s if s % 60 == 0 => format!("{} min", s / 60),
        s => format!("{s} s"),
    }
}

/// A value of `key` as a person reads it: `1 hour`, `250k`, `55 min`. A value no label is written for reads as it is.
pub(crate) fn spoken(key: &str, text: &str) -> String {
    let number = text.parse::<i64>().ok();
    match (key, number) {
        ("tokens.subagentCacheTtl", _) => match text {
            "5m" => "5 min".to_string(),
            "1h" => "1 hour".to_string(),
            other => other.to_string(),
        },
        ("tokens.compactionWindow", Some(tokens)) => format!("{}k", tokens / 1_000),
        ("timeouts.sessionQuietSeconds", Some(seconds)) => spoken_seconds(seconds),
        _ => text.to_string(),
    }
}

fn value_of(key: &str, text: &str) -> SettingValue {
    match (key, text.parse::<i64>()) {
        ("tokens.subagentCacheTtl", _) | (_, Err(_)) => SettingValue::Text(text.to_string()),
        (_, Ok(number)) => SettingValue::Integer(number),
    }
}

/// `choices` with `now` among them, in order.
fn offering(key: &str, choices: Vec<String>, now: &str) -> Vec<AdjustChoice> {
    let mut choices = choices;
    if !choices.iter().any(|one| one == now) {
        choices.push(now.to_string());
        if key != "tokens.subagentCacheTtl" {
            choices.sort_by_key(|one| one.parse::<i64>().unwrap_or(0));
        }
    }
    choices.into_iter().map(|one| AdjustChoice { value: value_of(key, &one), label: spoken(key, &one) }).collect()
}

/// What a share of the burn is: `net` units out of `burn`, held to 0 and 1, absent where it saves nothing.
fn share(net: f64, burn: f64) -> Option<f64> {
    (net > 0.0 && burn > 0.0).then(|| (net / burn).min(1.0))
}

/// What the store priced for the window.
pub(crate) struct Priced {
    pub burn: f64,
    pub cache: Option<CacheEstimate>,
    /// Compacting at [`SUGGESTED_COMPACTION`].
    pub compaction: Option<CompactionEstimate>,
    pub changes: Vec<ChangeRead>,
}

pub(crate) fn wire(settings: &Resolved, priced: Priced) -> UsageAdjustments {
    let now = held(settings);
    let now_of = |key: &str| now.iter().find(|(one, _)| *one == key).map(|(_, text)| text.clone()).unwrap_or_default();
    let mut rows = Vec::new();
    for one in TRACKED {
        let at = now_of(one.key);
        let (choices, suggested, saves) = match one.key {
            "tokens.subagentCacheTtl" => {
                let saves = priced.cache.filter(|_| at == "5m").and_then(|cache| share(cache.saved - cache.extra, priced.burn));
                (vec!["5m".to_string(), "1h".to_string()], saves.map(|_| "1h".to_string()), saves)
            }
            "tokens.compactionWindow" => {
                let over = at.parse::<u64>().is_ok_and(|tokens| tokens > SUGGESTED_COMPACTION);
                let saves = priced
                    .compaction
                    .filter(|_| over)
                    .and_then(|one| share(one.past - f64::from(one.stretches) * one.each, priced.burn));
                (COMPACTION_CHOICES.iter().map(i64::to_string).collect(), saves.map(|_| SUGGESTED_COMPACTION.to_string()), saves)
            }
            _ => (QUIET_CHOICES.iter().map(i64::to_string).collect(), None, None),
        };
        rows.push(AdjustSetting {
            key: one.key.to_string(),
            label: one.label.to_string(),
            now: value_of(one.key, &at),
            choices: offering(one.key, choices, &at),
            suggested: suggested.map(|text| value_of(one.key, &text)),
            cuts: one.cuts.id().to_string(),
            saves_share: saves,
        });
    }
    let changes = priced
        .changes
        .into_iter()
        .map(|one| AdjustChange {
            label: TRACKED.iter().find(|tracked| tracked.key == one.key).map(|tracked| tracked.label).unwrap_or_default().to_string(),
            from: spoken(&one.key, &one.from),
            to: spoken(&one.key, &one.to),
            at: one.at,
            cuts: one.kind.id().to_string(),
            before_share: one.before_share,
            since_share: one.since_share,
            since_days: (one.since_days * 10.0).round() / 10.0,
            key: one.key,
        })
        .collect();
    UsageAdjustments { settings: rows, changes }
}
