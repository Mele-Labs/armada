// Main going red on CI, and the owner's part in each way it ends: the Job that merged it takes it,
// a Job it is sent back to takes it, a new Job dispatched for it takes it. Drawn ahead of Fleet in
// `main-red-hub.ts`; the walk `mainGoesRed` plays it.

import { repository } from "@armada/screens/src/fixtures/build/base";

import { holding } from "../holding";
import { FIXTURES, mainRed, writingMainsLogs } from "../main-red-hub";
import type { Scenario } from "../moment";

function goesRed(): Scenario {
  const base = holding("main-red-hub", "Main green, then red three times, each handed to a Job", FIXTURES);
  const { first, later } = mainRed(repository().root);
  return { ...base, state: { ...base.state, ...first }, later, behaves: writingMainsLogs };
}

export const s180MainRedHub: Scenario = goesRed();
