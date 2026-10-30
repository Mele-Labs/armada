//! What the System layer reads and what stopping a process says: a log carries where it is, and a
//! stop names a process by pid or by port.

use crate::{ProcessStopped, SessionSystem, StopProcess, SystemLog};

#[test]
fn a_log_keeps_its_path_through_the_wire() {
    let log = SystemLog { name: "session.log".into(), path: "/keepers/a.log".into(), bytes: 42 };
    let text = serde_json::to_string(&log).unwrap();
    assert_eq!(text, r#"{"name":"session.log","path":"/keepers/a.log","bytes":42}"#);
    assert_eq!(serde_json::from_str::<SystemLog>(&text).unwrap(), log);
}

#[test]
fn a_reading_with_logs_round_trips_and_an_unread_figure_leaves_no_key() {
    let read = r#"{"session_id":"s1","processes":[{"pid":9,"command":"vite","port":5501,"state":"running"}],"worktrees":[],"logs":[{"name":"t.jsonl","path":"/h/t.jsonl","bytes":7}]}"#;
    let system: SessionSystem = serde_json::from_str(read).unwrap();
    assert_eq!(system.logs[0].path, "/h/t.jsonl");
    assert_eq!(system.processes[0].port, Some(5501));
    assert_eq!(system.cpu_percent, None);
    assert_eq!(serde_json::to_string(&system).unwrap(), read);
}

#[test]
fn a_stop_names_a_pid_or_a_port_and_the_answer_names_what_ended() {
    let by_pid: StopProcess = serde_json::from_str(r#"{"pid":4242}"#).unwrap();
    assert_eq!((by_pid.pid, by_pid.port, by_pid.session_id.is_none()), (Some(4242), None, true));
    assert_eq!(serde_json::to_string(&by_pid).unwrap(), r#"{"pid":4242}"#);

    let by_port: StopProcess = serde_json::from_str(r#"{"port":5501,"session_id":"s1"}"#).unwrap();
    assert_eq!((by_port.pid, by_port.port), (None, Some(5501)));
    assert_eq!(serde_json::to_string(&by_port).unwrap(), r#"{"session_id":"s1","port":5501}"#);

    let ended = ProcessStopped { pid: 4242, command: "vite --port 5501".into() };
    assert_eq!(serde_json::from_str::<ProcessStopped>(&serde_json::to_string(&ended).unwrap()).unwrap(), ended);
}
