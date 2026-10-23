// Armada fixes, under the Logbook's entries (`docs/concepts/retro.md`): what got in the way across Sessions, newest first. Each
// item reads as one arrow from whose way it got in to where the fix lands, each end its own hue, then
// a headline, what happened, what would change and two answers. The walk expands two items' evidence
// in place, opens a Session's retro, then rejects an Armada item, accepts a Kit item and reads the
// accepted ones. `retro-page.test.tsx` holds the claims.

import { button, dialog, inside, role, tab, text, toSystemPage, walk } from "../walk";

const RETROS = role("list", "Retros");
const RETRO = dialog("Retro");
const item = (words: string) => role("listitem", words);
const RESET = item("A reset waited 40 minutes for an answer");
const SEARCH = item("A search of the whole disk set off privacy prompts");
const COMMIT = item("A commit message was read as a redirect");
const WALKS = item("Walks opened in your browser");

export const retroFromTheRetrosPage = walk("retro/lessons", [
  ...toSystemPage("Retros", "The Logbook, with Armada fixes under the entries"),
  { look: RETROS, say: "Newest first, across Jobs" },
  { look: inside(RESET, text(/^Agent$/)), say: "Whose way it got in, in a word and a hue" },
  { look: inside(RESET, text(/^Armada$/)), say: "Where the fix lands, in a word and a hue" },
  { look: inside(WALKS, text(/^Kit$/)), say: "Kit has its own hue" },
  { look: inside(RESET, text(/Tell you when an ask has waited ten minutes/)), say: "What would change, set apart" },
  { hover: inside(RESET, role("img", "Agent")), say: "The marks keep their tooltips" },
  { press: inside(RESET, button("Evidence")), say: "Evidence on the list reads the Job's retro once" },
  { look: inside(RESET, text(/Allow git reset --hard origin\/main\?/)), say: "The rows it cites open under the card" },
  { press: inside(SEARCH, button("Evidence")), say: "Another item of that Job shares the read" },
  { look: inside(SEARCH, text(/Operation not permitted/)), say: "Its rows open in place too" },
  { press: inside(RESET, button(/^Retire guides 8 and 20/)), say: "The Job label opens its retro" },
  { press: inside(inside(RETRO, RESET), button("Evidence")), say: "The record rows, behind a control" },
  { look: inside(RETRO, text(/Allow git reset --hard origin\/main\?/)), say: "The record row it cites" },
  { press: inside(RETRO, button("Close")), say: "Back to Armada fixes" },
  { hover: inside(RESET, button("Start Job", { exact: true })), say: "Start Job on an Armada item" },
  { hover: inside(COMMIT, button("Reject change")), say: "Reject change" },
  { press: inside(COMMIT, button("Reject change")), say: "Reject change discards it" },
  { hover: inside(WALKS, button("Accept", { exact: true })), say: "Accept on a Kit item" },
  { press: inside(WALKS, button("Accept", { exact: true })), say: "Accept saves it" },
  { press: tab("Accepted"), say: "Accepted: the saved Kit items" },
  { look: WALKS, say: "The Kit item, saved" },
  { press: tab("Open"), say: "Back to Open" },
]);
