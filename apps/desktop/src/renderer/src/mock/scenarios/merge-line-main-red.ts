// Main red on the merge line's page with a Job already on the fix, the moment the main light's card
// has the most to say: the failing Check, its test, the merge that broke it, and the Job fixing it.
// `main-red-hub`'s second moment, opened on rather than walked to.

import { repository } from "@armada/screens/src/fixtures/build/base";

import { holding } from "../holding";
import { FIXTURES, mainRed, writingMainsLogs } from "../main-red-hub";
import type { Scenario } from "../moment";

function redAndTaken(): Scenario {
  const base = holding("merge-line-main-red", "Main red on the merge line, a Job already fixing it", FIXTURES);
  const { later } = mainRed(repository().root);
  return { ...base, state: { ...base.state, ...later[0]! }, behaves: writingMainsLogs };
}

export const s182MergeLineMainRed: Scenario = redAndTaken();
