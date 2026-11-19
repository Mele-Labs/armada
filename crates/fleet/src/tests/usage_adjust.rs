//! The settings that change the burn, served: a row for each with its choices in the words a person reads and
//! the values the save takes, a suggestion only where the window's own requests price one, and a change kept
//! whether it was saved or edited by hand. `docs/concepts/session/where-the-tokens-went.md`, *Settings that change it*.

use std::collections::BTreeMap;

use api::{Sessions, Settings};
use ipc::{SaveSettings, SettingValue};
use store::{CacheEstimate, CompactionEstimate};

use super::subagent_cache_ttl::rig_with;
use crate::usage_adjust::{wire, Priced};

fn priced(cache: Option<CacheEstimate>, compaction: Option<CompactionEstimate>) -> Priced {
    Priced { burn: 2.0, cache, compaction, changes: Vec::new() }
}

fn row<'a>(read: &'a ipc::UsageAdjustments, key: &str) -> &'a ipc::AdjustSetting {
    read.settings.iter().find(|one| one.key == key).unwrap_or_else(|| panic!("{key} in {:?}", read.settings))
}

fn labels(row: &ipc::AdjustSetting) -> Vec<&str> {
    row.choices.iter().map(|one| one.label.as_str()).collect()
}

#[tokio::test]
async fn each_setting_is_offered_with_its_choices_in_words_and_nothing_is_suggested_without_a_price() {
    let (rig, _home) = rig_with("{}");
    let read = rig.fleet.get_usage_adjustments(None).await.unwrap();
    let keys: Vec<&str> = read.settings.iter().map(|one| one.key.as_str()).collect();
    assert_eq!(keys, ["tokens.subagentCacheTtl", "tokens.compactionWindow", "timeouts.sessionQuietSeconds"]);

    let ttl = row(&read, "tokens.subagentCacheTtl");
    assert_eq!((ttl.label.as_str(), ttl.cuts.as_str(), &ttl.now), ("Subagent cache length", "cold_cache", &SettingValue::Text("5m".into())));
    assert_eq!(labels(ttl), ["5 min", "1 hour"]);
    assert_eq!(ttl.choices[1].value, SettingValue::Text("1h".into()), "a choice's value is what the save takes");
    let compaction = row(&read, "tokens.compactionWindow");
    assert_eq!((compaction.cuts.as_str(), &compaction.now), ("main_context", &SettingValue::Integer(250_000)));
    assert_eq!(labels(compaction), ["150k", "200k", "250k", "300k", "400k"]);
    let quiet = row(&read, "timeouts.sessionQuietSeconds");
    assert_eq!((quiet.cuts.as_str(), &quiet.now), ("fresh_starts", &SettingValue::Integer(600)));
    assert_eq!(labels(quiet), ["5 min", "10 min", "30 min", "55 min"]);
    assert!(read.settings.iter().all(|one| one.suggested.is_none() && one.saves_share.is_none()), "{read:?}");
    assert!(read.changes.is_empty());
    assert!(rig.fleet.get_usage_adjustments(Some("month".into())).await.is_err(), "a window that is none of the three is refused");
}

