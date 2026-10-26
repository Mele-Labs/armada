// The agent's sign-in lapsed for every Job at once: one alert with a Sign in act, the Jobs behind it. Pressed, it
// shows the sign-in under way and then goes. The source is `mock/agent-sign-in.ts`. The walk `agent-signed-out` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { RepositorySummary, SessionRow } from "@armada/protocol";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const now = () => new Date().toISOString();

const FAILED = "Failed to authenticate: OAuth session expired and could not be refreshed";

const JOBS = [
  { id: "01SIGNEDOUTAAAAAAAAAAAAAA", title: "Cart totals rounding", asked: "Fix the cart totals rounding." },
  { id: "01SIGNEDOUTBBBBBBBBBBBBBB", title: "Add the billing page", asked: "Add the billing page." },
  { id: "01SIGNEDOUTCCCCCCCCCCCCCC", title: "Retry the webhook", asked: "Retry the webhook on a timeout." },
];

/** Idle, the last turn ended on the harness's message and nothing is asked of him. */
const records = () => JOBS.map((one) => hosted(one.id, { title: one.title, last_turn_at: now(), standing: "idle" }));

const threads = (): Record<string, SessionRow[]> =>
  Object.fromEntries(
    JOBS.map((one) => [
      one.id,
      [
        { id: `${one.id}-1`, at: "09:13:40", kind: "message", from: { kind: "you" }, text: one.asked },
        { id: `${one.id}-2`, at: "09:14:02", kind: "message", from: { kind: "agent" }, text: FAILED },
      ] satisfies SessionRow[],
    ]),
  );

function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  const served = new FakeSessionsFleet(records(), threads()).scenario(board);
  return {
    ...served,
    name: "agent-signed-out",
    says: "Claude signed out for every Job: an alert with a Sign in act, which shows the sign-in under way and then goes",
    behaves: (handle) => new FakeSessionsFleet(records(), threads()).scenario(board).behaves!(handle),
  };
}

export const agentSignedOut: Scenario = build();
