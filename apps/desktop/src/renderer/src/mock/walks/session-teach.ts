// Teaching Armada from words in a message. Selecting them offers Teach beside Reply and Fork, on the bar and in a menu a right
// press opens. The owner says what the words are, Armada shows back what it understood, and Save keeps it. Part one teaches a
// Jira key as a link, from the bar, and ends in Settings › Links. Part two teaches a habit for agents, from the menu and the keyboard.

import { kit, toSessions } from "../sessions/walk-kit";
import { IDEA_MESSAGE, KEY_MESSAGE } from "../sessions/teach";
import { button, deckRow, dialog, inside, role, text, toSystemPage, walk } from "../walk";
import type { Step } from "../walk";

const JOB = "Pin the clock in the store";
const KEY_WORDS = "STORE-1234";

function steps(): Step[] {
  const { thread } = kit(false);
  const bar = role("toolbar", "Selection");
  const panel = dialog("Teach");
  const instruction = inside(panel, role("textbox", "Instruction"));
  const key = inside(thread, text(KEY_MESSAGE.slice(0, 40)));
  const idea = inside(thread, text(IDEA_MESSAGE.slice(0, 40)));
  const menuItem = (name: string) => role("menuitem", name);
  return [
    toSessions,
    { press: deckRow(JOB), say: "A Job opens" },

    // Part one: a Jira key, from the bar.
    { look: key, say: "Its agent writes a Jira key, as plain words" },
    { select: KEY_WORDS, in: key, say: "The key, selected" },
    { look: bar, say: "The bar over the selection has a third act" },
    { hover: inside(bar, button("Teach")), say: "Teach, named on hover" },
    { press: inside(bar, button("Teach")), say: "Teach opens a panel beside the thread, which stays in view" },
    { look: inside(panel, text(KEY_WORDS)), say: "The selected words, as they stand in the message" },
    { type: "A Jira key. Make it a link: https://myproject.jira.com/STORE-1234", into: instruction, say: "What it is and what to do, in his own words" },
    { look: inside(panel, button(/^armada$/)), say: "It starts on this repository. One press on the chip makes it every repository" },
    { press: inside(panel, button("Teach", { exact: true })), say: "Teach sends the words to be read" },
    { look: inside(panel, role("region", "Link")), say: "What Armada understood: the example, and the address it goes to" },
    { look: inside(panel, role("region", "Agents")), say: "And the note agents are given" },
    { look: inside(thread, role("link", "STORE-1250")), say: "Behind the panel, every key in the thread is already a link, as a preview" },
    { look: inside(thread, text("STORE-1234-pin-clock")), say: "A name in code stays as it was" },
    { press: inside(panel, button("Change")), say: "Change goes back to the box" },
    { look: instruction, say: "His words are kept" },
    { key: "Meta+Enter", on: instruction, say: "⌘↵ teaches again, from the keyboard" },
    { look: inside(panel, role("region", "Link")), say: "The same understanding" },
    { press: inside(panel, button("Save")), say: "Save keeps it, and closes the panel" },
    { look: inside(thread, role("link", "STORE-1234")), say: "The links stay in the thread" },
    { hover: inside(thread, role("link", "STORE-1250")), say: "Each names where it goes" },
    ...toSystemPage("Links", "Settings › Links lists what he taught"),
    { look: role("group", "STORE-1234"), say: "The rule, where it applies, and Forget" },
    { look: button("Forget STORE-1234"), say: "Forget takes it out of every Job at once" },

    // Part two: a habit for agents, from the right-click menu and the keyboard.
    toSessions,
    { press: deckRow(JOB), say: "Back in the Job" },
    { look: idea, say: "A later message proposes an idea in prose" },
    { select: IDEA_MESSAGE, in: idea, say: "The paragraph, selected" },
    { context: idea, say: "A right press on the selected words" },
    { look: menuItem("Teach"), say: "A menu with Teach first, then Reply and Fork from here" },
    { key: "ArrowDown", on: menuItem("Teach"), say: "↓ moves down the menu" },
    { key: "ArrowUp", on: menuItem("Reply"), say: "↑ moves back" },
    { key: "Escape", on: menuItem("Teach"), say: "Esc closes the menu and leaves the words selected" },
    { key: "Shift+F10", on: idea, say: "Shift+F10, or the context menu key, opens it from the selection" },
    { press: menuItem("Teach"), say: "Teach, from the menu" },
    { type: "Whenever you propose an idea like this, use a sketch and a short markdown list", into: instruction, say: "A habit, not a link" },
    { press: inside(panel, button(/^armada$/)), say: "The chip, once" },
    { look: inside(panel, button("Every repository")), say: "Now for every repository" },
    { key: "Meta+Enter", on: instruction, say: "⌘↵ teaches" },
    { look: inside(panel, role("region", "Agents")), say: "Only the note agents are given. There is no link to draw" },
    { press: inside(panel, button("Save")), say: "Save" },
  ];
}

const teach = walk("session-teach", steps());

export { teach as "session-teach" };