#[tokio::test]
async fn a_suggestion_is_made_only_where_the_priced_change_saves_something_and_the_setting_is_not_there_already() {
    let (rig, _home) = rig_with(r#"{"tokens.compactionWindow": 275000}"#);
    let settings = rig.fleet.machine_settings().now();
    let cache = Some(CacheEstimate { saved: 0.3, extra: 0.1, rewrites: 4 });
    let cutting = Some(CompactionEstimate { past: 0.5, stretches: 2, each: 0.1 });

    let read = wire(&settings, priced(cache, cutting));
    let ttl = row(&read, "tokens.subagentCacheTtl");
    assert_eq!(ttl.suggested, Some(SettingValue::Text("1h".into())));
    assert!((ttl.saves_share.unwrap() - 0.1).abs() < 1e-9, "0.3 less 0.1, of 2.0: {:?}", ttl.saves_share);
    let compaction = row(&read, "tokens.compactionWindow");
    assert_eq!(compaction.suggested, Some(SettingValue::Integer(200_000)));
    assert!((compaction.saves_share.unwrap() - 0.15).abs() < 1e-9, "0.5 less two at 0.1, of 2.0: {:?}", compaction.saves_share);
    assert_eq!(labels(compaction), ["150k", "200k", "250k", "275k", "300k", "400k"], "what is set is among the choices, in order");
    let quiet = row(&read, "timeouts.sessionQuietSeconds");
    assert_eq!((quiet.suggested.clone(), quiet.saves_share), (None, None), "no saving is estimated for it");

    // A change that costs more than it saves is not suggested, and neither is one with no price.
    let dearer = wire(&settings, priced(Some(CacheEstimate { saved: 0.1, extra: 0.3, rewrites: 1 }), Some(CompactionEstimate { past: 0.1, stretches: 2, each: 0.1 })));
    assert!(dearer.settings.iter().all(|one| one.suggested.is_none() && one.saves_share.is_none()), "{dearer:?}");
    let unpriced = wire(&settings, priced(None, None));
    assert!(unpriced.settings.iter().all(|one| one.suggested.is_none()));

    // Already at the hour, or at or under the suggested point: nothing to suggest.
    let (rig, _home) = rig_with(r#"{"tokens.subagentCacheTtl": "1h", "tokens.compactionWindow": 200000}"#);
    let there = wire(&rig.fleet.machine_settings().now(), priced(cache, cutting));
    assert!(there.settings.iter().all(|one| one.suggested.is_none()), "{there:?}");
}

#[tokio::test]
async fn a_change_is_kept_when_it_comes_into_force_and_a_save_of_the_same_value_is_none() {
    let (rig, _home) = rig_with("{}");
    let set = |pairs: &[(&str, SettingValue)]| SaveSettings {
        changes: pairs.iter().map(|(key, value)| (key.to_string(), Some(value.clone()))).collect::<BTreeMap<_, _>>(),
    };
    rig.fleet.save_settings(set(&[("tokens.subagentCacheTtl", SettingValue::Text("5m".into()))])).await.unwrap();
    assert!(rig.fleet.get_usage_adjustments(None).await.unwrap().changes.is_empty(), "5m was already in force");

    rig.fleet.save_settings(set(&[("tokens.subagentCacheTtl", SettingValue::Text("1h".into()))])).await.unwrap();
    let read = rig.fleet.get_usage_adjustments(None).await.unwrap();
    let [change] = &read.changes[..] else { panic!("one change: {:?}", read.changes) };
    assert_eq!((change.key.as_str(), change.label.as_str(), change.from.as_str(), change.to.as_str()), ("tokens.subagentCacheTtl", "Subagent cache length", "5 min", "1 hour"));
    assert_eq!(change.cuts, "cold_cache");
    assert_eq!((change.before_share, change.since_share), (0.0, None), "nothing burned, and no day has passed");
    assert_eq!(row(&read, "tokens.subagentCacheTtl").now, SettingValue::Text("1h".into()), "the page sees what it saved");

    // Two at once, one a number and one in seconds, and a key the page does not change.
    rig.fleet
        .save_settings(set(&[
            ("tokens.compactionWindow", SettingValue::Integer(200_000)),
            ("timeouts.sessionQuietSeconds", SettingValue::Integer(3_300)),
            ("tokens.summaryThinking", SettingValue::Bool(true)),
        ]))
        .await
        .unwrap();
    let read = rig.fleet.get_usage_adjustments(None).await.unwrap();
    let mut seen: Vec<(&str, &str, &str)> = read.changes.iter().map(|one| (one.key.as_str(), one.from.as_str(), one.to.as_str())).collect();
    seen.sort();
    assert_eq!(
        seen,
        [
            ("timeouts.sessionQuietSeconds", "10 min", "55 min"),
            ("tokens.compactionWindow", "250k", "200k"),
            ("tokens.subagentCacheTtl", "5 min", "1 hour"),
        ]
    );
}

#[tokio::test]
async fn a_repeat_read_is_served_from_memory_and_a_save_of_the_cache_length_makes_the_next_adjustments_read_again() {
    let (rig, _home) = rig_with("{}");
    let memo = rig.fleet.usage_memo();
    assert_eq!(memo.made(), (0, 0));

    let first = rig.fleet.get_usage_adjustments(None).await.unwrap();
    let waste = rig.fleet.get_usage_waste(None).await.unwrap();
    assert_eq!(memo.made(), (1, 1), "each is made once");
    assert_eq!(rig.fleet.get_usage_adjustments(None).await.unwrap(), first, "the same answer");
    assert_eq!(rig.fleet.get_usage_waste(None).await.unwrap(), waste);
    assert_eq!(memo.made(), (1, 1), "a second read within five minutes makes nothing");
    rig.fleet.get_usage_adjustments(Some("day".into())).await.unwrap();
    assert_eq!(memo.made(), (1, 2), "a window of its own is made of its own");
    assert!(rig.fleet.get_usage_adjustments(Some("month".into())).await.is_err());
    assert!(rig.fleet.get_usage_adjustments(Some("month".into())).await.is_err(), "a refusal is not kept");

    rig.fleet
        .save_settings(SaveSettings { changes: BTreeMap::from([("tokens.subagentCacheTtl".to_string(), Some(SettingValue::Text("1h".into())))]) })
        .await
        .unwrap();
    let after = rig.fleet.get_usage_adjustments(None).await.unwrap();
    assert_eq!(memo.made(), (1, 3), "the save dropped what the page held");
    assert_eq!(after.changes.len(), 1, "and the page sees its change at once");
    rig.fleet.get_usage_waste(None).await.unwrap();
    assert_eq!(memo.made(), (1, 3), "the waste has no setting behind it, and is kept");

    // A save of another key leaves both.
    rig.fleet
        .save_settings(SaveSettings { changes: BTreeMap::from([("tokens.summaryThinking".to_string(), Some(SettingValue::Bool(true)))]) })
        .await
        .unwrap();
    rig.fleet.get_usage_adjustments(None).await.unwrap();
    assert_eq!(memo.made(), (1, 3));
}

#[tokio::test]
async fn an_answer_is_served_for_five_minutes_and_made_again_after() {
    let memo = crate::usage_memo::Memo::default();
    let made = std::sync::atomic::AtomicUsize::new(0);
    let waste = || async {
        made.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(ipc::UsageWaste { until: String::new(), window: ipc::WasteWindow { id: "week".into(), since: String::new(), until: String::new() }, total_units: 0.0, waste_units: 0.0, kinds: Vec::new() })
    };
    let at = |millis: i64| Some(millis);
    memo.waste("week", at(1_000), waste()).await.unwrap();
    memo.waste("week", at(1_000 + 299_999), waste()).await.unwrap();
    assert_eq!(made.load(std::sync::atomic::Ordering::SeqCst), 1);
    memo.waste("week", at(1_000 + 300_000), waste()).await.unwrap();
    assert_eq!(made.load(std::sync::atomic::Ordering::SeqCst), 2, "five minutes on it is made again");
    memo.waste("week", None, waste()).await.unwrap();
    memo.waste("week", None, waste()).await.unwrap();
    assert_eq!(made.load(std::sync::atomic::Ordering::SeqCst), 4, "a clock that says no time keeps nothing");
}
