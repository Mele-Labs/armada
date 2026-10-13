// A write clash is marked on both rows of the pair: the Dashboard's tiles, the Sessions list, and the
// open Job or Session, where the other holder is a link and the conflicting files are listed. Two
// Sessions that edit one file without a conflict are not marked. Over `write-clash-mark`.

import { button, inside, region, role, tab, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const tile = (title: RegExp) => role("option", title);
const mark = (title: RegExp, other: RegExp) => inside(tile(title), button(other));
const work = role("navigation", "Work");

const writeClashMark = walk("write-clash-mark", [
  { press: tab("Active"), say: "Active, Jobs and Sessions as tiles" },
  { look: mark(/^Migration notes/, /Conflicts with Release script/), say: "Two Sessions whose changes conflict: the mark on this one" },
  { look: mark(/^Release script/, /Conflicts with Migration notes/), say: "And on the other" },
  { look: tile(/^Changelog tidy/), say: "Changelog tidy and Release notes draft edit one file and do not conflict: no mark on either" },
  { hover: mark(/^Cap the retry backoff/, /Conflicts with Retry tuning/), say: "The tooltip names the other holder and the files" },
  { look: mark(/^Retry tuning/, /Conflicts with Cap the retry backoff/), say: "A Job and a Session on two files, marked on both" },
  { press: mark(/^Migration notes/, /Conflicts with Release script/), say: "Press the mark" },
  { look: inside(region("Write clash"), button("Open Migration notes")), say: "The other Session opens, with the first linked" },
  { look: inside(region("Write clash"), text("clock.rs")), say: "And the file they both changed" },
  { press: inside(work, button("Cockpit")), say: "Back to the Cockpit" },
  { press: mark(/^Retry tuning/, /Conflicts with Cap the retry backoff/), say: "Press the mark on the Session that clashes with a Job" },
  { look: inside(region("Write clash"), text("backoff.rs")), say: "The Job opens, listing both files" },
  { press: inside(work, button("Cockpit")), say: "Back to the Cockpit" },
  { hover: inside(tile(/^Lint sweep/), role("img", /Conflicts with main/)), say: "A Session against a main that moved: the other holder is main" },
  toSessions,
  { look: inside(region("Sessions"), role("img", /Conflicts with main/)), say: "The Sessions list marks the same Session" },
]);

export { writeClashMark as "write-clash-mark" };
