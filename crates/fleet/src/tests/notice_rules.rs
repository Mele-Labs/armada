//! A Fleet mod's rule for a notice, obeyed where every notice is sent: quiet writes nothing, batch
//! sends what came in a while as one message, call raises the notice as an item on the Job as well
//! as telling it, the nearest place's rule wins, and the owner's mute is read before any rule.

use std::collections::BTreeMap;
use std::sync::Arc;

use api::{HostedSessions, Settings, Teaching};
use ipc::{
    Does, FleetRule, Notice, RuleScope, SaveSettings, SaveTeaching, SessionId, SessionRow, SettingValue,
    TeachingProposal, TeachingReach, WaitingItem,
};

use super::session_host::{eventually, rig_placing, Rig};

async fn ruled(rig: &Rig, notice: Notice, does: Does, scope: RuleScope) {
    let proposal = TeachingProposal {
        fleet: Some(FleetRule { notice: notice.word().to_string(), does, scope }),
        ..TeachingProposal::default()
    };
    let save = SaveTeaching {
        proposal,
        reach: TeachingReach::Everywhere,
        repository: None,
        session_id: None,
        cite: None,
        source: None,
    };
    Arc::clone(&rig.fleet).save_teaching(save).await.expect("saved");
}

/// What the thread holds of Fleet's notices: the word on each row, and its text.
async fn notices(rig: &Rig, id: &SessionId) -> Vec<(String, String)> {
    Arc::clone(&rig.fleet)
        .get_session(id.clone())
        .await
        .unwrap()
        .rows
        .into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { notice: Some(word), text, .. } => Some((word, text)),
            _ => None,
        })
        .collect()
}

async fn waiting(rig: &Rig, id: &SessionId) -> Vec<WaitingItem> {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.waiting_for
}

/// The window a batch is held for, as short as the setting allows.
async fn batching_for_a_second(rig: &Rig) {
    let changes = BTreeMap::from([("timeouts.noticeBatchSeconds".to_string(), Some(SettingValue::Integer(1)))]);
    rig.fleet.save_settings(SaveSettings { changes }).await.expect("saved");
}

#[tokio::test]
async fn a_quiet_notice_is_not_written_and_counts_as_handled() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    ruled(&rig, Notice::PrChecksFailed, Does::Quiet, RuleScope::Everywhere).await;

    let told = Arc::clone(&rig.fleet).told_by_fleet(id.as_str(), "#7 failed its checks.", Notice::PrChecksFailed).await;
    assert!(told.expect("answered"), "quiet is handled, so no caller asks again");
    assert!(notices(&rig, &id).await.is_empty(), "nothing is written to the thread");

    let other = Arc::clone(&rig.fleet).told_by_fleet(id.as_str(), "#7 conflicts with main.", Notice::PrConflicts).await;
    assert!(other.expect("answered"));
    assert_eq!(notices(&rig, &id).await.len(), 1, "a rule covers its own word and no other");
}

#[tokio::test]
async fn notices_of_a_batched_kind_go_out_as_one_message_when_the_window_ends() {
    let rig = rig_placing(None);
    batching_for_a_second(&rig).await;
    let id = rig.start().await;
    ruled(&rig, Notice::SlotFree, Does::Batch, RuleScope::Everywhere).await;

    for slot in ["slot-3", "slot-4"] {
        let told = Arc::clone(&rig.fleet).told_news_by_fleet(id.as_str(), &format!("{slot} is free to reuse."), Notice::SlotFree).await;
        assert!(told.expect("answered"), "held counts as handled");
    }
    assert!(notices(&rig, &id).await.is_empty(), "nothing goes out inside the window");

    eventually(|| async { !notices(&rig, &id).await.is_empty() }).await;
    let sent = notices(&rig, &id).await;
    assert_eq!(sent.len(), 1, "{sent:?}");
    assert_eq!(sent[0].0, "slot_free");
    assert!(sent[0].1.contains("slot-3 is free") && sent[0].1.contains("slot-4 is free"), "{sent:?}");
}

#[tokio::test]
async fn a_call_notice_is_told_and_raises_an_item_that_dismissing_settles() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    ruled(&rig, Notice::WriteClash, Does::Call, RuleScope::Everywhere).await;

    let words = "Another Job wrote src/a.rs too.";
    assert!(Arc::clone(&rig.fleet).told_by_fleet(id.as_str(), words, Notice::WriteClash).await.expect("answered"));
    assert_eq!(notices(&rig, &id).await, [("write_clash".to_string(), words.to_string())], "told as normal");
    let items = waiting(&rig, &id).await;
    let called: Vec<&WaitingItem> = items.iter().filter(|item| item.text == words).collect();
    assert_eq!(called.len(), 1, "{items:?}");
    assert_eq!(called[0].source, ipc::WaitingSource::FleetCall);

    let dismiss = ipc::DismissWaiting { session_id: id.clone(), item_id: called[0].id.clone(), reason: None, misread: false };
    Arc::clone(&rig.fleet).dismiss_waiting(dismiss).await.expect("dismissed");
    assert!(waiting(&rig, &id).await.iter().all(|item| item.text != words), "dismissing settles it");
}

#[tokio::test]
async fn a_rule_for_the_job_beats_the_repository_and_everywhere() {
    let rig = rig_placing(None);
    let (first, second) = (rig.start().await, rig.start().await);
    ruled(&rig, Notice::PrQueued, Does::Call, RuleScope::Everywhere).await;
    ruled(&rig, Notice::PrQueued, Does::Quiet, RuleScope::Job(first.as_str().to_string())).await;

    for id in [&first, &second] {
        Arc::clone(&rig.fleet).told_news_by_fleet(id.as_str(), "#9 is in the merge queue.", Notice::PrQueued).await.expect("answered");
    }
    assert!(notices(&rig, &first).await.is_empty() && waiting(&rig, &first).await.is_empty(), "the Job's own rule is quiet");
    assert_eq!(notices(&rig, &second).await.len(), 1, "another Job follows everywhere's");
    assert_eq!(waiting(&rig, &second).await.iter().filter(|item| item.text == "#9 is in the merge queue.").count(), 1);
}

#[tokio::test]
async fn a_mute_beats_a_rule_and_is_not_a_handled_notice() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    ruled(&rig, Notice::PrMerged, Does::Call, RuleScope::Everywhere).await;
    rig.fleet.store().lock().await.keep_mute(Notice::PrMerged.word(), "job", id.as_str(), None).unwrap();

    let told = Arc::clone(&rig.fleet).told_news_by_fleet(id.as_str(), "#4 merged into main.", Notice::PrMerged).await;
    assert!(!told.expect("answered"), "a muted notice is asked again, so lifting the mute resumes it");
    assert!(notices(&rig, &id).await.is_empty() && waiting(&rig, &id).await.is_empty(), "no line and no call");

    rig.fleet.store().lock().await.lift_mute(Notice::PrMerged.word(), "job", id.as_str()).unwrap();
    assert!(Arc::clone(&rig.fleet).told_news_by_fleet(id.as_str(), "#4 merged into main.", Notice::PrMerged).await.expect("answered"));
    assert_eq!(notices(&rig, &id).await.len(), 1, "the rule applies once the mute is lifted");
}
