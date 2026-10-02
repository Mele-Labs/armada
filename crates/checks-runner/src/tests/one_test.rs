//! Running one test by name. The name is a Drone's or a person's words, and
//! every runner a `one_test` is written for here reads it as a regex. #999.

use crate::matched::one_test_count;
use crate::narrow::one_test;
use crate::run::{split, Output};

const RUN: &str = "cargo nextest run -E test(={})";

/// The arguments `one_test`'s command reaches the runner as, once the one
/// splitter has taken its quotes off.
fn arguments(template: &str, test: &str) -> Vec<String> {
    let command = one_test(template, test).expect("a name that is not blank runs");
    split(&command).expect("a program").1
}

#[test]
fn the_name_goes_where_the_command_says() {
    assert_eq!(
        one_test(RUN, "  store::reads_the_last_row "),
        Some("cargo nextest run -E test(=store::reads_the_last_row)".to_string())
    );
}

#[test]
fn a_blank_name_runs_nothing() {
    assert_eq!(one_test(RUN, "   "), None);
}

#[test]
fn a_name_holding_a_space_stays_one_argument() {
    assert_eq!(
        one_test("run {}", "reads the last row"),
        Some("run \"reads the last row\"".to_string())
    );
}

/// **An apostrophe is in ordinary test names**, and was refused outright, so
/// `desktop_test`'s one-test form offered the whole suite instead.
#[test]
fn a_name_holding_an_apostrophe_runs_that_test() {
    assert_eq!(
        arguments("vitest run -t {}", "a node's bar fills the row"),
        ["run", "-t", "a node's bar fills the row"]
    );
}

/// **A quote cannot break out of the one argument a name is.** Each run of
/// one kind is wrapped in the other, and the splitter hands it back whole.
#[test]
fn a_name_holding_both_quotes_stays_one_argument() {
    assert_eq!(
        arguments("vitest run -t {}", "x' --workspace \"y\" '"),
        ["run", "-t", "x' --workspace \"y\" '"]
    );
}

/// `vitest -t` and nextest's `/…/` are regexes, so a bracket or a dot in a
/// name is escaped to match itself rather than read as a pattern.
#[test]
fn a_name_holding_regex_characters_matches_itself() {
    assert_eq!(
        arguments(
            "vitest run -t {}",
            "shows $1.50 (or more) [x] a|b * c? +{d} ^/\\"
        ),
        [
            "run",
            "-t",
            "shows \\$1\\.50 \\(or more\\) \\[x\\] a\\|b \\* c\\? \\+\\{d\\} \\^\\/\\\\"
        ]
    );
}

fn printed(stdout: &str, stderr: &str) -> Output {
    Output {
        stdout: stdout.to_string(),
        stderr: stderr.to_string(),
        truncated: false,
    }
}

/// A bare function name can match more than one test, and the count is said.
#[test]
fn nextests_summary_says_how_many_ran() {
    let output = printed(
        "",
        "     Summary [   0.012s] 3 tests run: 3 passed, 2031 skipped\n",
    );
    assert_eq!(one_test_count(&output), Some(3));
}

#[test]
fn vitests_summary_says_how_many_ran() {
    let output = printed("      Tests  2 passed | 1176 skipped (1178)\n", "");
    assert_eq!(one_test_count(&output), Some(2));
}

#[test]
fn no_summary_says_no_count() {
    assert_eq!(one_test_count(&printed("hello\n", "")), None);
}
