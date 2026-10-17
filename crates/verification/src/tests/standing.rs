//! What a repository requires of every change: in a Judge brief where it
//! names one, before the question, and nowhere where it does not.

use crate::{Standing, STANDING_RULES};

#[test]
fn a_file_with_nothing_in_it_says_nothing() {
    assert_eq!(Standing::read("RULES.md", "\n  \n"), Standing::unstated());
    assert_eq!(Standing::unstated().told(&crate::Wording::shipped()), "");
}

/// A cut that fell inside a character would panic the assembler; this one
/// falls on a line instead, and no line is half-shown.
#[test]
fn a_cut_lands_on_a_whole_line_and_never_inside_a_character() {
    let line = "é".repeat(30) + "\n";
    let text = line.repeat(STANDING_RULES / line.len() + 5);
    let told = Standing::read("RULES.md", &text).told(&crate::Wording::shipped());
    assert!(told.contains("(Cut here. RULES.md is"), "{told}");
    for shown in told.lines().filter(|shown| shown.contains('é')) {
        assert_eq!(shown.trim(), line.trim_end(), "a whole line");
    }
}
