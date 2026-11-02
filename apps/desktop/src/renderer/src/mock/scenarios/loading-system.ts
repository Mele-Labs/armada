// System pages before Fleet has answered: Checks (run sheet reading, run list pending, no merge line yet), Skills (the Kit
// inventory), Workflows (the list) and the Requests panel's collections tree. Each read is held pending, so every page
// draws its skeleton rows and nothing arrives. The same board as `checks`, which is the one with data to land on.

import type { Scenario } from "../moment";
import { checkingOver } from "../checks-fleet";

function build(): Scenario {
  const base = checkingOver(
    {
      state: "reading",
    },
    "loading/system",
    "System pages while Fleet's reads are still out: Checks, Skills, Workflows and the Requests collections tree show skeletons",
  );
  const never = () => new Promise<never>(() => {});
  return {
    ...base,
    state: {
      ...base.state,
      mergeLines: null,
    },
    behaves: (...args) => ({
      ...base.behaves?.(...args),
      listCheckoutRuns: never,
      readKitInventory: never,
      readWorkflows: never,
      readRequests: never,
    }),
  };
}

export const loadingSystem: Scenario = build();
