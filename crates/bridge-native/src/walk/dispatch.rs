//! The dispatch sheet walk: each step goes straight to a state of the sheet. The scenes are
//! `dispatch/scenes.rs`; a step that names one changes Fleet's answer and presses nothing. The
//! annotation layer is its own walk, `notes`.

use super::{answer, press_on, step, Step, Verb};

pub static STEPS: [Step; 35] = [
    step("⌘N opens the sheet over the screen, and the keys are in the request", Verb::Key("cmd-n"), "dialog", "Dispatch"),
    step("The Job's glyph stands at the start of the line", Verb::Look, "img", "Job"),
    step("The repository is one picker at the start of the band; Model and Effort sit by the act and show their default", Verb::Look, "combobox", "Repository"),
    press_on("The Model list opens under it", "combobox", "Model", "menu", "Model"),
    step("Esc shuts the list and the keys go back to the request", Verb::Key("escape"), "textbox", "Request"),
    step("Typing reads the repository: the mark turns to a spinner", Verb::Typed("Fix the flaky login test in sample-api"), "img", "Repository, finding"),
    answer("Fleet found sample-api in the words: the picker names it", "found", "combobox", "Repository sample-api"),
    step("⌥4 picks the fourth repository by hand, without opening the list", Verb::Key("alt-4"), "combobox", "Repository sample-docs"),
    answer("Fleet took the Job: the sheet goes and the draft is cleared", "answer-start", "heading", "armada"),
    step("⌘N opens a sheet with nothing kept: the send cleared the draft", Verb::Key("cmd-n"), "dialog", "Dispatch"),
    step("⌥A puts an @ in the request and opens the Jobs to tag", Verb::Key("alt-a"), "listbox", "Jobs"),
    step("↓ moves down the Jobs", Verb::Key("down"), "option", "Native Bridge foundation"),
    step("Enter tags the Job in the words", Verb::Key("enter"), "textbox", "Request"),
    answer("A tagged Job's brief waits on it: the sentence stands over the request", "waiting", "group", "Start condition"),
    step("Each part is a button; Enter or a press cycles its choices", Verb::Key("shift-tab"), "button", "Condition PR lands"),
    step("Esc shuts the sheet and keeps the words", Verb::Key("escape"), "heading", "armada"),
    step("⌘N opens it again with the draft where it was", Verb::Key("cmd-n"), "dialog", "Dispatch"),
    step("# opens the issues and pull requests to tag", Verb::Typed(" #"), "listbox", "Tag an issue or pull request"),
    step("Esc shuts the list first, the sheet second", Verb::Key("escape"), "dialog", "Dispatch"),
    answer("Fleet found no repository in the words: the mark says so", "unresolved", "img", "Repository, not found"),
    step("Start with no repository asks for one: the picker takes the call hue and its list opens, each repository with its key", Verb::Key("enter"), "menu", "Repository"),
    step("Esc shuts the list", Verb::Key("escape"), "combobox", "Repository"),
    answer("Twelve repositories: the first nine carry their keys", "many", "combobox", "Repository"),
    press_on("The list holds all twelve, with a search over them", "combobox", "Repository", "menu", "Repository"),
    step("Esc shuts it", Verb::Key("escape"), "combobox", "Repository"),
    step("Esc again puts the call away: the frame is as it was", Verb::Key("escape"), "dialog", "Dispatch"),
    answer("A pasted snippet stands over the line and can be removed", "attach", "button", "Remove pasted-snippet.txt"),
    answer("With Fleet unreachable everything in the sheet is off, and the title bar's dot says so too", "offline", "status", "Fleet unreachable"),
    answer("A repository not ready: the refusal stands over the band, and Fleet is running again", "refuse-start", "alert", "Job not started: the repository has no free slot"),
    answer("A wait Fleet read from the tag: a task of the Job's plan, from its PR branch", "waiting-live", "button", "Condition a task of its plan is done"),
    answer("A window scoped to a project offers only the repositories it holds", "storefront", "combobox", "Repository"),
    press_on("Its list holds them alone", "combobox", "Repository", "menuitemcheckbox", "sample-web"),
    step("Esc shuts it", Verb::Key("escape"), "combobox", "Repository"),
    answer("A project removed since the window picked it: Fleet's refusal, in its own words, over the band", "refuse-member", "alert", "there is no project storefront"),
    step("Esc shuts the sheet", Verb::Key("escape"), "heading", "armada"),
];
