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

pub mod all;
pub mod bar;
pub mod check;
pub mod dispatch;
pub mod fleet;
pub mod notes;
pub mod stage;
pub mod stand_in;
pub mod strip_marks;
pub mod title_bar;
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
    /// ⌥⌘A struck as a person strikes it, between steps rather than as a step's own key: the walk
    /// takes a note, wherever the keys are. A step's `Key("alt-cmd-a")` reaches the screen instead.
    NoteKey,
    /// Several keys one after the other, each drawn before the next is sent, as a person's would be.
    Keys(&'static [&'static str]),
    /// A right press on the step's own target, before the target is ringed: the target is
    /// what the press opens, so the press aims at `on`, a row's name.
    RightPress(&'static str),
    /// A press on the element of this role and name, before the target is ringed: the target is what the
    /// press shows, so the button itself is gone by then.
    Open(&'static str, &'static str),
    /// A press on the button of this name inside the article of this name: where one card among several
    /// has a button the others have too.
    OpenIn(&'static str, &'static str),
    /// A right press on the element of this role and name, before the target is ringed: the target is the menu it opens.
    RightOpen(&'static str, &'static str),
    /// A drag across the last line of the element of this role and name, as words are selected.
    Select(&'static str, &'static str),
    /// The pointer brought to rest on the target.
    Hover,
    /// A press in the middle of the target, then these words typed into whatever it focused.
    Type(&'static str),
    /// A right press on another element, `(role, name)`, before the target is ringed: for a thing
    /// that is not a row, such as a place of the title bar or a button the bar's own menu sits over.
    RightPressOn(&'static str, &'static str),
    /// The title bar moves to a named scene of its mock scenario (`titlebar/scenes.rs`), then the
    /// target is ringed. A scene changes data and presses nothing.
    Scene(&'static str),
    /// A press on another element, `(role, name)`, before the target is ringed: for an act whose
    /// own button is gone once it is pressed.
    PressOn(&'static str, &'static str),
    /// Characters typed into whatever holds the keys, one by one, with no press first (`Type` presses the target).
    Typed(&'static str),
    /// A drag across the target's inner area, to hold a box rather than the element.
    Drag,
    /// The dispatch screen moves to a named scene (`dispatch/scenes.rs`): Fleet's answer to a send,
    /// or a state of the sheet's face. A scene changes data and presses nothing.
    Answer(&'static str),
    /// The merge strip, the context ring or the write-clash marks move to a named scene
    /// (`strip_marks.rs`). A scene changes data and presses nothing.
    Marks(&'static str),
    /// The Jobs list's scope picked as a person picks it, from the keys: press the scope button, ↓ to
    /// Paused, Enter. One step for the three, since the list is on the same screen.
    PausedScope,
    /// The screen is built again from its mock, so a part of a long walk starts from the state its
    /// first step expects: no popup, sheet or layer open, every scene at rest. The target is ringed.
    Fresh,
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
    Dispatch,
    /// The Jobs screen with the Job head's stand-in drawn over it, for the walk of the strip and marks.
    Marks,
    /// Every piece at once, for the walk that plays the others in order: the title bar, the merge
    /// strip, the dispatch sheet and the notes layer over the Jobs screen, and the Job head's
    /// stand-in, drawn only while a scene shows the ring.
    All,
}

const fn scene(say: &'static str, name: &'static str, role: &'static str, target: &'static str) -> Step {
    Step { say, verb: Verb::Scene(name), target: Target { role, name: target } }
}

const fn press_on(say: &'static str, role: &'static str, name: &'static str, then_role: &'static str, then: &'static str) -> Step {
    Step { say, verb: Verb::PressOn(role, name), target: Target { role: then_role, name: then } }
}

const fn right_on(say: &'static str, role: &'static str, name: &'static str, then_role: &'static str, then: &'static str) -> Step {
    Step { say, verb: Verb::RightPressOn(role, name), target: Target { role: then_role, name: then } }
}

const fn marks(say: &'static str, name: &'static str, role: &'static str, target: &'static str) -> Step {
    Step { say, verb: Verb::Marks(name), target: Target { role, name: target } }
}

const fn answer(say: &'static str, name: &'static str, role: &'static str, target: &'static str) -> Step {
    Step { say, verb: Verb::Answer(name), target: Target { role, name: target } }
}

const fn fresh(say: &'static str, role: &'static str, target: &'static str) -> Step {
    Step { say, verb: Verb::Fresh, target: Target { role, name: target } }
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
        step("A row is one line, its title and its state, until the View turns more on", Verb::Look, "row", "Fix the merge line freshness chip"),
        step("⌥↑ moves the Job up past its neighbour", Verb::Key("alt-up"), "row", "Audit the docs index"),
        step("Shift+F10 opens the menu from the keys", Verb::Key("shift-f10"), "menuitem", "Close"),
        step("⌥⌘A takes a walk note even while the menu holds the keys: the bar says Capturing", Verb::NoteKey, "button", "Capturing"),
        step("Esc puts the note away and the keys go back to the menu", Verb::Key("escape"), "menuitem", "Close"),
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
        name: "strip-marks",
        title: "Merge strip and Job marks",
        screen: Screen::Marks,
        scenario: "jobs-many",
        steps: &strip_marks::STEPS,
    },
    Walk {
        name: "title-bar",
        title: "Title bar",
        screen: Screen::Jobs,
        scenario: "jobs-many",
        steps: &title_bar::STEPS,
    },
    Walk {
        name: "dispatch",
        title: "Dispatch sheet",
        screen: Screen::Dispatch,
        scenario: "jobs-many",
        steps: &dispatch::STEPS,
    },
    Walk {
        name: "notes",
        title: "Notes on ⌥⌘A",
        screen: Screen::Dispatch,
        scenario: "jobs-many",
        steps: &notes::STEPS,
    },
    Walk {
        name: "native-title-bar",
        title: "Dispatch and the title bar",
        screen: Screen::All,
        scenario: "jobs-many",
        steps: all::STEPS,
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
            step("A sentence Fleet read from a reply has thumbs at its end", Verb::Look, "button", "Not a question"),
            step("A right press on its row offers A question and Not a question", Verb::RightOpen("button", "Open Should the legend come back as a row in the tooltip?"), "menuitem", "Not a question"),
            step("Esc shuts the menu and the row has focus again", Verb::Key("escape"), "button", "Open Should the legend come back as a row in the tooltip?"),
            step("A question confirms it, and the thumbs go but Not a question", Verb::Open("button", "A question"), "button", "Not a question"),
            step("Not a question takes the sentence off the list", Verb::Open("button", "Not a question"), "button", "Needs you"),
            step("The question opens whole", Verb::Open("button", "Open Which colour does the stale chip take?"), "button", "Back"),
            step("One answer picks and moves to the next question", Verb::Open("radio", "Amber"), "checkbox", "In the tooltip"),
            step("A second question takes several", Verb::Press, "checkbox", "In the tooltip"),
            step("Answer goes to Fleet and waits for its word", Verb::Open("button", "Answer"), "button", "Needs you"),
            step("The band folds the panel", Verb::Press, "button", "Needs you"),
            step("⇧⌘Y opens the review sheet, as a plan waits on the Job", Verb::Key("shift-cmd-y"), "dialog", "Fix the merge line freshness chip"),
            step("Esc shuts it and the band is as it was", Verb::Key("escape"), "button", "Needs you"),
            step("A press in the thread gives it the keys", Verb::Press, "region", "Thread"),
            step("The plan and the pinned sketch wait at the end", Verb::Key("pageup"), "region", "Thread"),
            step("The plan says what each group gives; the groups stay behind Details", Verb::Look, "button", "Details"),
            step("The Judge's notes stand under the part of the plan they are about", Verb::Key("pageup"), "group", "Finding: Nothing says what the chip shows when the branch has no upstream"),
            step("A press on a note gives it the keys", Verb::Press, "group", "Finding: The tooltip names a figure no row carries yet"),
            step("J moves to the next note", Verb::Key("j"), "group", "Finding: behind_by counts commits on a branch with no merge base as zero"),
            step("K moves back", Verb::Key("k"), "group", "Finding: The tooltip names a figure no row carries yet"),
            step("Enter sends the note to the Job; it folds to a line and focus passes on", Verb::Key("enter"), "group", "Finding sent: The tooltip names a figure no row carries yet"),
            step("D puts the next one away", Verb::Key("d"), "group", "Finding dismissed: behind_by counts commits on a branch with no merge base as zero"),
            step("Backspace puts away the last", Verb::Key("backspace"), "group", "Finding dismissed: Nothing says what the chip shows when the branch has no upstream"),
            step("Details opens the groups and what changed", Verb::Open("button", "Details"), "button", "Show less"),
            step("Page down reaches the plan's answers", Verb::Key("pagedown"), "button", "Suggest"),
            step("The Judge's press stands after Not yet", Verb::Look, "button", "Judge plan"),
            step("R opens the Judge's box under it", Verb::Key("r"), "dialog", "Judge"),
            step("The box asks what to verify", Verb::Type("existing sign-ins survive the switch"), "textbox", "Verify that"),
            step("Enter runs the Judge; the press reads as reading", Verb::Key("enter"), "button", "Judge plan"),
            step("⌥E opens the plan's note, as Suggest does", Verb::Key("alt-e"), "textbox", "Note"),
            step("The suggestion is typed", Verb::Type("Name the colour in the tooltip"), "textbox", "Note"),
            step("Send changes goes to Fleet", Verb::Open("button", "Send changes"), "button", "Suggest"),
            step("Not yet puts the plan away", Verb::Open("button", "Not yet"), "button", "Plan set aside"),
            step("⌥X asks which waiting card to withdraw", Verb::Key("alt-x"), "radiogroup", "Waiting cards"),
            step("Esc puts the dialog away", Verb::Key("escape"), "region", "Thread"),
            step("Withdraw answers the pinned sketch", Verb::Open("button", "Withdraw"), "button", "Revision 2"),
            step("Page up reads the cards before it", Verb::Key("pageup"), "region", "Thread"),
            step("A workflow draft waits with Approve and Dismiss", Verb::Look, "button", "Approve"),
            step("Page up reads the drafts before it", Verb::Key("pageup"), "region", "Thread"),
            step("Page up reads the drafts before it", Verb::Key("pageup"), "region", "Thread"),
            step("Page up reads the drafts before it", Verb::Key("pageup"), "region", "Thread"),
            step("Page up reads the drafts before it", Verb::Key("pageup"), "group", "Words draft:dra-034"),
            step("Words selected in a draft are picked", Verb::Select("group", "Words draft:dra-034"), "group", "Words draft:dra-034"),
            step("Shift+F10 on them offers Edit and Add note", Verb::Key("shift-f10"), "menuitem", "Add note"),
            step("Add note opens the note box with the words quoted", Verb::Open("menuitem", "Add note"), "textbox", "Note"),
            step("Cancel puts the note away", Verb::Open("button", "Cancel"), "group", "Words draft:dra-034"),
            step("Edit opens the draft's own words in a box", Verb::OpenIn("Draft: Comment on #3290", "Edit"), "textbox", "Draft text"),
            step("The fix is typed", Verb::Type(" and says how far"), "textbox", "Draft text"),
            step("Approve with fixes settles the draft with the person's words", Verb::OpenIn("Draft: Comment on #3290", "Approve with fixes"), "region", "Thread"),
            step("Page down reads the drafts after it", Verb::Key("pagedown"), "region", "Thread"),
            step("Page down reads the workflow draft whole", Verb::Key("pagedown"), "region", "Thread"),
            step("A workflow draft is dismissed from its own row", Verb::OpenIn("Workflow: Workflow quick-fix", "Dismiss"), "status", "Dismissed"),
            step("Approve on a draft settles it as it stands", Verb::OpenIn("Draft: Comment on #3294", "Approve"), "status", "Approved"),
            step("Page down reads on", Verb::Key("pagedown"), "region", "Thread"),
            step("Note opens a box for what to send back", Verb::OpenIn("Draft: Comment on #3295", "Note"), "textbox", "Note"),
            step("The note is typed", Verb::Type("Say which pull request"), "textbox", "Note"),
            step("Send back settles the draft with the note", Verb::OpenIn("Draft: Comment on #3295", "Send back"), "status", "Sent back"),
            step("Home goes to the start of the thread", Verb::Key("home"), "region", "Thread"),
            step("Older items are read from Fleet by a press, or as the top comes near", Verb::Look, "button", "Show earlier"),
            step("Subagents running stay at the foot of the view while the thread is up", Verb::Look, "button", "1 subagent running"),
            step("The Plan chip says a plan waits below the words in view", Verb::Look, "button", "Plan"),
            step("A press on it goes to the plan", Verb::Open("button", "Plan"), "region", "Thread"),
            step("Home goes back to the start", Verb::Key("home"), "button", "Show earlier"),
            step("Debug info folds under the person's words; a press opens it", Verb::Press, "button", "Debug info"),
            step("A run of calls is one line; a press opens it", Verb::Press, "button", "Tool calls: Read, Grep, Edit"),
            step("Page down reads on", Verb::Key("pagedown"), "region", "Thread"),
            step("Page down reads on", Verb::Key("pagedown"), "region", "Thread"),
            step("Selected words bring up a bar of Reply, Fork from here and Teach", Verb::Select("group", "Words m-007"), "toolbar", "Selection"),
            step("Shift+F10 on selected words offers the same three", Verb::Key("shift-f10"), "menuitem", "Fork from here"),
            step("Esc shuts the menu", Verb::Key("escape"), "region", "Thread"),
            step("The pointer on an agent's words brings up its bar", Verb::Hover, "toolbar", "Message"),
            step("Copy puts the words on the clipboard and reads Copied", Verb::Open("button", "Copy message"), "button", "Copied"),
            step("A right press on a quick response offers Edit, Add and Remove", Verb::RightOpen("button", "Yes to this"), "menuitem", "Edit"),
            step("Edit opens the response in a small editor", Verb::Open("menuitem", "Edit"), "dialog", "Edit Yes to this"),
            step("Esc puts the editor away and the response has focus again", Verb::Key("escape"), "button", "Yes to this"),
            step("Page down reads on", Verb::Key("pagedown"), "region", "Thread"),
            step("A right press on a command offers Teach", Verb::RightOpen("listitem", "/reload-plugins"), "menuitem", "Teach"),
            step("Teach opens the panel over the thread, with the command and a box", Verb::Open("menuitem", "Teach"), "textbox", "Instruction"),
            step("The instruction is typed", Verb::Type("Run it once, never twice"), "textbox", "Instruction"),
            step("The reach chip says where it applies", Verb::Look, "button", "Where it applies"),
            step("Teach sends it to Fleet, which reads it back", Verb::Open("button", "Teach"), "group", "Agents"),
            step("Save is the stage's forward button, and Change goes back to the box", Verb::Look, "button", "Change"),
            step("Esc puts the panel away and the command has focus again", Verb::Key("escape"), "listitem", "/reload-plugins"),
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
    Walk {
        name: "job-bands",
        title: "The bands over the box",
        screen: Screen::Jobs,
        scenario: "job-bands",
        steps: &[
            step("A Job held open by what is not you shows a Waiting band", Verb::Press, "row", "Quiet the write-clash notices"),
            step("The band names each wait in a glyph", Verb::Look, "button", "Waiting"),
            step("A press opens a row for each wait, each with its own stop", Verb::Open("button", "Waiting"), "button", "Stop the watch: Held open by ci on branch"),
            step("Esc folds it and the band has focus again", Verb::Key("escape"), "button", "Waiting"),
            step("A Job whose agent came up without its tools says so", Verb::Press, "row", "Upgrade the router"),
            step("Each missing piece has the act that puts it right", Verb::Look, "button", "Restart the agent"),
            step("The cross puts the band away", Verb::Open("button", "Dismiss"), "textbox", "Message"),
            step("A Job with nothing left to do draws its ending over the box, open", Verb::Press, "row", "Dark mode for the settings page"),
            step("The ending lists what it made and the two ways on", Verb::Look, "button", "End job"),
            step("A press in the box puts the caret there; the ending stays up", Verb::Press, "textbox", "Message"),
            step("Esc anywhere in the Job folds it", Verb::Key("escape"), "button", "Nothing left to do"),
            step("A press on the band opens it", Verb::Open("button", "Nothing left to do"), "button", "Keep going"),
            step("⌥X asks which waiting card, with the ending among them", Verb::Key("alt-x"), "radiogroup", "Waiting cards"),
            step("Esc puts the dialog away", Verb::Key("escape"), "button", "Keep going"),
            step("Keep going puts the ending away and the caret goes to the box", Verb::Open("button", "Keep going"), "textbox", "Message"),
        ],
    },
    Walk {
        name: "review-palette",
        title: "Review sheet and command palette",
        screen: Screen::Jobs,
        scenario: "review-palette",
        steps: &[
            step("A press on a Job opens it; what waits on you is up", Verb::Press, "row", "Fix the merge line freshness chip"),
            step("The Review row names the kinds it holds", Verb::Look, "button", "Review"),
            step("Review opens the sheet", Verb::Open("button", "Review"), "dialog", "Fix the merge line freshness chip"),
            step("Items stand oldest first", Verb::Look, "group", "Waiting on you"),
            step("A plan", Verb::Look, "img", "Plan"),
            step("A sketch", Verb::Look, "img", "Sketch"),
            step("A document", Verb::Look, "img", "Document"),
            step("A draft", Verb::Look, "img", "Draft"),
            step("The primary names the item and its key", Verb::Look, "button", "Approve Plan v2"),
            step("Send back, ⌥R", Verb::Look, "button", "Send back"),
            step("Close", Verb::Look, "button", "Close"),
            step("A press on an item shows its body", Verb::Press, "button", "Tail layout"),
            step("↓ moves to the next item", Verb::Key("down"), "button", "Approve Stale branch design"),
            step("↑ moves back", Verb::Key("up"), "button", "Approve Tail layout"),
            step("End goes to the last item; a proposed workflow has no Send back", Verb::Key("end"), "button", "Approve Workflow quick-fix"),
            step("Home goes to the first", Verb::Key("home"), "button", "Approve Plan v2"),
            step("Approve answers in this frame", Verb::Open("button", "Approve Plan v2"), "img", "Approved"),
            step("A right press opens the item's menu", Verb::RightPress("Tail layout"), "menuitem", "Dismiss"),
            step("Esc shuts the menu and the keys return to the list", Verb::Key("escape"), "button", "Approve Tail layout"),
            step("Shift+F10 opens it from the keys", Verb::Key("shift-f10"), "menuitem", "Dismiss"),
            step("Dismiss puts the item away", Verb::Open("menuitem", "Dismiss"), "img", "Dismissed"),
            step("⌥R opens the note on the open item", Verb::Key("alt-r"), "textbox", "Note"),
            step("The note is typed", Verb::Type("Say which branches are affected"), "textbox", "Note"),
            step("⌘↵ sends it back", Verb::Key("cmd-enter"), "img", "Sent back"),
            step("The next open item is chosen", Verb::Look, "button", "Approve Comment on #3290"),
            step("⌥R opens a note again", Verb::Key("alt-r"), "textbox", "Note"),
            step("Esc leaves the note first", Verb::Key("escape"), "button", "Send back"),
            step("Esc shuts the sheet next", Verb::Key("escape"), "button", "Needs you"),
            step("⇧⌘Y opens the sheet from anywhere", Verb::Key("shift-cmd-y"), "dialog", "Fix the merge line freshness chip"),
            step("Tab stays inside the sheet", Verb::Key("tab"), "button", "Close"),
            step("⌘K opens the palette over the sheet", Verb::Key("cmd-k"), "dialog", "Command palette"),
            step("Esc gives the keys back to the sheet", Verb::Key("escape"), "dialog", "Fix the merge line freshness chip"),
            step("Close shuts the sheet", Verb::Open("button", "Close"), "button", "Needs you"),
            step("⌘K opens the palette from anywhere", Verb::Key("cmd-k"), "dialog", "Command palette"),
            step("One field searches", Verb::Look, "searchbox", "Search actions, jobs and settings"),
            step("The open Job heads the list with its acts and their keys", Verb::Look, "heading", "Fix the merge line freshness chip"),
            step("An act of the thread", Verb::Look, "option", "Send back"),
            step("Navigation lists the acts that work anywhere", Verb::Look, "heading", "Navigation"),
            step("An act with its key", Verb::Look, "option", "Message the Job"),
            step("Jobs follow, each with its repository", Verb::Look, "heading", "Jobs"),
            step("A Job row", Verb::Look, "option", "Fix the merge line freshness chip"),
            step("The state is a glyph with a tooltip", Verb::Hover, "img", "Waiting on you"),
            step("Typing narrows the list", Verb::Type("freshness"), "searchbox", "Search actions, jobs and settings"),
            step("Only the Job that matches", Verb::Look, "option", "Fix the merge line freshness chip"),
            step("⌘K on a row opens its acts", Verb::Key("cmd-k"), "menu", "Actions"),
            step("Open, ↵", Verb::Look, "menuitem", "Open"),
            step("Pause, ⌘P", Verb::Look, "menuitem", "Pause"),
            step("Fork, ⌥⌘F", Verb::Look, "menuitem", "Fork"),
            step("A pull request adds two acts, set apart", Verb::Look, "menuitem", "Open pull request"),
            step("Merge pull request, ⇧⌘M", Verb::Look, "menuitem", "Merge pull request"),
            step("Copy Job id, ⇧⌘C", Verb::Look, "menuitem", "Copy Job id"),
            step("Stop is last, in red", Verb::Look, "menuitem", "Stop"),
            step("⇧⌘F pins and the panel stays", Verb::Key("shift-cmd-f"), "menuitem", "Unpin"),
            step("Again unpins", Verb::Key("shift-cmd-f"), "menuitem", "Pin"),
            step("Once more pins it for the next time", Verb::Key("shift-cmd-f"), "menuitem", "Unpin"),
            step("Set alias asks for a word", Verb::Open("menuitem", "Set alias"), "searchbox", "Search for actions"),
            step("The word is typed", Verb::Type("chip"), "searchbox", "Search for actions"),
            step("Enter keeps it", Verb::Key("enter"), "menuitem", "Remove alias"),
            step("Remove alias takes it back and stays", Verb::Open("menuitem", "Remove alias"), "menuitem", "Set alias"),
            step("Set alias again", Verb::Open("menuitem", "Set alias"), "searchbox", "Search for actions"),
            step("The word is typed again", Verb::Type("chip"), "searchbox", "Search for actions"),
            step("Enter keeps it", Verb::Key("enter"), "menuitem", "Remove alias"),
            step("Esc shuts the panel", Verb::Key("escape"), "option", "Fix the merge line freshness chip"),
            step("Esc shuts the palette", Verb::Key("escape"), "button", "Needs you"),
            step("Pinned rows open the list, under Pinned", Verb::Key("cmd-k"), "heading", "Pinned"),
            step("A word he gave a row, typed whole, puts it first", Verb::Type("chip"), "searchbox", "Search actions, jobs and settings"),
            step("The row is found", Verb::Look, "option", "Fix the merge line freshness chip"),
            step("Esc shuts it", Verb::Key("escape"), "button", "Needs you"),
            step("A chord can be kept for any row", Verb::Key("cmd-k"), "dialog", "Command palette"),
            step("Type a Job", Verb::Type("Audit"), "searchbox", "Search actions, jobs and settings"),
            step("The row is found", Verb::Look, "option", "Audit the docs index"),
            step("Its acts", Verb::Key("cmd-k"), "menu", "Actions"),
            step("Record hotkey asks for a chord", Verb::Open("menuitem", "Record hotkey"), "searchbox", "Search for actions"),
            step("⌃⌘J is kept", Verb::Key("ctrl-cmd-j"), "menuitem", "Remove hotkey"),
            step("Esc shuts the panel", Verb::Key("escape"), "option", "Audit the docs index"),
            step("Esc shuts the palette", Verb::Key("escape"), "button", "Needs you"),
            step("The chord opens that Job from anywhere", Verb::Key("ctrl-cmd-j"), "textbox", "Message"),
            step("A paused Job offers Resume", Verb::Key("cmd-k"), "dialog", "Command palette"),
            step("Type a Job", Verb::Type("Pocket"), "searchbox", "Search actions, jobs and settings"),
            step("The row is found", Verb::Look, "option", "Pocket sign-in on the phone"),
            step("Its acts", Verb::Key("cmd-k"), "menu", "Actions"),
            step("Resume in place of Pause", Verb::Look, "menuitem", "Resume"),
            step("Esc shuts the panel", Verb::Key("escape"), "option", "Pocket sign-in on the phone"),
            step("Esc shuts the palette", Verb::Key("escape"), "button", "Needs you"),
            step("⌘K again, a Job to stop", Verb::Key("cmd-k"), "dialog", "Command palette"),
            step("Type a Job", Verb::Type("foundation"), "searchbox", "Search actions, jobs and settings"),
            step("The row is found", Verb::Look, "option", "Native Bridge foundation"),
            step("Its acts", Verb::Key("cmd-k"), "menu", "Actions"),
            step("⌘⌫ asks first", Verb::Key("cmd-backspace"), "dialog", "Stop"),
            step("Cancel keeps the Job", Verb::Open("button", "Cancel"), "menuitem", "Stop"),
            step("⌘⌫ asks again", Verb::Key("cmd-backspace"), "dialog", "Stop"),
            step("Esc shuts the dialog and leaves the panel up", Verb::Key("escape"), "menuitem", "Stop"),
            step("⌘⌫ once more", Verb::Key("cmd-backspace"), "dialog", "Stop"),
            step("Stop goes to Fleet and the palette shuts", Verb::Open("button", "Stop"), "button", "Needs you"),
            step("An act of the screen runs from the palette", Verb::Key("cmd-k"), "dialog", "Command palette"),
            step("Type an act", Verb::Type("message"), "searchbox", "Search actions, jobs and settings"),
            step("The row is found", Verb::Look, "option", "Message the Job"),
            step("Enter runs it and gives the keys to the Job's box", Verb::Key("enter"), "textbox", "Message"),
            step("Nothing found is said", Verb::Key("cmd-k"), "dialog", "Command palette"),
            step("Type what is not there", Verb::Type("zzzz"), "searchbox", "Search actions, jobs and settings"),
            step("The row is found", Verb::Look, "alert", "No match"),
            step("Esc shuts it", Verb::Key("escape"), "textbox", "Message"),
            step("Enter opens the Job under the cursor", Verb::Key("cmd-k"), "dialog", "Command palette"),
            step("Type a Job", Verb::Type("Audit"), "searchbox", "Search actions, jobs and settings"),
            step("The row is found", Verb::Look, "option", "Audit the docs index"),
            step("Enter opens it", Verb::Key("enter"), "row", "Audit the docs index"),
            step("A press on a row opens it too", Verb::Key("cmd-k"), "dialog", "Command palette"),
            step("Type a Job", Verb::Type("Dark mode"), "searchbox", "Search actions, jobs and settings"),
            step("The row is found", Verb::Look, "option", "Dark mode for the settings page"),
            step("A press on the row opens the Job", Verb::Open("option", "Dark mode for the settings page"), "row", "Dark mode for the settings page"),
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
            step("Aa opens the thread's look in a panel, the agent's name first", Verb::Press, "button", "Thread look"),
            step("The panel is a dialog, and the name field has the keys", Verb::Look, "dialog", "Thread look"),
            step("The agent's name is typed, and written once the keys settle", Verb::Type("Atlas"), "textbox", "Agent name"),
            step("Show times takes the stamps out of the thread at once", Verb::Press, "switch", "Show times"),
            step("Text size is a list", Verb::Press, "combobox", "Text size"),
            step("↓ moves to 18 px", Verb::Key("down"), "option", "18 px"),
            step("Enter sets the thread's words larger the same frame", Verb::Key("enter"), "combobox", "Text size"),
            step("Spacing is a list too", Verb::Press, "combobox", "Spacing"),
            step("↓ to Roomy", Verb::Key("down"), "option", "Roomy"),
            step("Enter gives the rows and the messages more room", Verb::Key("enter"), "combobox", "Spacing"),
            step("A response opens its editor under the row", Verb::Press, "button", "Edit Pick this"),
            step("A reaction fills the emoji, and the name follows it", Verb::Press, "button", "Rocket"),
            step("Save keeps it", Verb::Gone, "button", "Save"),
            step("The response row reads it", Verb::Look, "button", "Edit Rocket"),
            step("Add response opens the blank editor", Verb::Press, "button", "Add response"),
            step("Esc puts the editor away and leaves the panel open", Verb::Key("escape"), "dialog", "Thread look"),
            step("Pin keeps a quick reply first, and the button says Unpin", Verb::Gone, "button", "Pin Run the checks and land it"),
            step("Down moves a line one place; Alt+Down does the same from its row", Verb::Look, "button", "Move Keep going down"),
            step("A line Fleet learnt can be saved as his own", Verb::Look, "button", "Save Looks right, go ahead"),
            step("A new quick reply is typed", Verb::Type("Ship it"), "textbox", "New quick reply"),
            step("Add puts it in the list", Verb::Press, "button", "Add"),
            step("Edit turns a line into a field", Verb::Gone, "button", "Edit Ship it"),
            step("The words are ready to be changed", Verb::Look, "textbox", "Edit Ship it"),
            step("End puts the caret after them", Verb::Key("end"), "textbox", "Edit Ship it"),
            step("A letter is typed", Verb::Key("s"), "textbox", "Edit Ship it"),
            step("Enter saves the line", Verb::Key("enter"), "button", "Edit Ship its"),
            step("Down moves a line, and the keys stay on it", Verb::Press, "button", "Move Run the checks and land it down"),
            step("Alt+Up moves it back from anywhere on its row", Verb::Key("alt-up"), "button", "Move Run the checks and land it up"),
            step("Learn from my messages is a switch", Verb::Press, "switch", "Learn from my messages"),
            step("Esc puts the panel away, and the keys go back to Aa", Verb::Key("escape"), "button", "Thread look"),
            step("A line too long for the bar is cut at its edge and scrolls across", Verb::Look, "button", "Run the checks and land it"),
            step("The box is in front again", Verb::Press, "textbox", "Message"),
            step("Talk listens, and is lit the moment it is pressed", Verb::Press, "button", "Talk"),
            step("The words heard join the box as they settle", Verb::Look, "textbox", "Message"),
            step("A second press lets go, and the last words come", Verb::Press, "button", "Talk"),
            step("Command and a digit sends that line, and the draft stays", Verb::Key("cmd-3"), "textbox", "Message"),
            step("Send sends the draft with its chips and its file", Verb::Press, "button", "Send"),
            step("Another Job: a failed hand off stands on the box", Verb::Press, "row", "Native Bridge foundation"),
            step("Retry starts it again: the band goes the moment it is pressed", Verb::Gone, "button", "Retry"),
            step("Auto land is a press in the toolbelt", Verb::Press, "button", "Auto land"),
            step("Pause is another, and it reads Resume the moment it is pressed", Verb::Gone, "button", "Pause"),
            step("Resume puts it back", Verb::Look, "button", "Resume"),
            step("Talk on a Mac that withholds the microphone", Verb::Press, "button", "Talk"),
            step("A card says what to turn on, and where", Verb::Look, "status", "Talk could not hear"),
            step("Esc puts the card away", Verb::Key("escape"), "button", "Talk"),
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
    Walk {
        name: "jobs-screens",
        title: "Jobs: the list, the open Job, the box and the palette",
        screen: Screen::Jobs,
        scenario: "jobs-screens",
        steps: &[
            step("Jobs stand in groups by project; each glyph says the Job's state, and a Job that needs you breathes amber", Verb::Look, "img", "Waiting on you"),
            step("A right press opens the Job's menu", Verb::RightPress("Audit the docs index"), "menuitem", "Pause"),
            step("Esc shuts it", Verb::Key("escape"), "row", "Audit the docs index"),
            step("/ puts the keys in the search from anywhere on the screen", Verb::Key("/"), "searchbox", "Search Jobs"),
            step("↓ gives them back to the list", Verb::Key("down"), "row", "Fix the merge line freshness chip"),
            step("Enter opens the focused Job; what waits on you is up", Verb::Key("enter"), "button", "Needs you"),
            step("More opens the tab's menu", Verb::Open("button", "More"), "menuitem", "Rename"),
            step("↓ to the first entry", Verb::Key("down"), "menuitem", "Rename"),
            step("Enter turns the title into a field", Verb::Key("enter"), "textbox", "Rename Fix the merge line freshness chip"),
            step("End puts the caret after the title", Verb::Key("end"), "textbox", "Rename Fix the merge line freshness chip"),
            step("The title is typed in place", Verb::Key("s"), "textbox", "Rename Fix the merge line freshness chip"),
            step("Enter saves it, and the tab reads the new title at once", Verb::Key("enter"), "row", "Thread: Fix the merge line freshness chips"),
            step("The list's row, the review sheet and the palette read it too", Verb::Look, "row", "Fix the merge line freshness chips"),
            step("The question opens whole", Verb::Open("button", "Open Which colour does the stale chip take?"), "button", "Back"),
            step("One answer picks and moves to the next question", Verb::Open("radio", "Amber"), "checkbox", "In the tooltip"),
            step("A second question takes several", Verb::Press, "checkbox", "In the tooltip"),
            step("Answer goes to Fleet and waits for its word", Verb::Open("button", "Answer"), "button", "Needs you"),
            step("The band folds the panel", Verb::Press, "button", "Needs you"),
            step("A press in the thread gives it the keys", Verb::Press, "region", "Thread"),
            step("Page up reads back from the newest: the plan, with its answers under it", Verb::Key("pageup"), "button", "Details"),
            step("The Judge's notes stand under the part of the plan they are about", Verb::Key("pageup"), "group", "Finding: The tooltip names a figure no row carries yet"),
            step("A press on a note gives it the keys", Verb::Press, "group", "Finding: The tooltip names a figure no row carries yet"),
            step("Enter sends the note to the Job; it folds to a line and focus passes on", Verb::Key("enter"), "group", "Finding sent: The tooltip names a figure no row carries yet"),
            step("R opens the Judge's box under the plan", Verb::Key("r"), "dialog", "Judge"),
            step("Esc puts the box away", Verb::Key("escape"), "region", "Thread"),
            step("Home goes to the start of the thread", Verb::Key("home"), "button", "Show earlier"),
            step("Page down reads on: a run of calls is one line", Verb::Key("pagedown"), "button", "Tool calls: Read, Grep, Edit"),
            step("A press opens the run", Verb::Press, "button", "Tool calls: Read, Grep, Edit"),
            step("Page down reads on", Verb::Key("pagedown"), "region", "Thread"),
            step("Page down reads on", Verb::Key("pagedown"), "region", "Thread"),
            step("A right press on a command offers Teach", Verb::RightOpen("listitem", "/reload-plugins"), "menuitem", "Teach"),
            step("Teach opens the panel over the thread, with the command and a box", Verb::Open("menuitem", "Teach"), "textbox", "Instruction"),
            step("Esc puts the panel away and the command has focus again", Verb::Key("escape"), "listitem", "/reload-plugins"),
            step("⇧⌘Y opens the review sheet; the Job's waiting items stand in it", Verb::Key("shift-cmd-y"), "dialog", "Fix the merge line freshness chips"),
            step("The plan reads as what changed since the last version; Edit plan opens its groups", Verb::Open("button", "Edit plan"), "region", "Chip reads the stale state"),
            step("A press on a task's name makes it a field", Verb::Open("button", "Task: Read behind_by on the row"), "textbox", "Task"),
            step("The new name is typed", Verb::Type(" and its base"), "textbox", "Task"),
            step("Enter keeps it, as a mark not yet sent", Verb::Key("enter"), "button", "Task: Read behind_by on the row and its base"),
            step("The last line of a group takes a new task", Verb::Open("textbox", "New task in Write it down"), "textbox", "New task in Write it down"),
            step("The task is typed", Verb::Type("Say what the amber chip means"), "textbox", "New task in Write it down"),
            step("Enter adds it and the line stays open for the next", Verb::Key("enter"), "button", "Task: Say what the amber chip means"),
            step("Esc shuts the line", Verb::Key("escape"), "textbox", "New task in Write it down"),
            step("Each group's subagent type is a menu", Verb::Open("combobox", "Subagent type of Write it down"), "menuitemcheckbox", "general-purpose"),
            step("A type picked replaces it", Verb::Open("menuitemcheckbox", "rust-engineer"), "combobox", "Subagent type of Write it down"),
            step("Suggest opens a box under the group", Verb::Open("button", "Suggest on group Write it down"), "textbox", "Suggest on Write it down"),
            step("The suggestion is typed", Verb::Type("Link the legend from the chip's tooltip"), "textbox", "Suggest on Write it down"),
            step("Enter pins it under the group", Verb::Key("enter"), "list", "Suggestions"),
            step("A press on an item shows its body", Verb::Press, "button", "Tail layout"),
            step("↓ moves to the next item, ↑ back", Verb::Key("down"), "button", "Approve Stale branch design"),
            step("⌥R opens the note on the open item", Verb::Key("alt-r"), "textbox", "Note"),
            step("The note is typed", Verb::Type("Say which branches are affected"), "textbox", "Note"),
            step("⌘↵ sends it back, and the next open item is chosen", Verb::Key("cmd-enter"), "img", "Sent back"),
            step("⌘↵ is also the primary: it answers the item in view", Verb::Key("cmd-enter"), "img", "Approved"),
            step("Esc shuts the sheet", Verb::Key("escape"), "button", "Needs you"),
            step("The ring under the box says how full the Job's window is; a press opens Compact", Verb::Open("button", "Context 63%"), "button", "Compact"),
            step("Compact sends /compact to the Job in the same frame; the popover goes and the thread drops to its end", Verb::Open("button", "Compact"), "region", "Thread"),
            step("A Job held open by what is not you shows a Waiting band", Verb::Press, "row", "Quiet the write-clash notices"),
            step("A press opens a row for each wait, each with its own stop", Verb::Open("button", "Waiting"), "button", "Stop the watch: Held open by ci on branch"),
            step("Another Job: its box, its pickers and its quick bar", Verb::Press, "row", "Audit the docs index"),
            step("Words are typed", Verb::Type("Check the index again"), "textbox", "Message"),
            step("Enter sends them, and they stand in the thread the same frame", Verb::Key("enter"), "region", "Your message"),
            step("⌘1 sends the first quick line as it is", Verb::Key("cmd-1"), "region", "Your message"),
            step("A slash at the start lists the agent's commands", Verb::Type("/co"), "textbox", "Message"),
            step("Enter completes the command", Verb::Key("enter"), "textbox", "Message"),
            step("Attach adds a file; its tile has a Remove", Verb::Open("button", "Attach file"), "group", "Attached"),
                        step("Talk listens, and is lit the moment it is pressed", Verb::Press, "button", "Talk"),
            step("A second press lets go, and the words heard join the box", Verb::Press, "button", "Talk"),
            step("Aa opens the thread's look in a panel", Verb::Open("button", "Thread look"), "dialog", "Thread look"),
            step("Text size is a list", Verb::Press, "combobox", "Text size"),
            step("A pick sets the size, and the thread behind the panel takes it the same frame", Verb::Open("option", "18 px"), "combobox", "Text size"),
            step("Spacing is a list too", Verb::Press, "combobox", "Spacing"),
            step("A pick gives the rows and the messages more room", Verb::Open("option", "Roomy"), "combobox", "Spacing"),
            step("Esc puts the panel away: the thread reads larger and roomier, and the keys go back to Aa", Verb::Key("escape"), "button", "Thread look"),
            step("⌘K opens the palette from anywhere", Verb::Key("cmd-k"), "dialog", "Command palette"),
            step("Typing narrows the list to the Jobs that match", Verb::Type("freshness"), "searchbox", "Search actions, jobs and settings"),
            step("⌘K on a row opens its acts", Verb::Key("cmd-k"), "menu", "Actions"),
            step("⇧⌘F pins and the panel stays", Verb::Key("shift-cmd-f"), "menuitem", "Unpin"),
            step("Esc shuts the panel", Verb::Key("escape"), "option", "Fix the merge line freshness chips"),
            step("Esc shuts the palette; the list is as it was and the keys are back where they were", Verb::Key("escape"), "textbox", "Message"),
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
