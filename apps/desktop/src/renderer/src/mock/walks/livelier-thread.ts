// A Job's thread, roomier and livelier, with less to type: room between messages, his words as a bubble, a run that ticks and
// lands, the quick bar and its keys, Talk, the Aa panel and System › Thread, and a Job lifting off as it is dispatched. Over the
// `livelier-thread` scenario.

import { deckRow, button, dialog, inside, place, region, role, systemPage, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const thread = region("Thread");
const message = role("textbox", "Message");

const livelier = walk("livelier-thread", [
  toSessions,
  { press: deckRow("Store clock fix"), say: "A Job's thread, with more room between messages" },
  { look: inside(thread, region("Your message")), say: "Your words sit right, in a quiet bubble" },
  { later: inside(thread, role("timer")), say: "A run still going: its glyph moves and its time ticks" },
  { look: inside(thread, text(/212 passed/)), say: "It ended: the glyph settles and its last lines slide in" },
  { key: "Meta+1", on: message, say: "The quick bar answers the question. ⌘1 sends Restart it" },
  { press: button("Talk", { exact: true }), say: "Talk, or hold ⌥Space in the box: the words fill it, and nothing sends" },
  { press: button("Thread look"), say: "Aa: how this thread reads" },
  { type: "Xavier", into: inside(dialog("Thread look"), role("textbox", "Agent name")), say: "The agent's name, live in the thread" },
  { press: place("System"), say: "System" },
  { press: systemPage("Thread"), say: "System › Thread: the same settings" },
  { later: role("combobox", "Voice"), say: "Voice. Whisper shows its Install here" },
  { press: place("Cockpit"), say: "A Job just dispatched lifts off into the list" },
  { press: deckRow("Quiet the slot logs"), say: "Its card lifts off as it opens" },
]);

export { livelier as "livelier-thread" };
