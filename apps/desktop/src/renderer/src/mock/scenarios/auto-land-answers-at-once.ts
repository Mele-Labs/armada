// A Job holding five open pull requests, Auto land off. Fleet answers the press at once with the flag on and asks the forge after:
// the one it refuses reaches the Job's thread a moment later as a Fleet notice. The walk `auto-land-answers-at-once` plays it.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted, pullRequest } from "../sessions-fleet";

const ARMADA: RepositorySummary = {
  ...repository(),
  manifest: {
    ...repository().manifest!,
    id: "armada",
  },
};
export const TITLE = "Split the slot lease into two steps";
export const REFUSED =
  "Auto land is on, and the forge would not turn on auto-merge for #1843: Auto merge is not allowed";
const JOB = "01SESSIONAUTOLANDATONCE";
const AT = "2026-10-23T09:00:00.000Z";
const FORGE_MS = 1000;

const PULLS = [
  [
    1841,
    "Name the lease steps",
  ],
  [
    1842,
    "Move the lease clock",
  ],
  [
    1843,
    "Free a lease on release",
  ],
  [
    1844,
    "Count leases per bay",
  ],
  [
    1845,
    "Retire the old lease table",
  ],
] as const;

function asking(): FakeSessionsFleet {
  const fleet = new FakeSessionsFleet(
    [
      hosted(JOB, {
        title: TITLE,
        attachments: [
          held("slot", "3"),
          held("branch", "split/lease-steps"),
          ...PULLS.map(([number, title]) =>
            pullRequest(number, {
              title,
            }),
          ),
        ],
      }),
    ],
    {
      [JOB]: [
        {
          kind: "message",
          id: "m1",
          at: AT,
          from: {
            kind: "you",
          },
          text: "Split the slot lease into two steps, one pull request each.",
        },
      ],
    },
  );
  fleet.onAutoLand = (id, on) => {
    if (!on) return;
    window.setTimeout(
      () =>
        fleet.row(id, {
          kind: "message",
          id: "n-auto-land",
          at: new Date().toISOString(),
          from: {
            kind: "session",
            id: "fleet",
            title: "Fleet",
          },
          text: REFUSED,
          notice: "auto_land_refused",
        }),
      FORGE_MS,
    );
  };
  return fleet;
}

const build = (fleet: FakeSessionsFleet): Scenario =>
  fleet.scenario(
    onBoard({
      repositories: [
        ARMADA,
      ],
      picked: ARMADA.root,
    }),
  );

export const autoLandAnswersAtOnce: Scenario = {
  ...build(asking()),
  name: "auto-land-answers-at-once",
  says: "A Job holding five pull requests: Auto land answers at once, and the forge's refusal of one arrives in the thread as a Fleet notice",
  // A Fleet of its own each time, so a walk told twice starts from the same Job.
  behaves: (handle) => build(asking()).behaves!(handle),
};
