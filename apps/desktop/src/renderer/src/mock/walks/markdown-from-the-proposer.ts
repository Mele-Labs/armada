// What a model and a person wrote reads as the markdown it was written in: a
// criterion in the Proposer's wait and the request on the Brief card. The
// owner's note of 1 Oct 2026 on the Brief card; `markdownFromAReviewer` is the
// other half, on a Job a reviewer has commented on.

import { inside, region, text, walk } from "../walk";

export const markdownFromTheProposer = walk("arc/proposing-done-when-landed", [
  { press: text("Say which of the two a clear gave back"), say: "A Job the Proposer is still writing" },
  {
    look: text("does not say it twice"),
    say: "A done-when line the model wrote, with branch drawn as code",
  },
  {
    look: inside(region("Brief"), text("the branch as well")),
    say: "The request on the Brief card: a list, with bold and code in it",
  },
]);
