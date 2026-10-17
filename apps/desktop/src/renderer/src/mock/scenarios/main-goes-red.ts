// Main going red on CI, and the owner's part in each way it ends: a pull request that merged it, one
// merged by hand, a person's own. Drawn ahead of Fleet in `main-red-hub.ts`.

import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import { mainRed, writingMainsLogs } from "../main-red-hub";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

function goesRed(): Scenario {
  const base = holding("main-red-hub", "Main green, then red three times, each handed to a Session");
  const { first, later } = mainRed(repository().root);
  // Sessions served, so the Dispatch panel offers a Session.
  const draft = { ...base.draft, sessions: () => sessionsStore() };
  return { ...base, state: { ...base.state, ...first }, later, draft, behaves: writingMainsLogs };
}

export const s180MainRedHub: Scenario = goesRed();
