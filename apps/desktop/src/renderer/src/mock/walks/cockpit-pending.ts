// What the Cockpit shows while an answer is out to Fleet, and the tile of a Job whose workflow is still
// being settled. Over `cockpit-pending`, where Fleet is slow to answer and slower to stop carrying what
// it answered.

import { inside, region, role, tab, text, walk } from "../walk";

const cockpit = region("Dashboard");
const glass = role("listbox", "Tiles");
const question = region(/^Session question/);

const cockpitPending = walk("cockpit-pending", [
  { look: tab("Your move"), say: "Your move: a Session has asked two questions, and nothing else needs him" },
  { look: question, say: "The first question in front, the second stacked behind it" },
  { key: "1", on: cockpit, say: "A number picks an answer" },
  { key: "Enter", on: cockpit, say: "Enter sends it to Fleet" },
  { look: inside(question, role("radio", /A fake the test sets/)), say: "From the press until Fleet answers, the answer pressed carries the loop along its edge, with a tooltip. Nothing else on the card can be pressed, and Send is held" },
  { hover: inside(question, role("radio", /A fake the test sets/)), say: "The tooltip names it: waiting on Fleet" },
  { look: inside(question, role("radio", /Keep three/)), say: "Fleet answers and the card leaves, and the next question comes forward. Fleet still carries the first on the Session for a moment longer, and it does not come back to the deck" },
  { key: "2", on: cockpit, say: "Pick an answer to the second question" },
  { key: "Enter", on: cockpit, say: "Send it" },
  { look: inside(question, role("radio", /Raise it to five/)), say: "The loop again" },
  { look: text("Session Pin the store clock is already working on that"), say: "Fleet refuses. The loop stops, the card stays as it was, and the toast says Fleet's words" },
  { look: inside(question, role("radio", /Raise it to five/)), say: "Nothing is held now: the answer can be pressed again" },
  { key: "Escape", on: cockpit, say: "Esc puts the card off, and the Active filter shows the rest" },
  { key: "]", on: cockpit, say: "] steps to Active" },
  { look: inside(glass, role("img", "Workflow, still being settled")), say: "A Job whose workflow is still being settled holds the steps' place with a blinking caret. No step stands in for it" },
  { hover: inside(glass, role("img", "Workflow, still being settled")), say: "The tooltip names it" },
  { later: cockpit, say: "The proposer settles the workflow" },
  { look: inside(glass, role("list", "Steps")), say: "The caret goes, and the steps come onto the tile" },
]);

export { cockpitPending as "cockpit-pending" };
