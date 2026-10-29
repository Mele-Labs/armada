// Three hosted Jobs, one whose agent came up whole and two that came up without some of its Armada tools, as Fleet read the
// CLI's init line: a mark on each of those rows and in its header, its tooltip the label. The walk `job-tools-missing` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { RepositorySummary, SessionRow } from "@armada/protocol";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, hosted } from "../sessions-fleet";
import { AGENT } from "../slices/onboarding";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const now = () => new Date().toISOString();

const JOBS = [
  { id: "01TOOLSAAAAAAAAAAAAAAAAAA", title: "Cart totals rounding", asked: "Fix the cart totals rounding." },
  { id: "01TOOLSBBBBBBBBBBBBBBBBBB", title: "Add the billing page", asked: "Add the billing page." },
  { id: "01TOOLSCCCCCCCCCCCCCCCCCC", title: "Retry the webhook", asked: "Retry the webhook on a timeout." },
];

/** What the second and third Jobs came up without; the first came up whole. */
const MISSING: Record<string, NonNullable<ReturnType<typeof hosted>["tools_missing"]>> = {
  [JOBS[1]!.id]: { harness: AGENT.name, update: AGENT.update, harness_version: "2.1.312", missing: ["mod", "task_tools"] },
  [JOBS[2]!.id]: { harness: AGENT.name, update: AGENT.update, harness_version: "2.1.308", missing: ["door"] },
};

/** Idle, the last turn ended and nothing is asked of him. */
const records = () =>
  JOBS.map((one) => {
    const missing = MISSING[one.id];
    return hosted(one.id, { title: one.title, last_turn_at: now(), standing: "idle", ...(missing === undefined ? {} : { tools_missing: missing }) });
  });

const threads = (): Record<string, SessionRow[]> =>
  Object.fromEntries(
    JOBS.map((one) => [
      one.id,
      [
        { id: `${one.id}-1`, at: "09:13:40", kind: "message", from: { kind: "you" }, text: one.asked },
        { id: `${one.id}-2`, at: "09:14:02", kind: "message", from: { kind: "agent" }, text: "Done." },
      ] satisfies SessionRow[],
    ]),
  );

function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  const served = new FakeSessionsFleet(records(), threads()).scenario(board);
  return {
    ...served,
    name: "job-tools-missing",
    says: "Three Jobs: one whole, two whose agent came up without some Armada tools, each marked on its row and in its header",
    behaves: (handle) => new FakeSessionsFleet(records(), threads()).scenario(board).behaves!(handle),
  };
}

export const jobToolsMissing: Scenario = build();
