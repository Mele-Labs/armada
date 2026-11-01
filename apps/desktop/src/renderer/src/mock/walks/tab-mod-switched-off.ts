// A tab mod switched off empties its tab, which says so, and switched on draws again. Over the `job-layouts` scenario: a Custom tab built, then the Mod Job that built it, whose
// thread holds the mod's switch beside a pane showing the tab.

import { button, deckRow, inside, region, role, tab, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const pane = (name: string) => region(`Pane: ${name}`, { exact: true });
const empty = region("Empty pane", { exact: true });
const item = (name: string | RegExp) => role("menuitem", name);
const thread = inside(pane("Thread"), tab("Thread", { exact: true }));

const tabModSwitchedOff = walk("job-layouts", [
  toSessions,
  { press: deckRow("Retry the slot lease"), say: "A Job" },
  { key: "Meta+d", on: thread, say: "⌘D" },
  { press: inside(empty, button("Add tab")), say: "+" },
  { press: item("Custom"), say: "Custom" },
  { type: "Pull requests waiting on me", into: inside(pane("Custom"), role("textbox", "Custom")), say: "Describe the tab" },
  { press: inside(pane("Custom"), button("Send")), say: "Send" },
  { later: inside(pane("Review queue"), tab("Review queue", { exact: true })), say: "The Mod Job builds it" },
  { press: deckRow(/^Mod Pull requests/), say: "The Mod Job that built it" },
  { key: "Meta+d", on: thread, say: "⌘D" },
  { press: inside(empty, button("Add tab")), say: "+" },
  { press: item("Review queue"), say: "The tab, by name" },
  { look: inside(pane("Review queue"), button(/^Pull request #812/)), say: "The tab draws" },
  { press: role("switch", "Review-queue"), say: "The Job's thread holds the mod's switch: off" },
  { look: inside(pane("Review queue"), text("Switched off")), say: "The tab is empty and says why" },
  { press: role("switch", "Review-queue"), say: "On" },
  { look: inside(pane("Review queue"), button(/^Pull request #812/)), say: "It draws again" },
]);

export { tabModSwitchedOff };
