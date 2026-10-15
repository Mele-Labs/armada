// A Session's message box with `@` open: the Sessions first, then the Jobs, and once a few letters
// follow it the files that match them. Over the `sessions` scenario. The walk `composer-at-groups`
// plays it. The files are the mock's own (`searchFiles` in `@armada/jobs/fake`).

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

export const s245ComposerAtGroups: Scenario = {
  ...s200Sessions,
  name: "composer-at-groups",
  says: "A Session's `@` offers Sessions, then Jobs, then the files that match what follows it",
};
