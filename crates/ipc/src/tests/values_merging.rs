//! Two projects' values made one: a key only one sets goes to the merged set, and a key both set differently
//! takes the gone project's only where the request names it.

use crate::{LookSet, ValuesSet, VALUE_KEYS};

fn set(auto_merge: Option<&str>, delivery: Option<&str>, workflows_off: &[&str]) -> ValuesSet {
    ValuesSet {
        auto_merge: auto_merge.map(str::to_string),
        delivery: delivery.map(str::to_string),
        workflows_off: workflows_off.iter().map(|one| one.to_string()).collect(),
        ..ValuesSet::default()
    }
}

#[test]
fn a_key_both_set_differently_is_a_conflict_and_a_key_one_leaves_out_is_not() {
    let kept = ValuesSet { review_model: Some("opus".to_string()), ..set(Some("never"), None, &["bug"]) };
    let gone = ValuesSet { look: Some(LookSet { theme: Some("dusk".into()), ..LookSet::default() }), ..set(Some("always"), Some("ask"), &["bug"]) };

    assert_eq!(kept.conflicts(&gone), ["auto_merge"], "only the key both set to different words");
    assert_eq!(gone.conflicts(&kept), ["auto_merge"]);
    assert!(kept.conflicts(&kept).is_empty());
    assert!(VALUE_KEYS.contains(&"workflows_off") && VALUE_KEYS.contains(&"look"));
}

#[test]
fn the_merged_set_copies_what_only_the_gone_project_sets_keeps_what_only_the_kept_one_sets_and_asks_about_the_rest() {
    let kept = ValuesSet { review_model: Some("opus".to_string()), ..set(Some("never"), None, &["bug"]) };
    let gone = set(Some("always"), Some("ask"), &["feature"]);

    let kept_wins = ValuesSet::merged(&kept, &gone, &[]);
    assert_eq!(kept_wins.review_model.as_deref(), Some("opus"), "only the kept project sets it");
    assert_eq!(kept_wins.delivery.as_deref(), Some("ask"), "only the gone project sets it");
    assert_eq!(kept_wins.auto_merge.as_deref(), Some("never"), "both set it, and nothing named it");
    assert_eq!(kept_wins.workflows_off, ["bug"]);

    let gone_wins = ValuesSet::merged(&kept, &gone, &["auto_merge".to_string(), "workflows_off".to_string()]);
    assert_eq!(gone_wins.auto_merge.as_deref(), Some("always"));
    assert_eq!(gone_wins.workflows_off, ["feature"]);
    assert_eq!(gone_wins.review_model.as_deref(), Some("opus"));
    assert_eq!(ValuesSet::merged(&gone_wins, &gone, &["auto_merge".to_string()]), gone_wins, "merging again changes nothing");
}
