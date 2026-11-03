//! A Job's terminals on the wire: the IDs the bodies take and the answers and events name, and the
//! list and the rename that are new. A Job's shell is `t1`, `t2` and so on, a plain string.

use crate::{
    decode, encode, Event, JobTerminal, JobTerminals, ListedTerminal, OpenTerminal, ReadTerminal, RenameTerminal,
    SessionId, SessionRunUnderway, SessionTag, TagKind, TerminalScreen, TerminalTyped, TypeInTerminal,
};

#[test]
fn an_open_names_a_terminal_or_asks_for_a_new_one_and_says_neither_by_default() {
    let plain: OpenTerminal = decode("an open", br#"{"session_id":"s1"}"#).unwrap();
    assert_eq!((plain.terminal.as_deref(), plain.new), (None, false));
    assert_eq!(encode(&plain).unwrap(), r#"{"session_id":"s1"}"#, "neither is left out of the bytes");

    let named: OpenTerminal = decode("an open", br#"{"session_id":"s1","terminal":"t2","cols":100}"#).unwrap();
    assert_eq!((named.terminal.as_deref(), named.new, named.cols), (Some("t2"), false, Some(100)));

    let fresh: OpenTerminal = decode("an open", br#"{"session_id":"s1","new":true}"#).unwrap();
    assert_eq!((fresh.terminal.as_deref(), fresh.new), (None, true));
    assert_eq!(encode(&fresh).unwrap(), r#"{"session_id":"s1","new":true}"#);
}

#[test]
fn an_open_answers_the_terminal_of_a_jobs_shell_and_none_for_any_other_run() {
    let other: SessionRunUnderway = decode("a run", br#"{"run_id":"r1"}"#).unwrap();
    assert_eq!(other.terminal, None);
    assert_eq!(encode(&other).unwrap(), r#"{"run_id":"r1"}"#);

    let shell = SessionRunUnderway { run_id: "r2".into(), terminal: Some("t3".into()) };
    assert_eq!(encode(&shell).unwrap(), r#"{"run_id":"r2","terminal":"t3"}"#);
    assert_eq!(decode::<SessionRunUnderway>("a run", encode(&shell).unwrap().as_bytes()).unwrap(), shell);
}

#[test]
fn typing_names_a_terminal_or_asks_for_a_new_one_and_says_neither_by_default() {
    let plain: TypeInTerminal = decode("keys", br#"{"text":"ls"}"#).unwrap();
    assert_eq!((plain.terminal.as_deref(), plain.new, plain.enter), (None, false, true));
    assert_eq!(
        encode(&plain).unwrap(),
        r#"{"text":"ls","enter":true,"clear":true}"#,
        "neither terminal nor new is written when it is not asked"
    );

    let named: TypeInTerminal = decode("keys", br#"{"text":"ls","terminal":"t2"}"#).unwrap();
    assert_eq!((named.terminal.as_deref(), named.new), (Some("t2"), false));

    let fresh: TypeInTerminal = decode("keys", br#"{"text":"ls","new":true}"#).unwrap();
    assert!(fresh.new && fresh.terminal.is_none());
    assert!(encode(&fresh).unwrap().ends_with(r#","new":true}"#));
}

#[test]
fn a_read_names_a_terminal_and_leaves_it_out_when_it_does_not() {
    let plain: ReadTerminal = decode("a read", br#"{"lines":5}"#).unwrap();
    assert_eq!(plain.terminal, None);
    assert_eq!(encode(&plain).unwrap(), r#"{"lines":5}"#);

    let named: ReadTerminal = decode("a read", br#"{"terminal":"t4","until":"ready"}"#).unwrap();
    assert_eq!((named.terminal.as_deref(), named.until.as_deref()), (Some("t4"), Some("ready")));
}

/// **The three required fields.** An answer or an event that does not say which terminal is not
/// one a pane can be keyed on, so it does not decode.
#[test]
fn the_answers_and_the_event_must_name_their_terminal() {
    let typed = br#"{"run_id":"r1","printed":"","still_printing":false,"echoed":true}"#;
    let screen = br#"{"run_id":"r1","screen":""}"#;
    let event = br#"{"session_id":"s1","run_id":"r1"}"#;
    assert!(decode::<TerminalTyped>("an answer", typed).is_err());
    assert!(decode::<TerminalScreen>("an answer", screen).is_err());
    assert!(decode::<JobTerminal>("an event", event).is_err());

    let typed = br#"{"terminal":"t2","run_id":"r1","printed":"","still_printing":false,"echoed":true}"#;
    assert_eq!(decode::<TerminalTyped>("an answer", typed).unwrap().terminal, "t2");
    let screen = br#"{"terminal":"t2","run_id":"r1","screen":""}"#;
    assert_eq!(decode::<TerminalScreen>("an answer", screen).unwrap().terminal, "t2");
}

#[test]
fn both_terminal_events_carry_the_terminal() {
    let told = JobTerminal { session_id: SessionId::carried("s1"), terminal: "t2".into(), run_id: "r9".into() };
    for event in [Event::TerminalOpened(told.clone()), Event::TerminalExited(told.clone())] {
        let written = encode(&event).unwrap();
        assert!(written.contains(r#""terminal":"t2""#), "{written}");
        assert_eq!(decode::<Event>("an event", written.as_bytes()).unwrap(), event);
    }
}

#[test]
fn the_list_and_the_rename_round_trip() {
    let listed = JobTerminals {
        terminals: vec![
            ListedTerminal { terminal: "t1".into(), name: "Terminal 1".into(), run_id: "r1".into(), running: false },
            ListedTerminal { terminal: "t2".into(), name: "dev server".into(), run_id: "r2".into(), running: true },
        ],
    };
    let written = encode(&listed).unwrap();
    assert_eq!(
        written,
        r#"{"terminals":[{"terminal":"t1","name":"Terminal 1","run_id":"r1","running":false},{"terminal":"t2","name":"dev server","run_id":"r2","running":true}]}"#
    );
    assert_eq!(decode::<JobTerminals>("a list", written.as_bytes()).unwrap(), listed);

    let rename: RenameTerminal = decode("a rename", br#"{"session_id":"s1","terminal":"t2","name":""}"#).unwrap();
    assert_eq!((rename.terminal.as_str(), rename.name.as_str()), ("t2", ""));
    assert!(decode::<RenameTerminal>("a rename", br#"{"session_id":"s1","terminal":"t2"}"#).is_err(), "a name is given, if blank");
}

#[test]
fn a_message_can_tag_a_terminal() {
    let tag = SessionTag { kind: TagKind::Terminal, id: "t2".into(), title: "dev server".into(), url: None };
    let written = encode(&tag).unwrap();
    assert_eq!(written, r#"{"kind":"terminal","id":"t2","title":"dev server"}"#);
    assert_eq!(decode::<SessionTag>("a tag", written.as_bytes()).unwrap(), tag);
}
