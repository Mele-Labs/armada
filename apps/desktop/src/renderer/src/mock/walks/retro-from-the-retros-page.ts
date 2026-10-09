// The Retros page (`docs/concepts/retro.md`): what got in the way across Jobs,
// newest first. Each item reads as one arrow from whose way it got in to where
// the fix lands, each end its own hue, then a headline, what happened, what
// would change and two answers. The walk expands two items' evidence in place,
// opens Job 3's retro, then creates a Job from an Armada item, updates Kit
// from a Kit item, rejects a Manifest item and reads the accepted ones. `retro.test.tsx` holds the claims.

import { button, dialog, inside, role, tab, text, walk } from "../walk";

const RETROS = role("list", "Retros");
const RETRO = dialog("Retro");
const item = (words: string) => role("listitem", words);
const STALE_MAIN = item("The gate blamed the Drone for Fleet's own mistake");
const GREP = item("A Drone had to wait for grep to be allowed");
const DOCS = item("A docs edit ran every Rust test");

export const retroFromTheRetrosPage = walk("retro/lessons", [
  { press: button("Retros", { exact: true }), say: "Retros, under Work" },
  { look: RETROS, say: "Newest first, across Jobs" },
  { look: inside(STALE_MAIN, text(/^Fleet$/)), say: "Whose way it got in, in a word and a hue" },
  { look: inside(STALE_MAIN, text(/^Armada$/)), say: "Where the fix lands, in a word and a hue" },
  { look: inside(GREP, text(/^Kit$/)), say: "Kit has its own hue" },
  { look: inside(DOCS, text(/^Manifest$/)), say: "So does Manifest" },
  { look: inside(STALE_MAIN, text(/Compare against origin\/main/)), say: "What would change, set apart" },
  { hover: inside(STALE_MAIN, role("img", "Fleet")), say: "The marks keep their tooltips" },
  { press: inside(STALE_MAIN, button("Evidence")), say: "Evidence on the list reads Job 3's retro once" },
  { look: inside(STALE_MAIN, text(/4e1c2a9 on main/)), say: "The rows it cites open under the card" },
  { press: inside(GREP, button("Evidence")), say: "Another item of Job 3 shares that read" },
  { look: inside(GREP, text(/grep -n timed/)), say: "Its rows open in place too" },
  { press: inside(STALE_MAIN, button("Job 3")), say: "The Job label opens its retro" },
  { press: inside(inside(RETRO, STALE_MAIN), button("Evidence")), say: "The record rows, behind a control" },
  { look: inside(RETRO, text(/4e1c2a9 on main/)), say: "The record row it cites" },
  { look: inside(RETRO, role("region", "Notes")), say: "Notes left with Job 3 open" },
  { press: inside(RETRO, button("Close")), say: "Back to Retros" },
  { hover: inside(STALE_MAIN, button("Create Job", { exact: true })), say: "Create Job on an Armada item" },
  { press: inside(STALE_MAIN, button("Create Job", { exact: true })), say: "It proposes a Job" },
  { look: inside(STALE_MAIN, button("Proposed Job")), say: "The item keeps a link to it" },
  { press: tab("Kit"), say: "Kit: the tool set" },
  { hover: inside(GREP, button("Update Kit", { exact: true })), say: "Update Kit on a Kit item with a command" },
  { press: inside(GREP, button("Update Kit", { exact: true })), say: "Update Kit adds the command and saves it" },
  { press: tab("Manifest"), say: "Manifest: the repository's Checks, tests and code" },
  { hover: inside(DOCS, button("Reject change")), say: "Reject change" },
  { press: inside(DOCS, button("Reject change")), say: "Reject change discards it" },
  { press: tab("Kit"), say: "Kit again, where the saved item will be" },
  { press: tab("Accepted"), say: "Accepted: the saved Kit items" },
  { look: GREP, say: "The Kit item, saved" },
  { press: tab("Open"), say: "Back to Open" },
]);
