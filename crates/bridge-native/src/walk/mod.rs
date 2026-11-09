//! Native walks: a native screen on mock data, played step by step in a window of its own with a
//! walk bar, the way a `?walk=` page plays in Electron.
//!
//! A Job shows one with `show_window` on `armada://walk/<name>`. The owner presses Open on the
//! Job's tab and Fleet starts the Job's own slot build as `--walk <name> --job <id> --url <url>
//! --fleet-port <port>` (`crates/fleet/src/native_walks.rs`). `--walk <name> --check` plays every
//! step with no window on screen and fails on the first target it cannot find, so a Job plays its
//! walk before it shows it.
//!
//! **Steps live here, beside the scenario they play on**, not in the `show_window` call, so the
//! check and the window play the same list. Each step aims at a `{role, name}` from
//! [`crate::labels`], which is what VoiceOver reads too.

pub mod bar;
pub mod capture;
pub mod check;
pub mod fleet;
pub mod stage;
pub mod window;

use crate::jobs::Lamp;

/// What a step does when it is reached, before its target is ringed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    /// Nothing: the target is ringed.
    Look,
    /// A press in the middle of the target.
    Press,
    /// A key, as gpui writes one (`j`, `enter`, `cmd-k`), sent to the screen.
    Key(&'static str),
    /// A right press on the step's own target, before the target is ringed: the target is
    /// what the press opens, so the press aims at `on`, a row's name.
    RightPress(&'static str),
    /// The pointer brought to rest on the target.
    Hover,
    /// The Jobs list draws its lamps this way, so a walk can put takes side by side.
    Lamp(crate::jobs::Lamp),
}

/// Where a step points: an element's role, in the walk's words, and its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    pub role: &'static str,
    pub name: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub say: &'static str,
    pub verb: Verb,
    pub target: Target,
}

/// One walk: the screen it plays on, the mock data it starts from, and its steps.
#[derive(Debug)]
pub struct Walk {
    pub name: &'static str,
    pub title: &'static str,
    pub screen: Screen,
    pub scenario: &'static str,
    pub steps: &'static [Step],
}

/// The screens a walk can play on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Jobs,
}

const fn step(say: &'static str, verb: Verb, role: &'static str, name: &'static str) -> Step {
    Step { say, verb, target: Target { role, name } }
}

