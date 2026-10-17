// The one way a scenario is built from repositories: a connected Fleet serving them. Apart from
// `scenario.ts` so a row in `scenarios/` can build on it without importing the list it is listed in.

import type { RepositorySummary } from "@armada/protocol";
import { repository, workflow } from "@armada/screens/src/fixtures/repository";

import { connected } from "./moment";
import type { Scenario } from "./moment";

/**
 * A connected Fleet serving these repositories, the base one unless named. **Every repository
 * given is set up**, so one nobody set up can only arrive through `state.holds` by hand.
 */
export function holding(name: string, says: string, repositories: RepositorySummary[] = [repository()]): Scenario {
  return { name, says, state: connected([workflow()], repositories) };
}
