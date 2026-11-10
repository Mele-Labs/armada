// The Operator raising what needs him in Discussion: the day's briefing, then one line for each Needs you item with its
// read and its pick, one with no read, and one he already settled. For the walk `operatorRaises`.

import { holding } from "../holding";
import type { Scenario } from "../moment";
import type { OperatorMessage } from "../../operator/model";
import { operatorPanel } from "./operator-panel";

const DAY = "2026-11-11";

const said: readonly OperatorMessage[] = [
  {
    id: "raise-timings",
    from: "operator",
    at: `${DAY}T07:52:00Z`,
    text: "Time each build step needs you: Where should the step timings be kept? It reads them back on every run, and the store already keeps each run's rows, so a file beside it would drift. I'd pick: In the store. Settled at 08:05.",
  },
  {
    id: "briefing",
    from: "operator",
    at: `${DAY}T08:41:00Z`,
    text: "Good morning. Since last night, Fleet takes Bridge views merged as #3398 and Tail-first thread loading ended. Waiting on you: Quick dispatch has a plan, Rework the Map legend has a walk, and Store reports its size has a pull request. Each is below with my pick.",
  },
  {
    id: "raise-plan",
    from: "operator",
    at: `${DAY}T08:41:10Z`,
    text: "Quick dispatch needs you: Approve plan version 2. It adds the repository chips you asked for on 15 Oct and changes nothing else in the version you approved. I'd pick: Approve the plan.",
  },
  {
    id: "raise-walk",
    from: "operator",
    at: `${DAY}T08:41:20Z`,
    text: "Rework the Map legend needs you: Review the Map legend walk. It shows the new key beside the old one, and its last step is the change you sent back yesterday. I'd pick: Open the walk.",
  },
  {
    id: "raise-pull",
    from: "operator",
    at: `${DAY}T08:41:30Z`,
    text: "Store reports its size needs you: Review pull request #3410.",
  },
];

function build(): Scenario {
  const base = holding(
    "operator-raises",
    "The Operator's Discussion with the day's briefing and a raise for each thing that needs him, each with its read and its pick",
    [],
  );
  return {
    ...operatorPanel,
    name: base.name,
    says: base.says,
    draft: {
      ...operatorPanel.draft,
      operator: {
        ...operatorPanel.draft!.operator!,
        said,
      },
    },
  };
}

export const operatorRaises: Scenario = build();
