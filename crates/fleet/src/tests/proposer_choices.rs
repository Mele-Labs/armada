//! The models the proposer may pick are the roster narrowed by `models.proposerChoices`.

use crate::models_read::proposer_choices;

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|name| name.to_string()).collect()
}

#[test]
fn haiku_stays_on_the_roster_but_the_proposer_is_not_offered_it() {
    let held = names(&["opus", "fable", "sonnet", "haiku"]);
    assert_eq!(proposer_choices(held, &names(&["opus", "sonnet"])), names(&["opus", "sonnet"]));
}

#[test]
fn a_choice_off_the_roster_is_not_offered() {
    let held = names(&["opus", "haiku"]);
    assert_eq!(proposer_choices(held, &names(&["opus", "sonnet"])), names(&["opus"]));
}

#[test]
fn a_narrowing_that_leaves_nothing_leaves_the_roster() {
    let held = names(&["haiku"]);
    assert_eq!(proposer_choices(held.clone(), &names(&["opus", "sonnet"])), held);
}
