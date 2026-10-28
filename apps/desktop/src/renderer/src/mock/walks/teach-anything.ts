// Teaching Armada about anything that happens in a Job. Teach is pressed on selected words, a command, a Fleet notice, a Check and a
// plan task. The owner says what it is and what to do, Armada shows back what it understood as a section for each of Agents, Bridge and
// Fleet that the teaching changes, and Save keeps it. A Jira key is taught once and each repository's address is found by Armada itself:
// from the repository's settings, from its pull requests, or by asking once. Everything taught ends on the Logbook page. Over the
// `teach-anything` scenario.

import { kit, toSessions } from "../sessions/walk-kit";
import { ARMADA_TITLE, INSTALL, KEY_MESSAGE, LEDGER_MESSAGE, LEDGER_TITLE, PAYMENTS_MESSAGE, PAYMENTS_TITLE, TASK } from "../sessions/teach";
import { button, deckRow, dialog, inside, place, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

const KEY_WORDS = "STORE-1234";

function steps(): Step[] {
  const { thread } = kit(false);
  const bar = role("toolbar", "Selection");
  const panel = dialog("Teach");
  const instruction = inside(panel, role("textbox", "Instruction"));
  const key = inside(thread, text(KEY_MESSAGE.slice(0, 40)));
  const menuItem = (name: string) => role("menuitem", name);
  const section = (name: string) => inside(panel, region(name, { exact: true }));
  const chip = inside(panel, button(/^armada$/));
  const needs = region("Needs you");
  const now = region("Now", { exact: true });
  const calls = inside(thread, button(/Tool calls/));
  const command = inside(thread, text(INSTALL));
  const notice = inside(thread, button(/commits behind main/));
  return [
    // One: a Jira key, in a repository whose settings name its Jira site.
    toSessions,
    { press: deckRow(ARMADA_TITLE), say: "A Job in the first repository" },
    { look: key, say: "Its agent writes a Jira key, as plain words" },
    { select: KEY_WORDS, in: key, say: "The key, selected" },
    { press: inside(bar, button("Teach")), say: "Teach opens a panel beside the thread" },
    { look: section("Words"), say: "What it was pressed on" },
    { type: "A Jira key. Link it to our Jira", into: instruction, say: "What it is and what to do, in his own words" },
    { press: chip, say: "The chip, once, so it holds in every repository" },
    { key: "Meta+Enter", on: instruction, say: "⌘↵ teaches" },
    { look: section("Agents"), say: "The note agents are given" },
    { look: section("Bridge"), say: "How Bridge draws it: the pattern, and the address Armada found for this repository" },
    { look: inside(section("Bridge"), text("Found in this repository's settings")), say: "Where the address came from" },
    { press: inside(panel, button("Save")), say: "Save keeps it, and closes the panel" },
    { look: inside(thread, role("link", KEY_WORDS)), say: "The key is a link" },
    { hover: inside(thread, role("link", KEY_WORDS)), say: "Hover says where its address came from" },

    // Two: another repository, another site, found without being asked.
    toSessions,
    { press: deckRow(PAYMENTS_TITLE), say: "A Job in a second repository" },
    { look: inside(thread, text(PAYMENTS_MESSAGE.slice(0, 30))), say: "Its agent writes a key of its own" },
    { hover: inside(thread, role("link", "PAY-88")), say: "Already a link, to a different site. Armada found it in a pull request" },

    // Three: a repository with no site to find, so Armada asks once.
    toSessions,
    { press: deckRow(LEDGER_TITLE), say: "A Job in a third repository" },
    { look: inside(thread, text(LEDGER_MESSAGE.slice(0, 30))), say: "Its key stays plain" },
    { press: button("Needs you", { exact: true }), say: "Needs you, closed on the composer, opened" },
    { look: inside(needs, button(/Which Jira site does ledger use/)), say: "It holds the one question" },
    { press: inside(needs, button(/Which Jira site does ledger use/)), say: "Opened" },
    { press: inside(needs, text("acme-payments.atlassian.net")), say: "One of the sites it already knows" },
    { press: inside(needs, button("Answer", { exact: true })), say: "Answer" },
    { look: inside(thread, role("link", "LED-210")), say: "The key links" },

    // Four: a command.
    toSessions,
    { press: deckRow(ARMADA_TITLE), say: "Back in the first Job" },
    { press: calls, say: "The tool calls, opened" },
    { context: command, say: "A right press on a command" },
    { look: menuItem("Teach"), say: "A menu with Teach" },
    { press: menuItem("Teach"), say: "Teach" },
    { type: "This repo uses pnpm", into: instruction, say: "A habit for agents" },
    { key: "Meta+Enter", on: instruction, say: "⌘↵ teaches" },
    { look: section("Agents"), say: "Only the note agents are given" },
    { press: inside(panel, button("Save")), say: "Save" },

    // Five: a Fleet notice, taught to Fleet.
    { key: "Shift+F10", on: notice, say: "Shift+F10 opens a notice's menu from the keyboard" },
    { look: menuItem("Teach"), say: "Teach first, then Disable" },
    { key: "ArrowDown", on: menuItem("Teach"), say: "↓ moves down the menu" },
    { key: "ArrowUp", on: menuItem("For job"), say: "↑ moves back" },
    { press: menuItem("Teach"), say: "Teach" },
    { look: section("Notice"), say: "The notice, as Fleet sent it" },
    { type: "Batch these, and don't tell me unless it blocks a merge", into: instruction, say: "What Fleet should do" },
    { key: "Meta+Enter", on: instruction, say: "⌘↵ teaches" },
    { look: section("Fleet"), say: "What Fleet will do, and where" },
    { look: section("Agents"), say: "And what agents are told" },
    { press: inside(panel, button("Save")), say: "Save" },
    { look: inside(thread, button(/Slot behind main/)), say: "Later notices are one row" },
    { press: inside(thread, button(/Slot behind main/)), say: "Opened, a line for each" },

    // Six: a red Check, and a plan task.
    { context: inside(thread, button(/Check desktop_unit, failed/)), say: "A right press on a red Check" },
    { press: menuItem("Teach"), say: "Teach" },
    { type: "flaky, rerun once before digging", into: instruction, say: "Taught for agents" },
    { key: "Meta+Enter", on: instruction, say: "⌘↵ teaches" },
    { look: section("Agents"), say: "Only the note agents are given" },
    { press: inside(panel, button("Save")), say: "Save" },
    { press: inside(now, button("Whole plan")), say: "Now shows the group being worked. The whole plan is a press away" },
    { press: inside(now, button("Show the clock", { exact: true })), say: "A group opens to its tasks" },
    { hover: inside(now, text(TASK)), say: "A plan task shows its glyphs on hover" },
    { press: inside(now, button(`Teach on task ${TASK}`)), say: "Teach is the one with the mortarboard" },
    { type: "always walk before building UI", into: instruction, say: "For every repository" },
    { press: chip, say: "The chip, once" },
    { key: "Meta+Enter", on: instruction, say: "⌘↵ teaches" },
    { look: section("Agents"), say: "Only the note agents are given" },
    { press: inside(panel, button("Save")), say: "Save" },

    // Seven: all of it, on the Logbook page.
    { press: place("Logbook"), say: "Everything taught is on the Logbook page" },
    { key: "Enter", on: role("grid", "Entries"), say: "Enter opens an entry" },
    { look: inside(region("Entry"), text("desktop_unit failed")), say: "What Teach was pressed on, and the Job, as its evidence" },
    { key: "ArrowDown", on: role("grid", "Entries"), say: "↓ moves down" },
    { look: inside(region("Entry"), role("group", "Also changes")), say: "A notice also changed what Fleet does" },
    { press: inside(role("grid", "Entries"), text(/Tickets are Jira keys/)), say: "The Jira key entry" },
    { look: inside(region("Entry"), role("group", "Also changes")), say: "It also changed how Bridge draws keys, in every repository" },
    { key: "f", on: role("grid", "Entries"), say: "F forgets it, after asking" },
    { press: inside(dialog("Forget this entry"), button("Forget", { exact: true })), say: "Forget takes back all of it" },
    toSessions,
    { press: deckRow(PAYMENTS_TITLE), say: "The second repository's key" },
    { look: inside(thread, text(/PAY-88 asks for it/)), say: "Plain again" },
  ];
}

const teach = walk("teach-anything", steps());

export { teach as "teach-anything" };
