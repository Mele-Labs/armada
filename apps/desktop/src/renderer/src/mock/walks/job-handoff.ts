// A Job near the end of its window asks to carry on in a new one. He says yes with 1; the new Job takes the row, with the same title and
// place and every held thing carried across, and the old Job is behind it: h opens it from the row, Back returns. Over `job-handoff`.
// Keys: 1 carry on in a new Job, 2 stay here, Answer sends, h the earlier Job, Esc or Back returns.

import { button, deckRow, inside, region, role, walk } from "../walk";

const list = region("Jobs", { exact: true });
const ask = role("article", "Waiting on you");
const needs = region("Needs you");
const YES = "Carry on in a new Job";
const choice = (name: string) => inside(needs, role("radio", name, { exact: true }));
const row = deckRow("Retry the slot lease");

export const jobHandoff = walk("job-handoff", [
  { press: row, say: "The Job is open, and its question is in Needs you" },
  { press: inside(needs, button(/^Open Carry on/)), say: "The question, answered where it stands" },
  { look: ask, say: "Carry on in a new Job, or stay here. It has started nothing, and its answers are numbered" },
  { key: "1", on: choice(YES), say: "1 picks yes" },
  { press: inside(needs, button("Answer")), say: "Answer sends it" },
  { look: inside(list, role("img", "Handing off")), say: "The new Job is coming up. The row is where it was" },
  { look: inside(list, button("Earlier Job")), say: "One row, the same title and place. The earlier Job is behind it" },
  { key: "h", on: row, say: "h opens the earlier Job in the same place" },
  { look: inside(region(/^Job: /), role("img", "Ended")), say: "The earlier Job, on the stage where the new one was, holding nothing" },
  { press: inside(region(/^Job: /), button(/^Back/)), say: "Back returns to the new Job as it was" },
  { look: inside(list, button("Earlier Job")), say: "The one row, as it was" },
]);
