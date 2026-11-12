//! The terminal Job walk: ⌘T starts one at once in the repository the title bar names and opens it on
//! its shell; the shell keeps ⌥, ⌃ and Esc, ends, and opens again; the tab's menu and + Terminal; then
//! the sheet's Terminal kind and the palette's New terminal. The shells are `terminal::shells::Mock`'s.

use super::{press_on, step, Step, Verb};

pub static STEPS: [Step; 33] = [
    step("⌘T starts a terminal Job at once in the repository the title bar names: it opens on its shell", Verb::Key("cmd-t"), "tab", "Job column: Terminal 1"),
    step("The shell has the keys: its prompt and cursor", Verb::Look, "textbox", "Terminal 1"),
    step("Typing goes to the shell", Verb::Typed("git status"), "textbox", "Terminal 1"),
    step("⌥← reaches the shell: the cursor goes back a word, to before status", Verb::Key("alt-left"), "textbox", "Terminal 1"),
    step("Esc reaches the shell too, and the keys stay in it", Verb::Key("escape"), "textbox", "Terminal 1"),
    step("Enter runs the line", Verb::Key("enter"), "textbox", "Terminal 1"),
    step("exit ends the shell", Verb::Typed("exit"), "textbox", "Terminal 1"),
    step("Shell ended: the head offers New shell", Verb::Key("enter"), "button", "New shell"),
    step("New shell opens one in its place, under the same tab", Verb::Gone, "button", "New shell"),
    step("It runs: the head offers End shell", Verb::Look, "button", "End shell"),
    step("A right press on the tab opens its acts", Verb::RightOpen("tab", "Job column: Terminal 1"), "menuitem", "Send to agent"),
    step("Copy terminal ID", Verb::Look, "menuitem", "Copy terminal ID"),
    step("Close, the act that ends the shell", Verb::Look, "menuitem", "Close"),
    step("Esc shuts the menu", Verb::Key("escape"), "tab", "Job column: Terminal 1"),
    press_on("The pane's + offers Terminal after its own tabs", "button", "Add tab: Done", "menuitem", "Terminal"),
    step("Terminal opens a second shell in front", Verb::Gone, "menuitem", "Terminal"),
    step("Terminal 2 stands beside Terminal 1", Verb::Look, "tab", "Job column: Terminal 2"),
    step("⌘N opens the sheet on a Job", Verb::Key("cmd-n"), "img", "Job"),
    step("Tab turns the kind to Terminal: the line takes the terminal's glyph", Verb::Key("tab"), "img", "Terminal"),
    step("The kinds sit together, Terminal chosen", Verb::Look, "tab", "Terminal"),
    press_on("The repository list offers any folder below the repositories", "combobox", "Repository", "menuitemcheckbox", "Choose a folder"),
    step("Esc shuts the list", Verb::Key("escape"), "combobox", "Repository"),
    step("Tab turns it back to Job", Verb::Key("tab"), "img", "Job"),
    step("Tab again to Terminal", Verb::Key("tab"), "img", "Terminal"),
    step("⌥2 picks armada and starts at once, as it sends a Job that has words", Verb::Key("alt-2"), "tab", "Job column: Terminal 1"),
    step("⌘N opens the sheet again, on a Job", Verb::Key("cmd-n"), "img", "Job"),
    step("Tab to Terminal", Verb::Key("tab"), "img", "Terminal"),
    press_on("The repository list opens", "combobox", "Repository", "menuitemcheckbox", "armada"),
    step("A press picks armada and shuts the list", Verb::Gone, "menuitemcheckbox", "armada"),
    step("Enter with no title starts the terminal Job and opens it on its shell", Verb::Key("enter"), "tab", "Job column: Terminal 1"),
    step("⌘K opens the palette", Verb::Key("cmd-k"), "dialog", "Command palette"),
    step("Typing narrows to New terminal, with ⌘T", Verb::Type("terminal"), "searchbox", "Search actions, jobs and settings"),
    step("New terminal stays, the typed letters lit", Verb::Look, "option", "New terminal"),
];
