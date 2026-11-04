//! Two Jobs kept in touch read back from either side, with the paths they share.

use crate::tests::{open, TempDir};

#[test]
fn a_pair_kept_in_touch_reads_back_from_either_job_with_the_paths_named_last() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let at = "2026-11-04T10:00:00.000Z";
    store.keep_operator_in_touch("01TERMINAL", "01TABS", &["panes/host.tsx".to_string()], at).expect("kept");
    store.keep_operator_in_touch("01TERMINAL", "01TABS", &["panes/host.tsx".to_string(), "panes/tabs.tsx".to_string()], at).expect("kept again");
    store.keep_operator_in_touch("01OTHER", "01TERMINAL", &["panes/terminal.tsx".to_string()], at).expect("kept");

    let tabs = store.operator_in_touch("01TABS").expect("read");
    assert_eq!(tabs, vec![("01TERMINAL".to_string(), vec!["panes/host.tsx".to_string(), "panes/tabs.tsx".to_string()])]);
    let mut terminal = store.operator_in_touch("01TERMINAL").expect("read");
    terminal.sort();
    assert_eq!(terminal.iter().map(|(one, _)| one.as_str()).collect::<Vec<_>>(), ["01OTHER", "01TABS"]);
    assert!(store.operator_in_touch("01NOBODY").expect("read").is_empty());
}
