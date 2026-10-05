// The Lessons page (`docs/concepts/retro.md`): what got in the way across Jobs,
// newest first. Each item reads as words for whose way and where the fix lands,
// a headline, what happened, the fix and two answers. The walk opens Job 3's
// retro first, then agrees with an Armada item, saves a Kit item, discards a
// Manifest item and reads the saved ones. `retro.test.tsx` holds the claims.

import { button, dialog, inside, role, tab, text, walk } from "../walk";

const LESSONS = role("list", "Lessons");
const RETRO = dialog("Retro");
const item = (words: string) => role("listitem", words);
const STALE_MAIN = item("The gate blamed the Drone for Fleet's own mistake");
const GREP = item("A Drone had to wait for grep to be allowed");
const DOCS = item("A docs edit ran every Rust test");

export const retroFromTheLessonsPage = walk("retro/lessons", [
  { press: button("Lessons", { exact: true }), say: "Lessons, under Work" },
  { look: LESSONS, say: "Newest first, across Jobs" },
  { look: inside(STALE_MAIN, text(/^Fleet$/)), say: "Whose way it got in, in a word" },
  { look: inside(STALE_MAIN, text(/^Armada$/)), say: "Where the fix lands, in a word" },
  { look: inside(STALE_MAIN, text(/Compare against origin\/main/)), say: "The fix, set apart" },
  { hover: inside(STALE_MAIN, role("img", "Fleet")), say: "The marks keep their tooltips" },
  { press: inside(STALE_MAIN, button("Job 3")), say: "The Job label opens its retro" },
  { press: inside(STALE_MAIN, button("Evidence")), say: "The record rows, behind a control" },
  { look: inside(RETRO, text(/4e1c2a9 on main/)), say: "The record row it cites" },
  { look: inside(RETRO, role("region", "Notes")), say: "Notes left with Job 3 open" },
  { press: inside(RETRO, button("Close")), say: "Back to Lessons" },
  { hover: inside(STALE_MAIN, button("Agree", { exact: true })), say: "Agree on an Armada item" },
  { press: inside(STALE_MAIN, button("Agree", { exact: true })), say: "Agree proposes a Job" },
  { look: inside(STALE_MAIN, button("Proposed Job")), say: "The item keeps a link to it" },
  { press: tab("Kit"), say: "Kit: the tool set" },
  { hover: inside(GREP, button("Agree", { exact: true })), say: "Agree on a Kit item" },
  { press: inside(GREP, button("Agree", { exact: true })), say: "Agree saves it" },
  { press: tab("Manifest"), say: "Manifest: the repository's Checks, tests and code" },
  { hover: inside(DOCS, button("Disagree")), say: "Disagree" },
  { press: inside(DOCS, button("Disagree")), say: "Disagree discards it" },
  { press: tab("All"), say: "All, with the items from before" },
  { press: tab("Accepted"), say: "Accepted: the saved Kit items" },
  { look: GREP, say: "The Kit item, saved" },
  { press: tab("Open"), say: "Back to Open" },
]);
