//! The word an effort setting is spelled with, read as the level it names.

use adapter_traits::Effort;

use crate::tuning::effort_of;

/// **Every level the CLI takes is one Fleet reads**; the harness default and an
/// unknown word are none.
#[test]
fn a_setting_word_up_to_max_is_an_effort_and_anything_else_is_none() {
    assert_eq!(effort_of("low"), Some(Effort::Low));
    assert_eq!(effort_of("high"), Some(Effort::High));
    assert_eq!(effort_of("xhigh"), Some(Effort::XHigh));
    assert_eq!(effort_of("max"), Some(Effort::Max));
    assert_eq!(effort_of("harness default"), None);
    assert_eq!(effort_of("ultra"), None);
}
