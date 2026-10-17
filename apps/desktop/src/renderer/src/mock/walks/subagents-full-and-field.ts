// A Session with no plan and two subagents at work. The Field has a drone for each, Progress lists them, and Subagents opens one full size over
// the app, with Esc back to the list as it was. Over the `subagents-full-and-field` scenario.

import { button, deckRow, dialog, inside, region, role, walk } from "../walk";

const field = region("Field");
const layer = region("Subagents", { exact: true });

export const subagentsFullAndField = walk("subagents-full-and-field", [
  { press: deckRow("Name the store migrations"), say: "The Job has no plan, and two subagents are at work" },
  { look: inside(field, role("option", /Harden the restart scripts/)), say: "The Field has a drone for each, at rest until an edit places it" },
  { look: inside(region("Progress"), role("list", "Subagents")), say: "Progress lists them, each its working mark and its title" },
  { key: "9", on: region("Flight controls"), say: "Subagents, on the 9 key" },
  { press: inside(layer, button("Failed refresh toast")), say: "A subagent opens over the whole app, not inside the list" },
  { key: "Escape", on: dialog("Subagent Failed refresh toast"), say: "Esc is back to the list, with the row that was pressed still in focus" },
]);
