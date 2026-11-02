// The Logbook's Retros (`docs/concepts/retro.md`): what got in the way across Sessions, newest first, a line each. The open
// item has its title and answers in a head, what happened and what would change in its card, and whose way it got in and where the
// fix lands in its Record. The walk expands two items' evidence in place, opens a Session's retro, then rejects an Armada
// item, accepts a Kit item and reads the accepted ones. `retro-page.test.tsx` holds the claims.

import { button, dialog, inside, region, role, row, text, toSystemPage, walk } from "../walk";

const RETROS = role("grid", "Retros");
const ITEM = region("Retro item");
const RETRO = dialog("Retro");
const line = (words: string) => inside(RETROS, row(new RegExp(words)));
const RESET = line("A reset waited 40 minutes for an answer");
const SEARCH = line("A search of the whole disk set off privacy prompts");
const COMMIT = line("A commit message was read as a redirect");
const WALKS = line("Walks opened in your browser");

export const retroFromTheRetrosPage = walk("retro/lessons", [
  ...toSystemPage("Retros", "The Logbook's Retros"),
  { look: RETROS, say: "Newest first, across Jobs" },
  { press: RESET, say: "An item opens beside the log" },
  { look: inside(ITEM, text(/^Agent$/)), say: "Whose way it got in, in the Record" },
  { look: inside(ITEM, text(/^Armada$/)), say: "Where the fix lands, in the Record" },
  { press: WALKS, say: "Another item" },
  { look: inside(ITEM, text(/^Kit$/)), say: "A Kit item" },
  { press: RESET, say: "Back to the first" },
  { look: inside(ITEM, text(/Tell you when an ask has waited ten minutes/)), say: "What would change, set apart" },
  { hover: inside(RESET, role("img", "Agent")), say: "In the log, the marks keep their tooltips" },
  { press: inside(ITEM, button("Evidence")), say: "Evidence on the item reads the Job's retro once" },
  { look: inside(ITEM, text(/Allow git reset --hard origin\/main\?/)), say: "The rows it cites open under the card" },
  { press: SEARCH, say: "Another item of that Job" },
  { press: inside(ITEM, button("Evidence")), say: "It shares the read" },
  { look: inside(ITEM, text(/Operation not permitted/)), say: "Its rows open in place too" },
  { press: RESET, say: "Back to the first" },
  { press: inside(ITEM, button(/^Retire guides 8 and 20/)), say: "The Job button opens its retro" },
  { press: inside(inside(RETRO, role("listitem", "A reset waited 40 minutes for an answer")), button("Evidence")), say: "The record rows, behind a control" },
  { look: inside(RETRO, text(/Allow git reset --hard origin\/main\?/)), say: "The record row it cites" },
  { press: inside(RETRO, button("Close")), say: "Back to Retros" },
  { hover: inside(ITEM, button("Start Job", { exact: true })), say: "Start Job on an Armada item" },
  { press: COMMIT, say: "Another Armada item" },
  { hover: inside(ITEM, button("Reject change")), say: "Reject change" },
  { press: inside(ITEM, button("Reject change")), say: "Reject change discards it" },
  { press: WALKS, say: "A Kit item" },
  { hover: inside(ITEM, button("Accept", { exact: true })), say: "Accept on a Kit item" },
  { press: inside(ITEM, button("Accept", { exact: true })), say: "Accept saves it" },
  { press: button("Show"), say: "Open and Accepted are the Show menu" },
  { press: role("menuitem", "Accepted", { exact: true }), say: "Accepted: the saved Kit items" },
  { look: WALKS, say: "The Kit item, saved" },
  { press: button("Show"), say: "Show again" },
  { press: role("menuitem", "Open", { exact: true }), say: "Back to Open" },
]);
