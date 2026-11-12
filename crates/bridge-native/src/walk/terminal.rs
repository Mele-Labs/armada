//! The terminal Job walk: ⌘T starts one at once in the repository the title bar names, the sheet's
//! Terminal kind starts one with any folder or repository, and the palette's New terminal is the same
//! act as ⌘T. The shells are `terminal::shells::Mock`'s.

use super::{press_on, step, Step, Verb};

pub static STEPS: [Step; 19] = [
    step("⌘T starts a terminal Job at once in the repository the title bar names, and opens it", Verb::Key("cmd-t"), "region", "Thread"),
    step("⌘N opens the sheet on a Job", Verb::Key("cmd-n"), "img", "Job"),
    step("Tab turns the kind to Terminal: the line takes the terminal's glyph", Verb::Key("tab"), "img", "Terminal"),
    step("The kinds sit together, Terminal chosen", Verb::Look, "tab", "Terminal"),
    press_on("The repository list offers any folder below the repositories", "combobox", "Repository", "menuitemcheckbox", "Choose a folder"),
    step("Esc shuts the list", Verb::Key("escape"), "combobox", "Repository"),
    step("Tab turns it back to Job", Verb::Key("tab"), "img", "Job"),
    step("Tab again to Terminal", Verb::Key("tab"), "img", "Terminal"),
    step("⌥2 picks armada and starts at once, as it sends a Job that has words", Verb::Key("alt-2"), "region", "Thread"),
    step("⌘N opens the sheet again, on a Job", Verb::Key("cmd-n"), "img", "Job"),
    step("Tab to Terminal", Verb::Key("tab"), "img", "Terminal"),
    press_on("The repository list opens", "combobox", "Repository", "menuitemcheckbox", "armada"),
    step("A press picks armada and shuts the list", Verb::Gone, "menuitemcheckbox", "armada"),
    step("The picker names it", Verb::Look, "combobox", "Repository armada"),
    step("Enter with no title starts the terminal Job and opens it", Verb::Key("enter"), "region", "Thread"),
    step("⌘K opens the palette", Verb::Key("cmd-k"), "dialog", "Command palette"),
    step("New terminal is one of the acts that work anywhere, with ⌘T", Verb::Look, "option", "New terminal"),
    step("Typing narrows the list", Verb::Type("terminal"), "searchbox", "Search actions, jobs and settings"),
    step("New terminal stays, the typed letters lit", Verb::Look, "option", "New terminal"),
];
