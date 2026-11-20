// Prioritise from a Job row's menu: the row carries a mark, the menu reads Stop prioritising on the Job that holds it, and the mark moves to
// the Job pressed. Over `pausing-a-session`.

import { button, inside, region, role, walk } from "../walk";

const list = region("Jobs", {
  exact: true,
});
const held = inside(list, button("Pin the store clock"));
const other = inside(list, button("Rebuild the stage grid"));
const mark = inside(list, role("img", "Prioritised"));

const prioritiseAJob = walk("pausing-a-session", [
  {
    look: mark,
    say: "One Job carries the priority mark",
  },
  {
    key: "ContextMenu",
    on: held,
    say: "The menu of the Job that holds it",
  },
  {
    look: role("menuitem", "Stop prioritising"),
    say: "It reads Stop prioritising",
  },
  {
    key: "Escape",
    on: role("menu"),
    say: "Closed with nothing changed",
  },
  {
    key: "ContextMenu",
    on: other,
    say: "The menu of another Job",
  },
  {
    press: role("menuitem", "Prioritise"),
    say: "Prioritise",
  },
  {
    look: mark,
    say: "The mark moved to this Job and left the other",
  },
]);

export { prioritiseAJob as "prioritise-a-job" };
