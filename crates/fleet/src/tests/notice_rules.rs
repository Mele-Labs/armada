//! A Fleet mod's rule for a notice, obeyed where every notice is sent: quiet writes nothing, batch
//! sends what came in a while as one message, call raises the notice as an item on the Job as well
//! as telling it, the nearest place's rule wins, and the owner's mute is read before any rule.

use std::collections::BTreeMap;
use std::sync::Arc;

use api::{HostedSessions, Settings, Teaching};
use adapter_traits::{OpenPull, PullMergeable, QueueEntry, QueueState};
use ipc::{
    Condition, Does, FleetRule, Instead, Notice, ProposedFleet, RuleScope, SaveSettings, SaveTeaching, SessionId, SessionRow,
    SettingValue, TeachingProposal, TeachingReach, WaitingItem,
};
use store::{AttachmentState, Holder, KeptAttachment};

use super::session_host::{eventually, rig_placing, Rig};

async fn ruled(rig: &Rig, notice: Notice, does: Does, scope: RuleScope) {
    let rule = FleetRule { notice: notice.word().to_string(), does, scope, matches: None, when: Vec::new() };
    kept(rig, rule).await;
}

/// A batch rule everywhere for `notice` that does `instead` when `condition` holds.
async fn ruled_when(rig: &Rig, notice: Notice, does: Does, condition: Condition, instead: Does) {
    let when = vec![Instead { condition, does: instead }];
    kept(rig, FleetRule { notice: notice.word().to_string(), does, scope: RuleScope::Everywhere, matches: None, when }).await;
}

async fn kept(rig: &Rig, rule: FleetRule) {
    let proposal = TeachingProposal {
        fleet: Some(ProposedFleet { rule, untestable: Vec::new() }),
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

/// The Job holds a pull request the ledger follows, as `detail` says; and the forge's last listing
/// of the repository's open pull requests, when one is given.
async fn holding(rig: &Rig, id: &SessionId, number: u64, detail: &[(&str, &str)], listed: Option<OpenPull>) {
    let manifest = rig.fleet.manifest_id_at(&rig.root).expect("manifest");
    let row = KeptAttachment {
        holder: Holder::session(id.as_str()),
        kind: "pr".into(),
        manifest_id: manifest,
        target: number.to_string(),
        state: AttachmentState::Standing,
        detail: detail.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect(),
        since: "2026-10-23T00:00:00Z".into(),
        changed_at: "2026-10-23T00:00:00Z".into(),
    };
    assert!(rig.fleet.store().lock().await.attach(&row, false).expect("attached"));
    if let Some(pull) = listed {
        let open = crate::main_hub::OpenPulled { pull, owner: None, mine: false };
        rig.fleet.sweeping().lock().await.pulls.insert(rig.root.clone(), vec![open]);
    }
}

fn listed(number: u64, mergeable: PullMergeable) -> OpenPull {
    let mut pull = super::main_ci::pull(number, None, &[]);
    pull.facts.mergeable = Some(mergeable);
    pull
}

async fn notice_to(rig: &Rig, id: &SessionId, text: &str) {
    Arc::clone(&rig.fleet).told_news_by_fleet(id.as_str(), text, Notice::PrChecksFailed).await.expect("answered");
}

#[tokio::test]
async fn a_rule_with_matches_quiets_only_the_notices_whose_text_matches() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let rule = FleetRule {
        notice: Notice::PrChecksFailed.word().to_string(),
        does: Does::Quiet,
        scope: RuleScope::Everywhere,
        matches: Some(r"lint|fmt".to_string()),
        when: Vec::new(),
    };
    kept(&rig, rule).await;

    notice_to(&rig, &id, "#7 failed its checks: lint").await;
    assert!(notices(&rig, &id).await.is_empty(), "the notice whose text matches is quiet");
    notice_to(&rig, &id, "#7 failed its checks: unit tests").await;
    assert_eq!(notices(&rig, &id).await.len(), 1, "one that does not match is told as before");
}

#[tokio::test]
async fn the_nearest_rule_whose_match_fits_is_the_one_chosen() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let manifest = rig.fleet.manifest_id_at(&rig.root).expect("manifest");
    let word = Notice::PrChecksFailed.word().to_string();
    // The Job's rule does not fit this text, so the repository's does.
    let job = FleetRule { notice: word.clone(), does: Does::Tell, scope: RuleScope::Job(id.as_str().to_string()), matches: Some("never said".into()), when: Vec::new() };
    let repo = FleetRule { notice: word, does: Does::Quiet, scope: RuleScope::Repo(manifest), matches: None, when: Vec::new() };
    kept(&rig, job).await;
    kept(&rig, repo).await;

    notice_to(&rig, &id, "#7 failed its checks.").await;
    assert!(notices(&rig, &id).await.is_empty(), "the nearest rule whose match fits is the repository's");
}

