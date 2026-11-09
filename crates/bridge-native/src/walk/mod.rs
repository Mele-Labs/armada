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
pub mod title_bar;
pub mod window;

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
    /// The title bar moves to a named scene of its mock scenario (`titlebar/scenes.rs`), then the
    /// target is ringed. A scene changes data and presses nothing.
    Scene(&'static str),
    /// A press on another element, `(role, name)`, before the target is ringed: for an act whose
    /// own button is gone once it is pressed.
    PressOn(&'static str, &'static str),
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

const fn scene(say: &'static str, name: &'static str, role: &'static str, target: &'static str) -> Step {
    Step { say, verb: Verb::Scene(name), target: Target { role, name: target } }
}

const fn press_on(say: &'static str, role: &'static str, name: &'static str, then_role: &'static str, then: &'static str) -> Step {
    Step { say, verb: Verb::PressOn(role, name), target: Target { role: then_role, name: then } }
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
    Walk {
        name: "title-bar",
        title: "Title bar",
        screen: Screen::Jobs,
        scenario: "jobs-many",
        steps: &title_bar::STEPS,
    },
    Walk {
        name: "jobs-lamps",
        title: "Jobs list: a tinted hull",
        screen: Screen::Jobs,
        scenario: "jobs-many",
        steps: &[
            step("Each Job's surface takes a faint wash of its lamp's colour", Verb::Look, "row", "Quiet the write-clash notices"),
            step("A Job waiting on you pulses amber", Verb::Look, "img", "Waiting on you"),
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
