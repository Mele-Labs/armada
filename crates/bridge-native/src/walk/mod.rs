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
    /// A press in the middle of the target, which is gone once it is pressed: the step is reached
    /// when the target is no longer drawn.
    Gone,
    /// A key, as gpui writes one (`j`, `enter`, `cmd-k`), sent to the screen.
    Key(&'static str),
    /// A right press on the step's own target, before the target is ringed: the target is
    /// what the press opens, so the press aims at `on`, a row's name.
    RightPress(&'static str),
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
            step("A press on a Job opens it in the middle", Verb::Press, "row", "Fix the merge line freshness chip"),
            step("The head carries the Job's state and its title", Verb::Look, "row", "Thread: Fix the merge line freshness chip"),
            step("The thread opens at its newest row and follows it", Verb::Press, "region", "Thread"),
            step("Home goes to the start of the thread", Verb::Key("home"), "region", "Thread"),
            step("A run of calls is one line; a press opens it", Verb::Press, "button", "Tool calls: Read, Grep, Edit"),
            step("Page down reads on", Verb::Key("pagedown"), "region", "Thread"),
            step("Page down reads on", Verb::Key("pagedown"), "region", "Thread"),
            step("Page down reads on", Verb::Key("pagedown"), "region", "Thread"),
            step("Page down reads on", Verb::Key("pagedown"), "region", "Thread"),
            step("Page down reads on", Verb::Key("pagedown"), "region", "Thread"),
            step("End goes back to the newest row", Verb::Key("end"), "region", "Thread"),
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
    Walk {
        name: "job-view-composer",
        title: "The open Job: its head and message box",
        screen: Screen::Jobs,
        scenario: "job-view-composer",
        steps: &[
            step("A press on a Job opens it: the head, the thread and the box", Verb::Press, "row", "Fix the merge line freshness chip"),
            step("The head's mark says the Job is working, and Stop stands beside More", Verb::Look, "button", "Stop"),
            step("More opens the tab's menu", Verb::Press, "button", "More"),
            step("Esc shuts it and the keys go back to where they were", Verb::Key("escape"), "row", "Thread: Fix the merge line freshness chip"),
            step("Shift+F10 opens the same menu from the keys", Verb::Key("shift-f10"), "menuitem", "Pause"),
            step("Esc shuts it again", Verb::Key("escape"), "row", "Thread: Fix the merge line freshness chip"),
            step("A right press on the tab opens it too", Verb::RightPress("Thread: Fix the merge line freshness chip"), "menuitem", "Rename"),
            step("↓ to the first entry", Verb::Key("down"), "menuitem", "Rename"),
            step("Enter turns the title into a field", Verb::Key("enter"), "textbox", "Rename Fix the merge line freshness chip"),
            step("End puts the caret after the title", Verb::Key("end"), "textbox", "Rename Fix the merge line freshness chip"),
            step("The title is typed in place", Verb::Key("s"), "textbox", "Rename Fix the merge line freshness chip"),
            step("Enter saves it, and the tab reads the new title at once", Verb::Key("enter"), "row", "Thread: Fix the merge line freshness chips"),
            step("Stop ends the turn: the button goes the moment it is pressed", Verb::Gone, "button", "Stop"),
            step("Release lists what goes back", Verb::Press, "button", "Release everything the Job holds"),
            step("Esc keeps it", Verb::Key("escape"), "button", "Release everything the Job holds"),
            step("Another Job: its box, its pickers and its quick bar", Verb::Press, "row", "Audit the docs index"),
            step("The box takes the words, and the keys with them", Verb::Press, "textbox", "Message"),
            step("A slash at the start", Verb::Type("/co"), "textbox", "Message"),
            step("It lists the agent's commands", Verb::Look, "list", "Skills and commands"),
            step("Enter completes the command", Verb::Key("enter"), "textbox", "Message"),
            step("A hash", Verb::Type("#33"), "textbox", "Message"),
            step("It lists the repository's issues and pull requests", Verb::Look, "list", "Tag an issue or pull request"),
            step("Enter makes it one chip in the text", Verb::Key("enter"), "textbox", "Message"),
            step("An at sign", Verb::Type("@nat"), "textbox", "Message"),
            step("It lists the other Jobs, then files", Verb::Look, "list", "Tag a Job or a file"),
            step("Enter makes the Job a chip too", Verb::Key("enter"), "textbox", "Message"),
            step("Attach adds a file; its tile has a Remove", Verb::Press, "button", "Attach file"),
            step("The tray holds what is attached", Verb::Look, "group", "Attached"),
            step("The permission mode is a menu", Verb::Press, "button", "Permission mode"),
            step("↓ to Ask, which asks before it edits or runs anything", Verb::Key("down"), "menu", "Permission mode"),
            step("Enter picks it, and the box says so at once", Verb::Key("enter"), "button", "Permission mode"),
            step("The model and effort picker opens upward", Verb::Press, "button", "Model Sonnet 5.5, effort low"),
            step("The arrows move the model and slide the effort", Verb::Key("up"), "dialog", "Model and effort"),
            step("Right slides the effort up", Verb::Key("right"), "dialog", "Model and effort"),
            step("Esc puts it away; the trigger names the pick", Verb::Key("escape"), "button", "Model Opus 5.5, effort medium"),
            step("The quick bar keeps lines he saved, pinned first", Verb::Look, "button", "Keep going"),
            step("A right press on a line opens its menu", Verb::RightPress("Keep going"), "menuitem", "Unpin"),
            step("Esc shuts it", Verb::Key("escape"), "button", "Keep going"),
            step("The box is in front again", Verb::Press, "textbox", "Message"),
            step("Command and a digit sends that line, and the draft stays", Verb::Key("cmd-3"), "textbox", "Message"),
            step("Send sends the draft with its chips and its file", Verb::Press, "button", "Send"),
            step("Another Job: a failed hand off stands on the box", Verb::Press, "row", "Native Bridge foundation"),
            step("Retry starts it again: the band goes the moment it is pressed", Verb::Gone, "button", "Retry"),
            step("Auto land is a press in the toolbelt", Verb::Press, "button", "Auto land"),
            step("Pause is another, and it reads Resume the moment it is pressed", Verb::Gone, "button", "Pause"),
            step("Resume puts it back", Verb::Look, "button", "Resume"),
            step("A paused Job asks before a word is sent", Verb::Press, "textbox", "Message"),
            step("Words are typed", Verb::Type("Pick this up"), "textbox", "Message"),
            step("Enter asks first", Verb::Key("enter"), "dialog", "Job is paused"),
            step("Esc gives the box its words back", Verb::Key("escape"), "textbox", "Message"),
            step("End job takes the box away", Verb::Press, "row", "Audit the docs index"),
            step("A right press on the tab", Verb::RightPress("Thread: Audit the docs index"), "menuitem", "End job"),
            step("↑ to End job", Verb::Key("up"), "menuitem", "End job"),
            step("Enter closes it at once when nothing is in flight, and the box goes", Verb::Key("enter"), "row", "Thread: Audit the docs index"),
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