#[tokio::test]
async fn a_batch_rule_that_tells_when_blocked_tells_at_once_for_a_blocked_pull_request_and_batches_otherwise() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    ruled_when(&rig, Notice::PrChecksFailed, Does::Batch, Condition::BlocksMerge, Does::Tell).await;

    // No pull request, then an open one that can merge: nothing blocks, so the batch holds it.
    notice_to(&rig, &id, "first").await;
    holding(&rig, &id, 7, &[("state", "open"), ("checks", "passed")], Some(listed(7, PullMergeable::Clean))).await;
    notice_to(&rig, &id, "second").await;
    assert!(notices(&rig, &id).await.is_empty(), "held in the batch both times");

    // Its required checks fail in the ledger's record.
    let blocked = Holder::session(id.as_str());
    {
        let mut store = rig.fleet.store().lock().await;
        let row = store.attachments_of(&blocked).unwrap().into_iter().find(|row| row.kind == "pr").expect("the row");
        let mut detail = row.detail.clone();
        detail.insert("checks".into(), "failed".into());
        assert!(store.follow(&row, AttachmentState::Standing, &detail, "2026-10-23T00:01:00Z").unwrap());
    }
    notice_to(&rig, &id, "third").await;
    assert_eq!(notices(&rig, &id).await, [("pr_checks_failed".to_string(), "third".to_string())], "told at once");
}

#[tokio::test]
async fn a_conflict_or_an_unmergeable_queue_entry_blocks_a_merge_and_a_draft_alone_does_not() {
    for (case, mergeable, queue, state, blocks) in [
        ("conflicting", PullMergeable::Conflicting, None, "open", true),
        ("unmergeable in the queue", PullMergeable::Clean, Some(QueueState::Unmergeable), "open", true),
        ("queued and going on", PullMergeable::Clean, Some(QueueState::Queued), "open", false),
        ("the forge still computing", PullMergeable::Unknown, None, "open", false),
        ("a draft", PullMergeable::Clean, None, "draft", false),
    ] {
        let rig = rig_placing(None);
        let id = rig.start().await;
        ruled_when(&rig, Notice::PrChecksFailed, Does::Batch, Condition::BlocksMerge, Does::Tell).await;
        holding(&rig, &id, 7, &[("state", state), ("checks", "pending")], Some(listed(7, mergeable))).await;
        if let Some(state) = queue {
            rig.fleet.sweeping().lock().await.queues.insert(rig.root.clone(), vec![QueueEntry { number: 7, position: 1, state }]);
        }
        notice_to(&rig, &id, "text").await;
        assert_eq!(!notices(&rig, &id).await.is_empty(), blocks, "{case}");
    }
}

