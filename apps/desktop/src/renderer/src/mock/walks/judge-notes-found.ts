// Where a Judge's results are. A Judge of the whole Job has nothing to stand in, so its press opens a Judge tab with a box of its own: what was asked, the shape of the reading, then its notes. A Judge on an
// artifact shows its results in that artifact: under a sketch's row in the Ledger, under the words a document's note names. Over the `judge-on-demand` scenario.

import { FAILS } from "../judge-fleet";
import { button, deckRow, dialog, inside, region, role, walk } from "../walk";

const pane = (name: string) => region(`Pane: ${name}`, { exact: true });
const popover = dialog("Judge");
const JOB = "Switch sign-in to session tokens";
const DOC = "Session tokens";
const SKETCH = "Sign-in flow";
const tabBox = inside(pane("Judge"), role("textbox", "Verify that"));
const tabRun = inside(pane("Judge"), button("Judge", { exact: true }));
const finding = (claim: RegExp) => role("group", claim);

const judgeNotesFound = walk("judge-on-demand", [
  { press: deckRow(JOB), say: "A Job" },
  { press: button("Judge", { exact: true }), say: "The Judge in its head reads the whole Job" },
  { look: tabBox, say: "No popover. A Judge tab opens with its own box, in its own hue, and what judging is above it" },
  { type: "existing sign-ins survive the switch", into: tabBox, say: "Say what to verify, or leave it empty" },
  { press: tabRun, say: "Run it" },
  { look: role("status", "Judge reading"), say: "What was asked, and the shape of the reading" },
  { look: finding(/^Finding: No task says what happens/), say: "Its results land above the box, each note saying where it is" },
  { press: button("Close Judge"), say: "Close the tab" },
  { press: inside(pane("Thread"), button(`Judged ${JOB}`, { exact: true })), say: "The line in the thread opens it again" },
  { look: finding(/^Finding: No task says what happens/), say: "Same results" },
  { type: FAILS, into: tabBox, say: "Ask again from the same box. This reading will fail" },
  { press: tabRun, say: "Run it" },
  { look: role("text", /^The judge stopped/), say: "The tab says why" },
  { press: button("Close Judge"), say: "Now a Judge on one artifact" },
  { press: inside(pane("Ledger"), button(`Judge ${SKETCH}`)), say: "The Judge on a sketch's row" },
  { press: inside(popover, button("Judge", { exact: true })), say: "Run it. No tab opens" },
  { look: finding(/^Finding: The sketch has the token refreshed/), say: "Its note stands under its own row" },
  { press: inside(pane("Ledger"), button(DOC, { exact: true })), say: "A document" },
  { press: button("Judge this document"), say: "The Judge in its head, also on J" },
  { press: inside(popover, button("Judge", { exact: true })), say: "It reads the page" },
  { look: finding(/^Finding: The write-up says a token lasts a day/), say: "A note under the paragraph it names" },
  { look: finding(/^Finding: The write-up names no owner/), say: "A note on the whole document stands at the head of the page" },
]);

export { judgeNotesFound as "judge-notes-found" };
