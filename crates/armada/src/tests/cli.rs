//! What was typed, and what a refusal says back.
//!
//! A refusal is read by a person at a terminal who is about to retype the
//! line, so what is asserted here is mostly what the message names — the verbs
//! that exist, the flag that was meant, the arguments that had nowhere to go.

use crate::cli::{read, Fault, Verb};

fn asked(line: &str) -> Result<Verb, crate::cli::Misread> {
    read(line.split_whitespace().map(str::to_string))
}

fn said(line: &str) -> String {
    asked(line).expect_err("this line is refused").to_string()
}

#[test]
fn the_four_verbs_parse() {
    assert_eq!(
        asked("serve"),
        Ok(Verb::Serve { repository: None }),
        "no path means the working directory"
    );
    assert_eq!(
        asked("serve /repos/armada"),
        Ok(Verb::Serve {
            repository: Some("/repos/armada".into())
        })
    );
    assert_eq!(
        asked("check build"),
        Ok(Verb::Check {
            name: "build".to_string(),
            test: None,
            changed: false
        })
    );
    assert_eq!(
        asked("check test a_test"),
        Ok(Verb::Check {
            name: "test".to_string(),
            test: Some("a_test".to_string()),
            changed: false
        }),
        "a second name is one test, through the Check's `one_test`"
    );
    assert_eq!(
        asked("run fmt"),
        Ok(Verb::Run {
            name: "fmt".to_string()
        })
    );
    assert_eq!(
        asked("clean"),
        Ok(Verb::Clean {
            everything: false,
            force: false
        })
    );
    assert_eq!(
        asked("clean --all"),
        Ok(Verb::Clean {
            everything: true,
            force: false
        })
    );
}

/// **Two flags because they are two questions.** `--all` clears this machine's
/// store; `--force` deletes work nobody has taken. Either alone, or both.
#[test]
fn force_and_all_are_separate_answers_and_compose() {
    assert_eq!(
        asked("clean --force"),
        Ok(Verb::Clean {
            everything: false,
            force: true
        })
    );
    assert_eq!(
        asked("clean --all --force"),
        Ok(Verb::Clean {
            everything: true,
            force: true
        })
    );
}

/// **`check` and `run` stay two verbs.** There is no flag on either that turns
/// it into the other, and nothing here parses one.
#[test]
fn there_is_no_flag_that_turns_one_verb_into_the_other() {
    let refused = said("check --command fmt");
    assert!(
        refused.contains("`--command` is not a flag this verb takes"),
        "{refused}"
    );
}

#[test]
fn a_verb_that_does_not_exist_is_answered_with_the_ones_that_do() {
    let refused = said("chekc");
    for verb in ["serve", "check", "run", "clean"] {
        assert!(refused.contains(verb), "the verbs are named: {refused}");
    }
}

#[test]
fn asking_for_nothing_gets_the_usage() {
    let refused = read(Vec::<String>::new()).expect_err("nothing is not a verb");
    assert_eq!(refused.faults, vec![Fault::NothingAsked]);
    assert!(refused.to_string().contains("armada serve"));
}

/// There is no default Check. A verb that ran something when nothing was named
/// is a verb that runs the wrong thing on a typo.
#[test]
fn check_and_run_are_named_at() {
    assert!(said("check").contains("needs the name of one thing the Manifest declares"));
    assert!(said("run").contains("needs the name of one thing the Manifest declares"));
}

#[test]
fn help_is_a_verb_and_a_flag() {
    for spelling in ["help", "--help", "-h"] {
        assert_eq!(asked(spelling), Ok(Verb::Help));
    }
}

/// Every fault, never the first one — the same rule `config` holds for a
/// Manifest, because one correction should fix the whole line.
#[test]
fn a_line_with_two_things_wrong_names_both() {
    let refused = asked("clean --wat extra").expect_err("two faults");
    assert_eq!(refused.faults.len(), 2, "{refused}");
    let said = refused.to_string();
    assert!(said.contains("--wat"));
    assert!(said.contains("extra"));
}

/// `clean` acts where you are standing. A path would let somebody clean a
/// repository they are not looking at, which is the shape the branch mistake
/// had.
#[test]
fn clean_takes_no_path() {
    assert!(said("clean /somewhere/else").contains("nowhere to put"));
}

#[test]
fn a_flag_that_does_not_exist_names_the_one_that_does() {
    assert!(said("clean --everything").contains("it takes `--all`, `--force`"));
}

/// `covers` takes its paths on stdin, so a path typed after it is refused by
/// saying where paths go rather than read as a name.
#[test]
fn covers_parses_and_refuses_a_path_given_as_an_argument() {
    assert_eq!(asked("covers"), Ok(Verb::Covers));
    let refused = said("covers crates/fleet/src/lib.rs");
    assert!(refused.contains("on stdin"), "{refused}");
    assert!(asked("covers x")
        .unwrap_err()
        .faults
        .contains(&Fault::PathsComeOnStdin {
            given: "x".to_string()
        }),);
}

/// `--changed` reads the paths on stdin, as `covers` does, and is a whole
/// Check narrowed: one test beside it has nowhere to go.
#[test]
fn check_takes_the_changed_paths_on_stdin_and_never_beside_one_test() {
    assert_eq!(
        asked("check test --changed"),
        Ok(Verb::Check {
            name: "test".to_string(),
            test: None,
            changed: true
        })
    );
    let refused = said("check test a_test --changed");
    assert!(refused.contains("`--changed`"), "{refused}");
    assert!(said("run fmt --changed").contains("`--changed`"));
}

/// `--withdraw` names a branch the way `--status` does: the one checked out
/// unless another is given.
#[test]
fn worktree_leases_releases_and_lists() {
    use crate::cli::WorktreeAct;
    assert_eq!(
        asked("worktree lease fix-the-gate"),
        Ok(Verb::Worktree(WorktreeAct::Lease {
            branch: "fix-the-gate".to_string()
        }))
    );
    assert_eq!(
        asked("worktree release"),
        Ok(Verb::Worktree(WorktreeAct::Release { path: None }))
    );
    assert_eq!(
        asked("worktree release /repo/.armada/slots/slot-2"),
        Ok(Verb::Worktree(WorktreeAct::Release {
            path: Some("/repo/.armada/slots/slot-2".into())
        }))
    );
    assert_eq!(
        asked("worktree --status"),
        Ok(Verb::Worktree(WorktreeAct::Status))
    );
}

#[test]
fn a_lease_with_no_branch_says_one_is_needed() {
    assert!(said("worktree lease").contains("needs the branch"));
    assert!(said("worktree borrow x").contains("`worktree lease <branch>`"));
}

#[test]
fn withdraw_takes_this_branch_or_the_one_named() {
    use crate::cli::LandAct;
    assert_eq!(
        asked("land --withdraw"),
        Ok(Verb::Land(LandAct::Withdraw { branch: None }))
    );
    assert_eq!(
        asked("land --withdraw fix/one"),
        Ok(Verb::Land(LandAct::Withdraw {
            branch: Some("fix/one".to_string())
        }))
    );
    assert!(said("land --withdrw").contains("`--withdraw`"));
}
