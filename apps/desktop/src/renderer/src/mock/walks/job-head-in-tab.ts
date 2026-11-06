// A Job in panes has no head bar: its Thread tab carries the state mark, the name, Rename and More, which holds the Job's acts
// and End job last. In an opened Needs you item Back sits on the title line, Remove is centred on it, and the reply box has
// no fill. Over the `thread-states` scenario.

import { button, deckRow, inside, region, role, tab, walk } from "../walk";

const job = tab(/^Thread: /);
const thread = region(/^Pane: Thread/);
const needs = region("Needs you");
const item = "Choose when the migration runs on staging";

const jobHeadInTab = walk("thread-states", [
  {
    press: deckRow("Staging migration"),
    say: "A Job waiting on an answer. No bar over its thread",
  },
  {
    look: job,
    say: "Its tab carries the state mark and the name",
  },
  {
    press: inside(thread, button("More")),
    say: "More on the tab: Rename, the tab's own acts, and End job last",
  },
  {
    press: role("menuitem", "Rename"),
    say: "Rename selects the name where it stands, ready to type over",
  },
  {
    look: role("textbox", "Rename Staging migration"),
    say: "The mark, More and Close keep their places",
  },
  {
    key: "Escape",
    on: role("textbox", "Rename Staging migration"),
    say: "Esc keeps the name",
  },
  {
    press: role("button", /^Needs you/),
    say: "Needs you opened on its one item",
  },
  {
    look: inside(needs, button("Back")),
    say: "Back sits on the item's title line",
  },
  {
    look: inside(needs, button(`Remove ${item}`)),
    say: "Remove is centred on the same line",
  },
  {
    look: inside(needs, role("textbox", `Reply to ${item}`)),
    say: "The reply box has a border and no fill",
  },
]);

export { jobHeadInTab as "job-head-in-tab" };
