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
    /// A press on the element of this role and name, before the target is ringed: the target is what the
    /// press shows, so the button itself is gone by then.
    Open(&'static str, &'static str),
    /// A right press on the element of this role and name, before the target is ringed: the target is the menu it opens.
    RightOpen(&'static str, &'static str),
    /// The pointer brought to rest on the target.
    Hover,
    /// A press in the middle of the target, then these words typed into whatever it focused.
    Type(&'static str),
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
    Walk {
        name: "job-view",
        title: "The open Job",
        screen: Screen::Jobs,
        scenario: "job-view",
        steps: &[
            step("A press on a Job opens it in the middle; what waits on you is up", Verb::Press, "row", "Fix the merge line freshness chip"),
            step("What waits on you stays up", Verb::Look, "button", "Needs you"),
            step("The question opens whole", Verb::Open("button", "Open Which colour does the stale chip take?"), "button", "Back"),
            step("One answer picks and moves to the next question", Verb::Open("radio", "Amber"), "checkbox", "In the tooltip"),
            step("A second question takes several", Verb::Press, "checkbox", "In the tooltip"),
            step("Answer goes to Fleet and waits for its word", Verb::Open("button", "Answer"), "button", "Needs you"),
            step("The band folds the panel", Verb::Press, "button", "Needs you"),
            step("A press in the thread gives it the keys", Verb::Press, "region", "Thread"),
            step("The plan and the pinned sketch wait at the end", Verb::Key("pageup"), "region", "Thread"),
            step("The plan says what each group gives; the groups stay behind Details", Verb::Look, "button", "Details"),
            step("Details opens the groups and what changed", Verb::Open("button", "Details"), "button", "Show less"),
            step("Page down reaches the plan's answers", Verb::Key("pagedown"), "button", "Suggest"),
            step("⌥E opens the plan's note, as Suggest does", Verb::Key("alt-e"), "textbox", "Note"),
            step("The suggestion is typed", Verb::Type("Name the colour in the tooltip"), "textbox", "Note"),
            step("Send changes goes to Fleet", Verb::Open("button", "Send changes"), "button", "Suggest"),
            step("Not yet puts the plan away", Verb::Open("button", "Not yet"), "button", "Plan set aside"),
            step("⌥X asks which waiting card to withdraw", Verb::Key("alt-x"), "radiogroup", "Waiting cards"),
            step("Esc puts the dialog away", Verb::Key("escape"), "region", "Thread"),
            step("Withdraw answers the pinned sketch", Verb::Open("button", "Withdraw"), "button", "Revision 2"),
            step("Page up reads the cards before it", Verb::Key("pageup"), "region", "Thread"),
            step("Page up reads the cards before it", Verb::Key("pageup"), "region", "Thread"),
            step("Page up reads the cards before it", Verb::Key("pageup"), "region", "Thread"),
            step("Home goes to the start of the thread", Verb::Key("home"), "region", "Thread"),
            step("Older items are read from Fleet by a press, or as the top comes near", Verb::Look, "button", "Show earlier"),
            step("A run of calls is one line; a press opens it", Verb::Press, "button", "Tool calls: Read, Grep, Edit"),
            step("Page down reads on", Verb::Key("pagedown"), "region", "Thread"),
            step("Page down reads on", Verb::Key("pagedown"), "region", "Thread"),
            step("Page down reads on", Verb::Key("pagedown"), "region", "Thread"),
            step("A right press on a command offers Teach", Verb::RightOpen("listitem", "/reload-plugins"), "menuitem", "Teach"),
            step("Esc shuts the menu", Verb::Key("escape"), "listitem", "/reload-plugins"),
            step("Page down reads on", Verb::Key("pagedown"), "region", "Thread"),
            step("A Fleet turn is one dim row", Verb::Look, "region", "Fleet turn"),
            step("A right press on it offers Teach and the three mutes", Verb::RightOpen("region", "Fleet turn"), "menuitem", "For repo"),
            step("Esc shuts the menu and the row has focus again", Verb::Key("escape"), "button", "Fleet turn"),
            step("Shift+F10 opens the same menu under the row", Verb::Key("shift-f10"), "menuitem", "For 1 hour"),
            step("Esc shuts it and the row has focus again", Verb::Key("escape"), "button", "Fleet turn"),
            step("Page down reads on", Verb::Key("pagedown"), "region", "Thread"),
            step("End goes back to the newest row", Verb::Key("end"), "region", "Thread"),
            step("Subagents running are one card at the foot; the list opens behind it", Verb::Open("button", "1 subagent running"), "row", "Subagent Read the chip's stories"),
            step("Another Job: a long thread, at its newest row", Verb::Press, "row", "Native Bridge foundation"),
            step("Its calls fold to one line", Verb::Press, "button", "Tool calls: Read, Grep, Bash"),
            step("Words typed in the box stay with this Job", Verb::Type("Check the webhook queue too"), "textbox", "Message"),
            step("Another Job opens with its own box and place", Verb::Press, "row", "Fix the merge line freshness chip"),
            step("Back: the place, the open run and the draft are as they were", Verb::Press, "row", "Native Bridge foundation"),
            step("Enter sends: the words are in the thread at once", Verb::Press, "textbox", "Message"),
            step("Sent", Verb::Key("enter"), "textbox", "Message"),
            step("A Job whose thread has not come draws its shape", Verb::Press, "row", "Quiet the write-clash notices"),
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
