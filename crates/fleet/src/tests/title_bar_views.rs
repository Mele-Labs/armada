//! What Fleet works out for the title bar and the clash marks, so the native Bridge and Pocket draw it
//! and do not: a ring's band, a Job's context percent, whether Compact is offered, what the build
//! switcher draws, and the clashes of every repository. Each is one pure function; the ledger case
//! at the end holds the record carrying them.

use std::sync::Arc;

use api::HostedSessions;
use ipc::{
    AllWriteClashes, BuildPosition, BuildSource, ClashHolder, ClashSide, CompactHow, CompactOff, CompactOffer, Instant,
    ManifestId, PlanWindow, RestartAct, RingBand, SessionRecord, SessionState, TerminalCommand, TerminalFacts, WriteClash,
};

use super::session_host::rig_placing;
use crate::building::switcher_of;
use crate::clashing::{across, moved};
use crate::session_host::banded;
use crate::sessioning::compact::compact_of;
use crate::sessioning::context_reading;

#[test]
fn a_ring_is_normal_below_eighty_warning_from_eighty_and_danger_from_ninety_five() {
    let bands: Vec<RingBand> = [0, 79, 80, 94, 95, 100, 140].into_iter().map(RingBand::of).collect();
    assert_eq!(
        bands,
        [
            RingBand::Normal,
            RingBand::Normal,
            RingBand::Warning,
            RingBand::Warning,
            RingBand::Danger,
            RingBand::Danger,
            RingBand::Danger,
        ]
    );
}

#[test]
fn a_context_is_a_whole_percent_rounded_half_up_and_clamped_with_its_band() {
    let read = |tokens, window| context_reading(Some(tokens), Some(window));
    assert_eq!(read(40_000, 200_000), Some((20, RingBand::Normal)));
    assert_eq!(read(1, 200), Some((1, RingBand::Normal)), "half a percent rounds up, as the ring's label does");
    assert_eq!(read(159_999, 200_000), Some((80, RingBand::Warning)));
    assert_eq!(read(190_000, 200_000), Some((95, RingBand::Danger)));
    assert_eq!(read(250_000, 200_000), Some((100, RingBand::Danger)), "past the window is the whole ring");
    assert_eq!(read(0, 200_000), Some((0, RingBand::Normal)));
}

#[test]
fn a_context_with_no_window_to_fill_reads_nothing() {
    assert_eq!(context_reading(None, Some(200_000)), None);
    assert_eq!(context_reading(Some(40_000), None), None);
    assert_eq!(context_reading(Some(40_000), Some(0)), None, "a window of nothing draws no ring");
}

#[test]
fn a_plan_window_takes_the_band_of_its_percent_whatever_a_report_named() {
    let window = |percent_used, band| PlanWindow { kind: "five_hour".into(), percent_used, resets_at: None, band };
    let read = banded(vec![window(96, RingBand::Normal), window(81, RingBand::Danger), window(12, RingBand::Warning)]);
    let bands: Vec<RingBand> = read.iter().map(|one| one.band).collect();
    assert_eq!(bands, [RingBand::Danger, RingBand::Warning, RingBand::Normal]);
    assert_eq!(read[0].percent_used, 96, "the figure is as reported");
}

/// A hosted Job's record, which a case then turns into the kind it needs.
async fn a_record() -> SessionRecord {
    let rig = rig_placing(None);
    let id = rig.start().await;
    Arc::clone(&rig.fleet).get_session(id).await.expect("a session").session
}

fn terminal(listening: bool, commands: &[&str]) -> TerminalFacts {
    TerminalFacts {
        listening,
        commands: commands.iter().map(|name| TerminalCommand { name: (*name).into(), says: String::new() }).collect(),
        ..TerminalFacts::default()
    }
}

#[tokio::test]
async fn compact_is_offered_to_a_hosted_job_as_a_message_once_its_agent_lists_it() {
    let mut record = a_record().await;
    record.hosted.as_mut().expect("hosted").commands = vec!["help".into(), "compact".into()];
    assert_eq!(compact_of(&record), CompactOffer::Offered { how: CompactHow::Message });
    record.hosted.as_mut().expect("hosted").commands = vec!["help".into()];
    assert_eq!(compact_of(&record), CompactOffer::Off { why: CompactOff::AgentNotListed });
    record.hosted.as_mut().expect("hosted").commands = Vec::new();
    assert_eq!(compact_of(&record), CompactOffer::Off { why: CompactOff::AgentNotListed });
}

#[tokio::test]
async fn compact_is_offered_to_a_terminal_job_as_a_command_while_its_mod_listens_and_lists_it() {
    let mut record = a_record().await;
    record.hosted = None;
    record.terminal = Some(terminal(true, &["compact"]));
    assert_eq!(compact_of(&record), CompactOffer::Offered { how: CompactHow::Terminal });
    record.terminal = Some(terminal(true, &["clear"]));
    assert_eq!(compact_of(&record), CompactOffer::Off { why: CompactOff::TerminalNotListed });
    record.terminal = Some(terminal(false, &["compact"]));
    assert_eq!(compact_of(&record), CompactOffer::Off { why: CompactOff::NotListening }, "a mod not listening is quiet first");
    record.terminal = None;
    assert_eq!(compact_of(&record), CompactOffer::Off { why: CompactOff::NotListening });
}

