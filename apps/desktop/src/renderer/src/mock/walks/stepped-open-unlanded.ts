// A stepped Job's Open zone holds what has not landed: the open ticket and the draft pull request stand there until they
// land, beside the work already merged. The owner's read of the stepped card's empty Open on 4 Nov 2026, fixed by drawing
// `unlandedOf` into it. `StoryColumn.test.tsx` holds the claims.

import { deckRow, text, walk } from "../walk";

export const steppedOpenUnlanded = walk("stepped-open-unlanded", [
  {
    press: deckRow("Rotate the audit log"),
    say: "Open the Job on the feature workflow, at Refine",
  },
  {
    look: text("Document the audit log rotation"),
    say: "Open lists the ticket still open, until it closes",
  },
  {
    look: text("Keep the rotation window in the config"),
    say: "And the draft pull request, until it merges",
  },
]);
