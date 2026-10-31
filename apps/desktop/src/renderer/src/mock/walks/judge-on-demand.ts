// A judge on demand: Judge on a Ledger row asks what to verify, reads for two seconds, marks the row while its notes are unread, leaves a line in the thread, and its notes
// stand in the pull request's Files tab on the line they name, with Send to Job. Over the `judge-on-demand` scenario.

import { button, deckRow, dialog, inside, region, role, walk } from "../walk";

const pane = (name: string) => region(`Pane: ${name}`, { exact: true });
const ledger = pane("Ledger");
const popover = dialog("Judge");
const note = role("group", /^Finding: Existing sign-ins do not survive/);

const judgeOnDemand = walk("judge-on-demand", [
  { press: deckRow("Switch sign-in to session tokens"), say: "The Job is open. Its Ledger lists what it made, and every row has a Judge at its end" },
  { press: inside(ledger, button("Judge #2418", { exact: true })), say: "Judge on the pull request opens a small box. Nothing is read until it is pressed" },
  { type: "existing sign-ins survive the switch", into: inside(popover, role("textbox", "Verify that")), say: "What to verify, in a line. Left empty it asks what was missed" },
  { press: inside(popover, button("Judge", { exact: true })), say: "Enter does the same" },
  { look: inside(ledger, role("img", "Judge reading")), say: "The pressed Judge pulses while it reads" },
  { look: inside(ledger, role("img", "Notes to read")), say: "A dot on the row's Judge while its notes are unread" },
  { press: inside(pane("Thread"), button("Judged #2418", { exact: true })), say: "A line in the thread says the judge finished. Pressing it opens the pull request at its notes" },
  { look: note, say: "A note stands under the line of the diff it names: You should know, the claim and where it is" },
  { press: inside(note, button("Send to Job")), say: "Send to Job gives it to the Job as your message and folds the note to one line" },
]);

export { judgeOnDemand as "judge-on-demand" };
