// What a model and a person wrote reads as the markdown it was written in: a
// criterion in the Proposer's wait and the request on the Brief card. The
// owner's note of 1 Oct 2026 on the Brief card; `markdownFromAReviewer` is the
// other half, on a Job a reviewer has commented on.

import { button, card, dialog, inside, role, tab, text, walk } from "../walk";

export const markdownFromTheProposer = walk("arc/proposing-done-when-landed", [
  { press: tab("Active"), say: "A Job the Proposer is still writing" },
  { press: role("option", /Say which of the two a clear gave back/), say: "Picked" },
  { press: button(/^(Open|Review|Redirect|Attest)$/), say: "Open the Job" },
  {
    look: text("does not say it twice"),
    say: "A done-when line the model wrote, with branch drawn as code",
  },
  { press: card("Brief"), say: "The request, on the canvas's Brief" },
  {
    look: inside(dialog("Brief"), text("the branch as well")),
    say: "The request on the Brief: a list, with bold and code in it",
  },
]);