/// Every walk this build can play, by name.
pub static WALKS: &[Walk] = &[
    Walk {
    name: "jobs-list",
    title: "Jobs list",
    screen: Screen::Jobs,
    scenario: "jobs-many",
    steps: &[
        step("Live Jobs, grouped by project", Verb::Look, "heading", "armada"),
        step("Each glyph says the Job's state, and a Job that needs you breathes amber", Verb::Look, "img", "Waiting on you"),
        step("J or ↓ moves down the list", Verb::Key("j"), "row", "Native Bridge foundation"),
        step("K or ↑ moves back up", Verb::Key("k"), "row", "Fix the merge line freshness chip"),
        step("Enter opens the focused Job", Verb::Key("enter"), "row", "Fix the merge line freshness chip"),
        step("A press opens a Job too", Verb::Press, "row", "Checkout page loads twice"),
        step("Each project heads its own Jobs", Verb::Look, "heading", "sample-api"),
        step(
            "A Job another started stands under it, linked",
            Verb::Look,
            "row",
            "Move Bridge's thread rows into Fleet, Linked Job, started by Native Bridge foundation in armada",
        ),
        step("Rest the pointer on a Job and its card opens beside it", Verb::Hover, "row", "Native Bridge foundation"),
        step("A right press opens the Job's menu", Verb::RightPress("Audit the docs index"), "menuitem", "Pause"),
        step("Under the Job's acts, the View group picks what every row draws", Verb::Look, "menuitemcheckbox", "Now"),
        step("Esc shuts it", Verb::Key("escape"), "row", "Audit the docs index"),
        step("Every field is on here: the issue and pull request, the workflow's step, what it is doing, the time", Verb::Look, "row", "Fix the merge line freshness chip"),
        step("⌥↑ moves the Job up past its neighbour", Verb::Key("alt-up"), "row", "Audit the docs index"),
        step("Shift+F10 opens the menu from the keys", Verb::Key("shift-f10"), "menuitem", "Close"),
        step("Esc shuts it again", Verb::Key("escape"), "row", "Audit the docs index"),
        step("The list's edge drags wider or narrower; a double press puts it back", Verb::Press, "separator", "Resize the Jobs list"),
        step("→ widens it from the keys, ← narrows it", Verb::Key("right"), "separator", "Resize the Jobs list"),
        step("/ puts the keys in the search from anywhere on the screen", Verb::Key("/"), "searchbox", "Search Jobs"),
        step("↓ gives them back to the list", Verb::Key("down"), "row", "Fix the merge line freshness chip"),
        step("The scope beside the search picks which Jobs the list holds", Verb::Press, "button", "Active"),
        step("↓ to Paused", Verb::Key("down"), "menuitemcheckbox", "Paused"),
        step("Enter picks it: only paused Jobs, each with its mark", Verb::Key("enter"), "img", "Paused"),
        step("The scope says what the list holds", Verb::Look, "button", "Paused"),
    ],
    },
    // Where a row's colour goes, as three takes of one walk: each sets its look, then rings a Job
    // that is working and one waiting on the owner.
    Walk {
        name: "jobs-lamps-pennant",
        title: "Jobs list: a signal pennant",
        screen: Screen::Jobs,
        scenario: "jobs-many",
        steps: &[
            step("Each Job flies a signal flag in its lamp's colour from its top edge", Verb::Lamp(Lamp::Pennant), "row", "Fix the merge line freshness chip"),
            step("A Job waiting on you flies amber", Verb::Look, "img", "Waiting on you"),
        ],
    },
    Walk {
        name: "jobs-lamps-brackets",
        title: "Jobs list: corner brackets",
        screen: Screen::Jobs,
        scenario: "jobs-many",
        steps: &[
            step("A HUD's corner marks frame each Job in its lamp's colour", Verb::Lamp(Lamp::Brackets), "row", "Fix the merge line freshness chip"),
            step("A Job waiting on you is framed in amber", Verb::Look, "img", "Waiting on you"),
        ],
    },
    Walk {
        name: "jobs-lamps-masthead",
        title: "Jobs list: a masthead light",
        screen: Screen::Jobs,
        scenario: "jobs-many",
        steps: &[
            step("A small light on each Job's top edge, in its lamp's colour", Verb::Lamp(Lamp::Masthead), "row", "Quiet the write-clash notices"),
            step("A Job waiting on you carries amber", Verb::Look, "img", "Waiting on you"),
        ],
    },
    Walk {
        name: "jobs-lamps-waterline",
        title: "Jobs list: a waterline",
        screen: Screen::Jobs,
        scenario: "jobs-many",
        steps: &[
            step("A thin line along the inside of each Job's foot, in its lamp's colour", Verb::Lamp(Lamp::Waterline), "row", "Quiet the write-clash notices"),
            step("A Job waiting on you carries amber", Verb::Look, "img", "Waiting on you"),
        ],
    },
    Walk {
        name: "jobs-lamps-hull",
        title: "Jobs list: a tinted hull",
        screen: Screen::Jobs,
        scenario: "jobs-many",
        steps: &[
            step("No mark: each Job's surface takes a faint wash of its lamp's colour", Verb::Lamp(Lamp::Hull), "row", "Quiet the write-clash notices"),
            step("A Job waiting on you carries amber", Verb::Look, "img", "Waiting on you"),
        ],
    },
    Walk {
        name: "jobs-lamps-fine",
        title: "Jobs list: fine brackets",
        screen: Screen::Jobs,
        scenario: "jobs-many",
        steps: &[
            step("The brackets are the row's own edge, 1pt, bending with its corner", Verb::Lamp(Lamp::Fine), "row", "Native Bridge foundation"),
            step("A Job waiting on you is bracketed in amber", Verb::Look, "img", "Waiting on you"),
        ],
    },
    Walk {
        name: "jobs-lamps-etched",
        title: "Jobs list: etched brackets",
        screen: Screen::Jobs,
        scenario: "jobs-many",
        steps: &[
            step("The brackets are cut into the row's edge, a shadow line inside each", Verb::Lamp(Lamp::Etched), "row", "Native Bridge foundation"),
            step("A Job waiting on you is bracketed in amber", Verb::Look, "img", "Waiting on you"),
        ],
    },
    Walk {
        name: "jobs-lamps-housing",
        title: "Jobs list: a lit housing",
        screen: Screen::Jobs,
        scenario: "jobs-many",
        steps: &[
            step("No edge mark: each Job's state glyph sits in a housing lit in its lamp's colour", Verb::Lamp(Lamp::Housing), "row", "Fix the merge line freshness chip"),
            step("A Job waiting on you is lit amber", Verb::Look, "img", "Waiting on you"),
        ],
    },
];

pub fn named(name: &str) -> Option<&'static Walk> {
    WALKS.iter().find(|walk| walk.name == name)
}

/// The address a Job shows this walk on, which Fleet keys the walk by.
pub fn address_of(walk: &Walk) -> String {
    format!("armada://walk/{}", walk.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_step_names_a_role_the_labels_know_and_every_scenario_decodes() {
        for walk in WALKS {
            crate::mock::jobs(walk.scenario).unwrap_or_else(|why| panic!("{}: {why}", walk.name));
            for step in walk.steps {
                assert!(crate::labels::role_named(step.target.role).is_some(), "{}: {}", walk.name, step.target.role);
            }
        }
    }
}
