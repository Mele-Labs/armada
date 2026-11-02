//! What a Job came up with: absent until its opening line is kept, replaced by the next, and read back
//! from the file after a reopen.

use crate::tests::{open, TempDir};
use crate::KeptInit;

const AT: &str = "2026-11-02T10:00:00.000Z";

fn init(tools: &[&str], skills: &[&str]) -> KeptInit {
    KeptInit {
        harness_version: "2.1.310".into(),
        mod_loaded: true,
        door: "connected".into(),
        task_tools: true,
        tools: tools.iter().map(|name| name.to_string()).collect(),
        skills: skills.iter().map(|name| name.to_string()).collect(),
    }
}

#[test]
fn an_opening_line_is_absent_kept_replaced_and_read_back_after_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.init_of("s1").unwrap(), None);

    store.keep_init("s1", &init(&["Bash", "Read"], &["armada:reading-a-brief"]), AT).unwrap();
    assert_eq!(store.init_of("s1").unwrap(), Some(init(&["Bash", "Read"], &["armada:reading-a-brief"])));

    store.keep_init("s1", &init(&["Bash"], &[]), AT).unwrap();
    assert_eq!(store.init_of("s1").unwrap(), Some(init(&["Bash"], &[])), "a later line replaces, and an empty list stays empty");
    assert_eq!(store.init_of("s2").unwrap(), None, "one Job's line is not another's");
    drop(store);

    assert_eq!(open(&dir).init_of("s1").unwrap(), Some(init(&["Bash"], &[])));
}
