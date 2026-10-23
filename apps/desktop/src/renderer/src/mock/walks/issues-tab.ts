// The Issues tab, mock only: open it with a key, change the query, read an issue a Job works and a Jira issue's fields,
// describe a query and save it, edit its text, start a Job on an issue, and teach Armada a tracker it cannot read.
// Over the `issues-tab` scenario.

import { button, dialog, inside, region, role, text, walk } from "../walk";

const places = role("navigation", "Places");
const list = region("Issues", { exact: true });

const issuesTab = walk("issues-tab", [
  { key: "Meta+k", on: places, say: "⌘K, then Issues" },
  { type: "Issues\n", into: inside(dialog("Command palette"), role("combobox")), say: "Each repository under its heading, from its own tracker. The glyph on #2810 is the Job working it" },
  { key: "L", on: list, say: "L opens the queries" },
  { press: role("menuitem", "Assigned to me"), say: "Assigned to me, in each tracker's own terms" },
  { press: inside(list, button(/^ABC-41 /)), say: "ABC-41 from Jira: every field it returned, and the Job working it, waiting on you" },
  { key: "/", on: list, say: "/ describes a query" },
  { type: "Open bugs nobody has picked up for this release\n", into: role("textbox", "Describe a query"), say: "The words, sent with Enter" },
  { press: inside(dialog("Describe a query"), button("Run")), say: "Asks before a model spends tokens" },
  { look: text(/priority in \(High, Highest\)/), say: "One query per tracker, in its own language, with the fields its rows show and what it matched" },
  { press: inside(role("form", "Save the query"), button("Save")), say: "Saved, named after the words" },
  { press: inside(role("listitem", "storefront"), button("Edit the query")), say: "A saved query's text can be edited by hand" },
  { type: 'is:open label:bug milestone:"v0.9"\n', into: role("textbox", "Query"), say: "Run again as written, with no model call. #2810 now matches too" },
  { press: inside(list, button(/^#2799 /)), say: "#2799 has no Job working it" },
  { key: "s", on: region(/^Issue #2799/), say: "s opens the dispatch sheet holding #2799 as a tag" },
  { key: "Escape", on: role("textbox", "Request"), say: "Esc puts the sheet away" },
  { press: button("Teach Armada this tracker"), say: "mobile names Linear, which Armada cannot read. Teach opens a brief for a tracker mod" },
]);

export { issuesTab as "issues-tab" };