#[tokio::test]
async fn an_ended_job_is_off_because_it_ended_before_anything_else_is_asked() {
    let mut record = a_record().await;
    record.hosted.as_mut().expect("hosted").commands = vec!["compact".into()];
    record.state = SessionState::Ended;
    assert_eq!(compact_of(&record), CompactOffer::Off { why: CompactOff::Ended });
    record.hosted = None;
    record.terminal = Some(terminal(true, &["compact"]));
    assert_eq!(compact_of(&record), CompactOffer::Off { why: CompactOff::Ended });
}

#[tokio::test]
async fn a_job_in_the_ledger_carries_what_it_offers() {
    let record = a_record().await;
    assert_eq!(record.compact, Some(CompactOffer::Off { why: CompactOff::AgentNotListed }), "an agent that has listed nothing");
}

fn at(ahead: u32, behind: u32) -> Option<BuildPosition> {
    Some(BuildPosition { ahead, behind })
}

#[test]
fn the_restart_is_idle_only_on_an_aligned_main_with_nothing_restarting() {
    let idle = switcher_of(BuildSource::Main, at(0, 0), None);
    assert!(idle.aligned && idle.idle_on_main);
    assert_eq!(idle.restart, RestartAct::UpdateToMain);

    let behind = switcher_of(BuildSource::Main, at(0, 3), None);
    assert!(!behind.aligned && !behind.idle_on_main);
    let ahead = switcher_of(BuildSource::Main, at(2, 0), None);
    assert!(!ahead.aligned && !ahead.idle_on_main);

    let unknown = switcher_of(BuildSource::Main, None, None);
    assert!(!unknown.aligned && !unknown.idle_on_main, "a position that cannot be counted is not aligned");

    let restarting = switcher_of(BuildSource::Main, at(0, 0), Some(BuildSource::Main));
    assert!(restarting.aligned && !restarting.idle_on_main, "a restart under way is not idle");
}

#[test]
fn the_preview_is_never_idle_and_the_act_follows_the_build_being_restarted_onto() {
    let preview = switcher_of(BuildSource::Preview, at(0, 0), None);
    assert!(preview.aligned && !preview.idle_on_main);
    assert_eq!(preview.restart, RestartAct::RefreshPreview);

    let to_preview = switcher_of(BuildSource::Main, at(0, 3), Some(BuildSource::Preview));
    assert_eq!(to_preview.restart, RestartAct::RefreshPreview, "moving onto the preview, from main");
    let to_main = switcher_of(BuildSource::Preview, at(0, 3), Some(BuildSource::Main));
    assert_eq!(to_main.restart, RestartAct::UpdateToMain, "moving onto main, from the preview");
}

fn clash(job: &str) -> WriteClash {
    let holder = |kind, id: &str| ClashHolder { kind, id: id.into(), label: id.into() };
    WriteClash {
        one: holder(ClashSide::Session, job),
        other: holder(ClashSide::Main, "main"),
        paths: vec!["a.rs".into()],
        since: Instant::carried("2026-11-09T10:00:00.000Z"),
    }
}

#[test]
fn clashes_across_repositories_leave_out_one_with_none_and_keep_the_order() {
    let all = across(vec![
        (ManifestId::carried("01A"), vec![clash("s1")]),
        (ManifestId::carried("01B"), Vec::new()),
        (ManifestId::carried("01C"), vec![clash("s2"), clash("s3")]),
    ]);
    let seen: Vec<(&str, usize)> = all.repositories.iter().map(|one| (one.manifest_id.as_str(), one.clashes.len())).collect();
    assert_eq!(seen, [("01A", 1), ("01C", 2)]);
    assert_eq!(across(vec![(ManifestId::carried("01A"), Vec::new())]), AllWriteClashes::default());
}

#[test]
fn the_set_of_clashes_is_pushed_only_when_it_moves_and_the_first_read_is_the_baseline() {
    let one = across(vec![(ManifestId::carried("01A"), vec![clash("s1")])]);
    let two = across(vec![(ManifestId::carried("01A"), vec![clash("s1"), clash("s2")])]);
    let mut last = None;
    assert_eq!(moved(&mut last, one.clone()), None, "the first read sets the baseline");
    assert_eq!(moved(&mut last, one.clone()), None, "the same set says nothing");
    assert_eq!(moved(&mut last, two.clone()), Some(two.clone()), "a clash found");
    assert_eq!(moved(&mut last, two), None);
    assert_eq!(moved(&mut last, AllWriteClashes::default()), Some(AllWriteClashes::default()), "the last one cleared");
}
