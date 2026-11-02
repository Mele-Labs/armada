// Where a Judge's notes are. The Job's Judge leaves notes on plan tasks and diff lines, and its box and the Ledger tab lead to all of them. The document has a Judge in
// its head, and its notes stand on the page under the words they name. Over the `judge-on-demand` scenario.

import { button, deckRow, dialog, inside, region, role, walk } from "../walk";

const pane = (name: string) => region(`Pane: ${name}`, { exact: true });
const popover = dialog("Judge");
const JOB = "Switch sign-in to session tokens";
const DOC = "Session tokens";
const finding = (claim: RegExp) => role("group", claim);

const judgeNotesFound = walk("judge-on-demand", [
  { press: deckRow(JOB), say: "A Job. Its flight controls hold the Judge that reads the whole Job" },
  { press: button("Close Ledger"), say: "With the Ledger tab closed, nothing shows the notes yet" },
  { press: button("Judge", { exact: true }), say: "Judge the Job" },
  { press: inside(popover, button("Judge", { exact: true })), say: "It reads for two seconds" },
  { look: role("img", "Notes to read"), say: "The dot says it has notes. These name a plan task and a diff line, the kind Bridge used to drop" },
  { press: button("Judge", { exact: true }), say: "The same press now leads to them" },
  { press: inside(popover, button("Read the notes")), say: "Read the notes" },
  { look: region(`Notes on ${JOB}`), say: "Every note of the reading, at the head of the Ledger, each saying where it is" },
  { press: inside(pane("Ledger"), button(DOC, { exact: true })), say: "Now the document" },
  { press: button("Judge this document"), say: "A Judge in the document's head, also on J" },
  { press: inside(popover, button("Judge", { exact: true })), say: "It reads the page" },
  { look: finding(/^Finding: The write-up says a token lasts a day/), say: "A note under the paragraph it names" },
  { look: finding(/^Finding: The heading says how a token lives/), say: "A heading can hold one too" },
  { look: finding(/^Finding: The write-up names no owner/), say: "A note about the whole document, or one whose words the page no longer has, stands at the head" },
  { key: "Escape", on: region("Document", { exact: true }), say: "Back to the Job" },
  { press: inside(pane("Thread"), button(`Judged ${DOC}`, { exact: true })), say: "The line in the thread opens the page at its notes" },
  { look: finding(/^Finding: The heading says how a token lives/), say: "Here they are" },
]);

export { judgeNotesFound as "judge-notes-found" };
