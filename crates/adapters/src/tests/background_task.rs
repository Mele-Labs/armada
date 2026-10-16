//! A background task, read off the lines the harness writes for it.
//!
//! **The lines are a recording**, byte for byte but for the session and run ids: one headless
//! session on claude 2.1.300 was asked for a background command that exits 0, one that exits 3 and
//! a `Monitor`, on 16 Oct 2026. The stop (`killed`/`stopped`) is the line `docs/spikes/004-*`
//! holds, from a command the session stopped itself.

use adapter_traits::{DroneEvent, TaskStatus};

use crate::background_form;
use crate::transcript::read;

const OK_STARTED: &str = r#"{"type":"system","subtype":"task_started","task_id":"bsr32fcbn","run_id":"0mvb9acly-75a717e1","tool_use_id":"toolu_015gM3DJAc7ui6cBMn6qAUYb","description":"ok job","is_backgrounded":true,"task_type":"local_bash","uuid":"201db2ef","session_id":"f80fd7a9"}"#;
const MONITOR_STARTED: &str = r#"{"type":"system","subtype":"task_started","task_id":"bciisie4k","run_id":"0mvb9af3h-d5804741","tool_use_id":"toolu_01XVtqtRtkKsJ64duwf45juF","description":"ticker","is_backgrounded":true,"task_type":"local_bash","uuid":"5d05a3fe","session_id":"f80fd7a9"}"#;
const FOREGROUND_STARTED: &str = r#"{"type":"system","subtype":"task_started","task_id":"b7ywoajwn","tool_use_id":"toolu_0183JjKtMu3aiSdZobZu5kqJ","description":"python3 -c 'import time; time.sleep(40)'","is_backgrounded":false,"task_type":"local_bash","uuid":"852a104e","session_id":"2bab36eb"}"#;

const OK_UPDATED: &str = r#"{"type":"system","subtype":"task_updated","task_id":"bsr32fcbn","run_id":"0mvb9acly-75a717e1","patch":{"status":"completed","end_time":1792172592999},"uuid":"fc1daf9f","session_id":"f80fd7a9"}"#;
const BAD_UPDATED: &str = r#"{"type":"system","subtype":"task_updated","task_id":"bvz7c6w5u","run_id":"0mvb9acpe-f97ac3ca","patch":{"status":"failed","end_time":1792172593076},"uuid":"1df5ca9e","session_id":"f80fd7a9"}"#;
const KILLED_UPDATED: &str = r#"{"type":"system","subtype":"task_updated","task_id":"b2qi6vicy","patch":{"status":"killed","end_time":1787484796558},"uuid":"33dc3a46","session_id":"d6ee7ba1"}"#;

const OK_NOTICE: &str = r#"{"type":"system","subtype":"task_notification","task_id":"bsr32fcbn","run_id":"0mvb9acly-75a717e1","tool_use_id":"toolu_015gM3DJAc7ui6cBMn6qAUYb","status":"completed","output_file":"/private/tmp/claude-501/-private-tmp-bgrec-work/f80fd7a9/tasks/bsr32fcbn.output","summary":"Background command \"ok job\" completed (exit code 0)","uuid":"2864e7e9","session_id":"f80fd7a9"}"#;
const BAD_NOTICE: &str = r#"{"type":"system","subtype":"task_notification","task_id":"bvz7c6w5u","run_id":"0mvb9acpe-f97ac3ca","tool_use_id":"toolu_01YCkLXEXmRejWiyFcaHVkk3","status":"failed","output_file":"/private/tmp/claude-501/-private-tmp-bgrec-work/f80fd7a9/tasks/bvz7c6w5u.output","summary":"Background command \"bad job\" failed with exit code 3","uuid":"5dc1762c","session_id":"f80fd7a9"}"#;
const MONITOR_NOTICE: &str = r#"{"type":"system","subtype":"task_notification","task_id":"bciisie4k","run_id":"0mvb9af3h-d5804741","tool_use_id":"toolu_01XVtqtRtkKsJ64duwf45juF","status":"completed","output_file":"/private/tmp/claude-501/-private-tmp-bgrec-work/f80fd7a9/tasks/bciisie4k.output","summary":"Monitor \"ticker\" stream ended","uuid":"6bf8fa68","session_id":"f80fd7a9"}"#;
const STOPPED_NOTICE: &str = r#"{"type":"system","subtype":"task_notification","task_id":"b2qi6vicy","tool_use_id":"toolu_01CLcgLc1QCRUB9uWSBjEcRi","status":"stopped","output_file":"/private/tmp/claude-501/tasks/b2qi6vicy.output","summary":"Sleep for 40 seconds","uuid":"b4a58822","session_id":"d6ee7ba1"}"#;

