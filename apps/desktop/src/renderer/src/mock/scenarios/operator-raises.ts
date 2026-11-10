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
    text: "**Time each build step** needs you: Where should the step timings be kept?\nIt reads them back on every run, and the store already keeps each run's rows, so a file beside it would drift.\nPick: **In the store**\n\nSettled at 08:05.",
  },
  {
    id: "briefing",
    from: "operator",
    at: `${DAY}T08:41:00Z`,
    text: "**Good morning.** Quick dispatch's plan is the one to look at first.\n\n**Since last night**\n- Merged #3398 Fleet takes Bridge views\n- Ended Tail-first thread loading\n\n**Waiting on you**\n- Quick dispatch: Approve plan version 2. Pick: Approve the plan\n- Rework the Map legend: Review the Map legend walk. Pick: Open the walk\n- Store reports its size: Review pull request #3410\n\n**I'm on**\n- Nothing",
  },
  {
    id: "raise-plan",
    from: "operator",
    at: `${DAY}T08:41:10Z`,
    text: "**Quick dispatch** needs you: Approve plan version 2.\nIt adds the repository chips you asked for on 15 Oct and changes nothing else in the version you approved.\nPick: **Approve the plan**",
  },
  {
    id: "raise-walk",
    from: "operator",
    at: `${DAY}T08:41:20Z`,
    text: "**Rework the Map legend** needs you: Review the Map legend walk.\nIt shows the new key beside the old one, and its last step is the change you sent back yesterday.\nPick: **Open the walk**",
  },
  {
    id: "raise-pull",
    from: "operator",
    at: `${DAY}T08:41:30Z`,
    text: "**Store reports its size** needs you: Review pull request #3410.",
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
