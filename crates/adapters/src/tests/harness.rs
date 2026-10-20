//! What the headless agent CLI is told, as a value nothing has to start a
//! process to read.

use adapter_traits::Effort;

use crate::harness::thinking;

pub(super) fn value_after(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|at| args.get(at + 1))
        .cloned()
}

/// **The effort a person set reaches argv, and none sends no flag**, so the
/// CLI's own default stands on every call nobody tuned.
#[test]
fn an_effort_is_rendered_only_where_one_was_set() {
    let mut args = vec![String::from("-p")];
    thinking(&mut args, None);
    assert_eq!(value_after(&args, "--effort"), None);

    thinking(&mut args, Some(Effort::High));
    assert_eq!(value_after(&args, "--effort").as_deref(), Some("high"));
}

/// **The two levels above `high` reach argv as the CLI spells them.**
#[test]
fn xhigh_and_max_reach_the_flag() {
    for (effort, word) in [(Effort::XHigh, "xhigh"), (Effort::Max, "max")] {
        let mut args = vec![String::from("-p")];
        thinking(&mut args, Some(effort));
        assert_eq!(value_after(&args, "--effort").as_deref(), Some(word));
    }
}