fn ended(task: &str, status: TaskStatus, output: Option<&str>, exit: Option<i32>) -> Vec<DroneEvent> {
    vec![DroneEvent::TaskEnded {
        task: task.into(),
        status,
        output: output.map(String::from),
        exit,
    }]
}

#[test]
fn a_background_command_and_a_watch_are_read_as_started_with_the_call_that_asked() {
    assert_eq!(
        read(OK_STARTED),
        vec![DroneEvent::TaskStarted {
            task: "bsr32fcbn".into(),
            call: "toolu_015gM3DJAc7ui6cBMn6qAUYb".into(),
            description: "ok job".into(),
            backgrounded: true,
        }]
    );
    assert_eq!(
        read(MONITOR_STARTED),
        vec![DroneEvent::TaskStarted {
            task: "bciisie4k".into(),
            call: "toolu_01XVtqtRtkKsJ64duwf45juF".into(),
            description: "ticker".into(),
            backgrounded: true,
        }]
    );
}

/// A foreground command is a task too, and the line says it did not run beside the turn.
#[test]
fn a_foreground_command_says_it_was_not_backgrounded() {
    let DroneEvent::TaskStarted { backgrounded, .. } = &read(FOREGROUND_STARTED)[0] else {
        panic!("not a started task")
    };
    assert!(!backgrounded);
}

#[test]
fn the_status_change_names_how_it_ended_in_the_ledgers_words() {
    assert_eq!(read(OK_UPDATED), ended("bsr32fcbn", TaskStatus::Completed, None, None));
    assert_eq!(read(BAD_UPDATED), ended("bvz7c6w5u", TaskStatus::Failed, None, None));
    assert_eq!(read(KILLED_UPDATED), ended("b2qi6vicy", TaskStatus::Killed, None, None));
}

/// A patch that moves something other than the status is not an end.
#[test]
fn a_patch_without_a_final_status_is_not_an_end() {
    for patch in [r#"{"is_backgrounded":true}"#, r#"{"status":"running"}"#] {
        let line = format!(r#"{{"type":"system","subtype":"task_updated","task_id":"t","patch":{patch}}}"#);
        assert_eq!(
            read(&line),
            vec![DroneEvent::Unrecognised { kind: "system/task_updated".into() }],
            "{patch}"
        );
    }
}

#[test]
fn the_notice_carries_where_the_output_went_and_the_exit_code() {
    assert_eq!(
        read(OK_NOTICE),
        ended(
            "bsr32fcbn",
            TaskStatus::Completed,
            Some("/private/tmp/claude-501/-private-tmp-bgrec-work/f80fd7a9/tasks/bsr32fcbn.output"),
            Some(0)
        )
    );
    assert_eq!(
        read(BAD_NOTICE),
        ended(
            "bvz7c6w5u",
            TaskStatus::Failed,
            Some("/private/tmp/claude-501/-private-tmp-bgrec-work/f80fd7a9/tasks/bvz7c6w5u.output"),
            Some(3)
        )
    );
}

/// A watch has no exit code, and a stop says only what was stopped.
#[test]
fn a_notice_without_an_exit_code_has_none_and_stopped_is_killed() {
    let DroneEvent::TaskEnded { status, exit, .. } = &read(MONITOR_NOTICE)[0] else {
        panic!("not an end")
    };
    assert_eq!((*status, *exit), (TaskStatus::Completed, None));
    let DroneEvent::TaskEnded { status, exit, .. } = &read(STOPPED_NOTICE)[0] else {
        panic!("not an end")
    };
    assert_eq!((*status, *exit), (TaskStatus::Killed, None));
}

/// The summary quotes the description, which is the agent's own text.
#[test]
fn an_exit_code_in_the_description_is_not_the_commands() {
    let line = r#"{"type":"system","subtype":"task_notification","task_id":"t","status":"completed","output_file":"o","summary":"Background command \"retry after exit code 9\" completed (exit code 0)"}"#;
    let DroneEvent::TaskEnded { exit, .. } = &read(line)[0] else { panic!("not an end") };
    assert_eq!(*exit, Some(0));

    let stopped = r#"{"type":"system","subtype":"task_notification","task_id":"t","status":"stopped","output_file":"o","summary":"retry after exit code 9"}"#;
    let DroneEvent::TaskEnded { exit, .. } = &read(stopped)[0] else { panic!("not an end") };
    assert_eq!(*exit, None);
}

#[test]
fn only_a_shell_command_and_a_watch_are_background_tasks_of_a_call() {
    assert_eq!(background_form("Bash"), Some("command"));
    assert_eq!(background_form("Monitor"), Some("monitor"));
    for other in ["Task", "Agent", "Read", "Write", "TaskStop", "mcp__armada__Bash"] {
        assert_eq!(background_form(other), None, "{other}");
    }
}