#[tokio::test]
async fn an_open_pull_request_switches_the_action_and_a_merged_one_does_not() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    ruled_when(&rig, Notice::PrChecksFailed, Does::Quiet, Condition::OpenPullRequest, Does::Tell).await;

    notice_to(&rig, &id, "no pull request").await;
    assert!(notices(&rig, &id).await.is_empty(), "the rule's own action stands");
    holding(&rig, &id, 7, &[("state", "draft")], None).await;
    notice_to(&rig, &id, "a draft is open").await;
    assert_eq!(notices(&rig, &id).await.len(), 1, "told while a pull request is open");

    let rig = rig_placing(None);
    let id = rig.start().await;
    ruled_when(&rig, Notice::PrChecksFailed, Does::Quiet, Condition::OpenPullRequest, Does::Tell).await;
    holding(&rig, &id, 7, &[("state", "merged")], None).await;
    {
        let mut store = rig.fleet.store().lock().await;
        let row = store.attachments_of(&Holder::session(id.as_str())).unwrap().into_iter().find(|row| row.kind == "pr").expect("the row");
        store.follow(&row, AttachmentState::Spent, &row.detail, "2026-10-23T00:01:00Z").unwrap();
    }
    notice_to(&rig, &id, "after the merge").await;
    assert!(notices(&rig, &id).await.is_empty(), "a merged pull request is not open");
}

#[tokio::test]
async fn working_switches_the_action_while_the_agent_is_mid_turn() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    ruled_when(&rig, Notice::PrChecksFailed, Does::Quiet, Condition::Working, Does::Tell).await;

    notice_to(&rig, &id, "while idle").await;
    assert!(notices(&rig, &id).await.is_empty(), "idle, so the rule's own action stands");

    rig.send(&id, "do the thing").await;
    rig.stand_in.init(0);
    eventually(|| async { rig.fleet.hosts().of(id.as_str()).state().queued == 0 }).await;
    assert_eq!(rig.turn(&id).await, ipc::SessionTurn::Working { woken_by: None });
    notice_to(&rig, &id, "while working").await;
    assert_eq!(notices(&rig, &id).await.len(), 1, "told mid-turn");
}

#[tokio::test]
async fn owner_away_is_sleep_mode_and_switches_the_action() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    ruled_when(&rig, Notice::PrChecksFailed, Does::Quiet, Condition::OwnerAway, Does::Tell).await;

    notice_to(&rig, &id, "while he is here").await;
    assert!(notices(&rig, &id).await.is_empty());
    rig.fleet.store().lock().await.begin_sleep("2026-10-23T00:00:00Z").unwrap();
    notice_to(&rig, &id, "while he sleeps").await;
    assert_eq!(notices(&rig, &id).await.len(), 1);
}

#[tokio::test]
async fn the_first_condition_that_holds_decides_and_a_mute_still_beats_a_rule_with_conditions() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let when = vec![
        Instead { condition: Condition::OwnerAway, does: Does::Quiet },
        Instead { condition: Condition::OpenPullRequest, does: Does::Tell },
    ];
    let word = Notice::PrChecksFailed.word().to_string();
    kept(&rig, FleetRule { notice: word, does: Does::Batch, scope: RuleScope::Everywhere, matches: None, when }).await;
    holding(&rig, &id, 7, &[("state", "open")], None).await;
    rig.fleet.store().lock().await.begin_sleep("2026-10-23T00:00:00Z").unwrap();

    notice_to(&rig, &id, "both hold").await;
    assert!(notices(&rig, &id).await.is_empty(), "owner_away is first, so quiet");
    rig.fleet.store().lock().await.end_sleep().unwrap();
    notice_to(&rig, &id, "only the pull request holds").await;
    assert_eq!(notices(&rig, &id).await.len(), 1, "the second condition decides once the first does not hold");

    rig.fleet.store().lock().await.keep_mute(Notice::PrChecksFailed.word(), "job", id.as_str(), None).unwrap();
    let told = Arc::clone(&rig.fleet).told_news_by_fleet(id.as_str(), "muted", Notice::PrChecksFailed).await;
    assert!(!told.expect("answered"), "a mute beats the condition that says tell");
    assert_eq!(notices(&rig, &id).await.len(), 1, "nothing more was written");
}
