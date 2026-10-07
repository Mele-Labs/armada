// A Session in a narrow window: below the Helm dock's own breakpoint
// (`--layout-breakpoint`, 1100px) the ledger folds into a sheet, the
// conversation takes the width, and the message box's selects are a glyph and
// a value. The owner: "It kind of sucks on a more narrow screen", 7 Oct 2026.

import { button, inside, role, region, text, walk } from "../walk";

const sessions = region("Sessions");
const thread = region("Thread");
const message = role("textbox", "Message");
const ledgerSheet = role("dialog", "Attachments");
const owner = role("group", "Owned by Flaky store test");
const rail = (name: string) => role("button", name, { exact: true });

export const sessionsNarrow = walk(
  "sessions",
  [
    { press: inside(sessions, button("New Session")), say: "A new Session, in a window below the breakpoint" },
    { look: message, say: "The conversation has the whole width" },
    { look: role("combobox", "Permission mode"), say: "The message box is one row: each select a glyph and its value" },
    { hover: role("combobox", "Model"), say: "What each sets is on hover" },
    { look: button("Attachments"), say: "The ledger is a button in the header" },
    { press: button("Attachments"), say: "It opens as a sheet" },
    { look: inside(ledgerSheet, text("Worktree Slot")), say: "Every section is there, dim, and empty" },
    { press: inside(ledgerSheet, button("Close")), say: "Back to the conversation" },
    { type: "Fix the flaky store test.", into: message, say: "A message" },
    { press: button("Send"), say: "The agent reads first" },
    { look: region("Leased on first write"), say: "The first write, in the thread" },
    { press: button("Attachments"), say: "The ledger took the slot" },
    { press: button("Open Worktree slot 3"), say: "Its row closes the sheet and opens what it names" },
    { look: role("dialog", "slot-3"), say: "The worktree panel, over the Session" },
    { press: inside(role("dialog", "slot-3"), button("Close")), say: "Back to the Session" },
    { later: thread, say: "The agent carries on" },
    { later: thread, say: "And opens a pull request" },
    { press: rail("Overview"), say: "Overview, at the same width" },
    { type: "#1843", into: role("searchbox", "Search Sessions"), say: "A pull request number" },
    { look: inside(sessions, role("listitem", "Flaky store test")), say: "The row wraps its slot, pull requests and Jobs inside it" },
    { press: rail("Cleanup"), say: "Cleanup" },
    { hover: button("Branch fix/52-pin-store-clock"), say: "A chip names its owner" },
    { press: button("Branch fix/52-pin-store-clock"), say: "A press keeps the card up" },
    { look: owner, say: "The card stays inside the window" },
  ],
  { width: 900, height: 900 },
);

export { sessionsNarrow as "sessions-narrow" };
