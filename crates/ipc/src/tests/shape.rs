//! A refused body names its whole shape, read off the type that decodes it.

use crate::{decode_body, shape, SetWaitingFor, TypeInTerminal};

#[test]
fn a_body_reads_every_field_and_marks_the_ones_it_may_leave_out() {
    assert_eq!(
        shape::of::<TypeInTerminal>(),
        "{text: string, enter?: boolean, clear?: boolean, wait_seconds?: number, session_id?: string}"
    );
}

#[test]
fn a_nested_list_an_optional_object_and_an_enum_read_whole() {
    assert_eq!(
        shape::of::<SetWaitingFor>(),
        "{items: [{id: string, text: string, act?: {kind: walk|answer|approve_pr|run|message, target: string}, \
         options?: [{label: string, description?: string}], steps?: [string]}], session_id?: string}"
    );
}

#[test]
fn a_refused_body_names_its_shape_after_serdes_own_words() {
    let refused = decode_body::<SetWaitingFor>("a waiting list", br#"{"items":[{"id":"a"}]}"#).unwrap_err();
    let said = refused.to_string();
    assert!(said.starts_with("not a readable a waiting list: missing field `text`"), "{said}");
    assert!(said.contains(". The body is {items: [{id: string, text: string,"), "{said}");
    assert_eq!(crate::decode::<SetWaitingFor>("a waiting list", b"{}").unwrap_err().shape, None);
}

/// Every body an agent's tool sends, read whole: none reads as a bare `any`.
#[test]
fn every_agent_body_reads_as_an_object_of_named_fields() {
    let shapes = [
        ("all_done", shape::of::<crate::AllDone>()),
        ("ask_in_window", shape::of::<crate::AskInWindow>()),
        ("change_waiting", shape::of::<crate::ChangeWaiting>()),
        ("claim_issue", shape::of::<crate::ClaimIssue>()),
        ("claim_pull_request", shape::of::<crate::ClaimPullRequest>()),
        ("dispute_fleet", shape::of::<crate::DisputeFleet>()),
        ("finish_up", shape::of::<crate::FinishUp>()),
        ("fork_this_session", shape::of::<crate::ForkThisSession>()),
        ("mark_annotation_done", shape::of::<crate::MarkAnnotationDone>()),
        ("move_session", shape::of::<crate::MoveSession>()),
        ("record_plan", shape::of::<crate::RecordPlan>()),
        ("report_bug", shape::of::<crate::ReportBug>()),
        ("save_kit_manifest", shape::of::<crate::SaveKitManifest>()),
        ("save_request_to_collection", shape::of::<crate::SaveRequestToCollection>()),
        ("scaffold_mod", shape::of::<crate::ScaffoldMod>()),
        ("send_request", shape::of::<crate::SendRequest>()),
        ("settle_later", shape::of::<crate::SettleLater>()),
        ("show_file", shape::of::<crate::ShowFile>()),
        ("show_sketch", shape::of::<crate::ShowSketch>()),
        ("show_window", shape::of::<crate::ShowWindow>()),
        ("start_another_session", shape::of::<crate::StartAnotherSession>()),
        ("start_watch", shape::of::<crate::StartWatch>()),
        ("stop_process", shape::of::<crate::StopProcess>()),
        ("stop_watch", shape::of::<crate::StopWatch>()),
        ("take_main_red", shape::of::<crate::TakeMainRed>()),
        ("try_tracker_mod", shape::of::<crate::TryTrackerMod>()),
        ("weigh_watch", shape::of::<crate::WeighWatch>()),
        ("withdraw_draft", shape::of::<crate::WithdrawDraft>()),
        ("withdraw_window", shape::of::<crate::WithdrawWindow>()),
    ];
    for (operation, shape) in shapes {
        assert!(shape.starts_with('{') && shape.ends_with('}') && shape.contains(": "), "{operation}: {shape}");
        assert!(!shape.contains("object"), "{operation} reads deeper than a shape goes: {shape}");
    }
}

#[test]
fn a_struct_inside_itself_ends_and_one_met_again_is_named_by_its_first_field() {
    let shape = shape::of::<crate::ShowWindow>();
    assert!(shape.contains("press?: {role: string, name?: string, exact?: boolean, within?: {...}}"), "{shape}");
    assert!(shape.contains("look?: same as press"), "{shape}");
}

#[test]
fn an_internally_tagged_enum_names_its_tag_and_each_value() {
    assert_eq!(
        shape::of::<crate::ChangeWaiting>(),
        "{waiting_session: string, condition?: {kind: ends|group_done|task_done|pr_pushed|pr_lands, ...}, base?: main|branch}"
    );
}

#[test]
fn report_bug_asks_for_a_title_and_what_and_leaves_the_lines_and_the_job_out() {
    assert_eq!(
        shape::of::<crate::ReportBug>(),
        "{title: string, what: string, steps?: [string], errors?: [string], logs?: [string], session_id?: string}"
    );
}
