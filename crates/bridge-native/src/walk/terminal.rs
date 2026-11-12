//! The quick Job walk: ⌘T starts a Job at once in the repository the title bar names and opens it on its
//! thread, the keys in its message box; a message sent from there; then its shell from the + Fleet
//! offers it under, a new one beside it, the card under the sheet that starts the same, and the
//! palette's New Job here. The shells are `terminal::shells::Mock`'s.

use super::{press_on, step, Step, Verb};

pub static STEPS: [Step; 39] = [
    step("⌘T starts a Job at once in the repository the title bar names: it opens on its thread, the keys in its message box", Verb::Key("cmd-t"), "textbox", "Message"),
    step("Its row has no title until the first message gives it one: it reads as its id", Verb::Look, "row", "s-7c41e2a9"),
    step("Typing goes to the message box", Verb::Type("Why is the merge line stale?"), "textbox", "Message"),
    step("⌘↵ sends it: the message stands in the thread", Verb::Key("cmd-enter"), "region", "Your message"),
    press_on("The right column's + offers the shell Fleet opened with the Job, and a new one", "button", "Add tab: Done", "menuitem", "Open Terminal 1"),
    step("Open Terminal 1 brings it in as a tab of that pane, in front", Verb::Gone, "menuitem", "Open Terminal 1"),
    step("The shell has the keys: its prompt and cursor", Verb::Look, "textbox", "Terminal 1"),
    step("Typing goes to the shell", Verb::Typed("git status"), "textbox", "Terminal 1"),
    step("⌥← reaches the shell: the cursor goes back a word, to before status", Verb::Key("alt-left"), "textbox", "Terminal 1"),
    step("Esc reaches the shell too, and the keys stay in it", Verb::Key("escape"), "textbox", "Terminal 1"),
    step("Enter runs the line", Verb::Key("enter"), "textbox", "Terminal 1"),
    step("exit ends the shell", Verb::Typed("exit"), "textbox", "Terminal 1"),
    step("Shell ended: the head offers New shell", Verb::Key("enter"), "button", "New shell"),
    step("Enter opens a new shell in its place, under the same tab: the head offers End shell", Verb::Key("enter"), "button", "End shell"),
    step("It runs, and has the keys again", Verb::Look, "textbox", "Terminal 1"),
    step("A right press on the tab opens its acts", Verb::RightOpen("tab", "Job column: Terminal 1"), "menuitem", "Send to agent"),
    step("Copy terminal ID", Verb::Look, "menuitem", "Copy terminal ID"),
    step("Rename, which makes the tab's title a field", Verb::Look, "menuitem", "Rename"),
    step("Close, the act that ends the shell", Verb::Look, "menuitem", "Close"),
    step("Esc shuts the menu", Verb::Key("escape"), "tab", "Job column: Terminal 1"),
    step("A program that keeps running holds the shell", Verb::Typed("pnpm dev"), "textbox", "Terminal 1"),
    step("Enter starts it", Verb::Key("enter"), "textbox", "Terminal 1"),
    step("Close while it runs asks first", Verb::RightOpen("tab", "Job column: Terminal 1"), "menuitem", "Close"),
    step("End Terminal 1: a program is running in it", Verb::Open("menuitem", "Close"), "dialog", "End Terminal 1"),
    step("Cancel keeps the terminal and its program", Verb::Open("button", "Cancel"), "tab", "Job column: Terminal 1"),
    press_on("The + offers Terminal for a new shell", "button", "Add tab: Done", "menuitem", "Terminal"),
    step("Terminal opens a second shell in front, in the pane whose + was pressed", Verb::Gone, "menuitem", "Terminal"),
    step("Terminal 2 stands beside Terminal 1", Verb::Look, "tab", "Job column: Terminal 2"),
    step("⌘N opens the sheet, with New Job in Armada on a card of its own under it", Verb::Key("cmd-n"), "button", "New Job in Armada"),
    step("Tab from the request moves to the card, its edge lit", Verb::Key("tab"), "button", "New Job in Armada"),
    step("Enter on the card shuts the sheet and starts what ⌘T starts: the Job's thread, the keys in its box", Verb::Key("enter"), "textbox", "Message"),
    step("⌘N opens the sheet again", Verb::Key("cmd-n"), "button", "New Job in Armada"),
    press_on("A press on the card does the same", "button", "New Job in Armada", "textbox", "Message"),
    step("⌘N once more", Verb::Key("cmd-n"), "button", "New Job in Armada"),
    step("⌘T over the sheet does what the card does", Verb::Key("cmd-t"), "textbox", "Message"),
    step("⌘K opens the palette", Verb::Key("cmd-k"), "dialog", "Command palette"),
    step("Typing narrows to New Job here, with ⌘T", Verb::Type("job here"), "searchbox", "Search actions, jobs and settings"),
    step("New Job here stays, the typed letters lit", Verb::Look, "option", "New Job here"),
    step("Enter starts one the same way, on its thread with the keys in its box", Verb::Key("enter"), "textbox", "Message"),
];
