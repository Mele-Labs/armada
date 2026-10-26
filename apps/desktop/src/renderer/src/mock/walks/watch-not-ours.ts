// A QA deploy watch whose only failure was a part the agent set aside ends not ours, never passed. Its mark is its own and never green, named
// with what failed; the side panel keeps the real failed part, struck through; and Fleet asks the person in Needs you, where Escalate gives
// lines to copy for the owners and Fix it asks the agent. Over `watch-not-ours`.

import { button, deckRow, dialog, inside, region, role, tab, text, walk } from "../walk";

const landing = region("Landing", { exact: true });
const watchingBlock = inside(landing, region("Watching"));
const panel = dialog("Watch QA deploy");
const needs = region("Needs you");
const card = inside(needs, role("group", "Not ours: QA deploy, provision failed"));

const watchNotOurs = walk("watch-not-ours", [
  { press: deckRow("Ship the search filters to QA"), say: "A Job whose QA deploy ended" },
  { look: inside(watchingBlock, button("Watch QA deploy", { exact: true })), say: "Ended, and still listed without Show ended while Needs you asks about it" },
  { hover: inside(watchingBlock, role("img", "Not ours: provision failed")), say: "Its mark is not ours, never green, named with what failed" },
  { press: inside(watchingBlock, button("Watch QA deploy", { exact: true })), say: "The watch opens in the side panel" },
  { press: inside(panel, tab("Parts")), say: "Its parts" },
  { look: inside(panel, role("list", "Parts")), say: "provision keeps its failed state, struck through with the sentence that set it aside" },
  { key: "Escape", on: panel, say: "Esc closes it" },
  { press: inside(needs, button("Needs you", { exact: true })), say: "Fleet put it in Needs you, alone there, so it opens at once" },
  { look: card, say: "The watch and what failed, the agent's sentence, the failing run, and Fix it, Ignore and Escalate" },
  { press: inside(card, button("Escalate")), say: "Escalate" },
  { look: inside(card, region("Summary for the owners")), say: "Lines to copy for the owners of what broke, with one copy control. Nothing is sent" },
  { press: inside(card, button("Fix it")), say: "Fix it" },
  { look: text(/^Fix provision in QA deploy/), say: "The agent is asked to fix provision, and the item is settled" },
  { look: inside(watchingBlock, role("switch", "Show ended")), say: "Settled, the watch hides as every ended one does, until Show ended" },
]);

export { watchNotOurs };
