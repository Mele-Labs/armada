// A dispatched request is a Job at `proposing` from the press, and it fills in
// as the proposer writes: workflow, then title, then done-when, then settings.
// The owner's decision of 30 Sep 2026; Fleet names the Job on `proposal.moved`
// since 21.6, and `proposer-fleet.ts` sends those messages as Fleet does.

import { button, inside, role, text, walk } from "../walk";

const REQUEST = "Make the stat say what is running";
const TITLE = "Show what's running in the Drones stat";

/** The Board's row for a Job, by the words on it. */
const boardRow = (words: string) => role("option", words);

export const proposingFillsIn = walk("proposing-fills-in", [
  { press: button("Dispatch", { exact: true }), say: "Dispatch the request" },
  { look: boardRow(REQUEST), say: "Its row arrives at once, titled with the request" },
  { look: inside(boardRow(REQUEST), text("feature")), say: "The workflow lands first" },
  { look: boardRow(TITLE), say: "Then the title replaces the request" },
  { press: boardRow(TITLE), say: "Open the Job" },
  { look: text("The rail's Drones stat reads one running"), say: "Done-when arrives a line at a time" },
  { look: text("opus"), say: "The settings land last" },
]);
