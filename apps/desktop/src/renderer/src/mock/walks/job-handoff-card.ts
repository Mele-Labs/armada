// The handoff card with its four answers: stay and do not ask again, or remind at a higher limit, which brings a second question with three figures. Over `job-handoff-card`.
// Keys: 1 to 4 pick an answer, Answer sends, then 1 to 3 pick the figure.

import { button, deckRow, inside, region, role, walk } from "../walk";

const needs = region("Needs you");
const choice = (name: string) => inside(needs, role("radio", name, { exact: true }));

export const jobHandoffCard = walk("job-handoff-card", [
  { press: deckRow("Retry the slot lease"), say: "The Job is open, and its question is in Needs you" },
  { look: choice("Carry on in a new Job"), say: "1 carries on in a new Job" },
  { look: choice("Stay here"), say: "2 stays here" },
  { look: choice("Stay, don't ask again this Job"), say: "3 stays, and the card does not come back for this Job" },
  { look: choice("Remind me at a higher limit"), say: "4 stays, and asks again at a higher figure" },
  { key: "4", on: choice("Remind me at a higher limit"), say: "4 picks remind" },
  { press: inside(needs, button("Answer")), say: "Answer sends it" },
  { look: choice("450k"), say: "The figures, numbered like the answers" },
  { key: "1", on: choice("450k"), say: "1 picks 450k" },
  { press: inside(needs, button("Answer")), say: "Answer sends it" },
  { look: deckRow("Retry the slot lease"), say: "The Job stays on its row, and the card is gone" },
]);
