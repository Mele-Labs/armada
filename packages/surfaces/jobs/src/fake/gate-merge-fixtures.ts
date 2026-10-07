// A Job at its review gate with a pull request open, by what the forge's checks on it have come
// to. Merge pressed on each: passed merges and the Job completes, running enables auto-merge and
// the Job stays, failed is off with the names.

import type { PullRequestChecks } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import { reviewAtDelivery } from "../fixtures/build/delivering";

function withChecks(checks: PullRequestChecks, name: string): JobFixture {
  const fixture = reviewAtDelivery();
  if (fixture.watched.state !== "read") return fixture;
  const whole = fixture.watched.detail;
  const delivery = whole.delivery;
  if (delivery === undefined) return fixture;
  return {
    ...fixture,
    name,
    watched: {
      ...fixture.watched,
      detail: {
        ...whole,
        delivery: {
          ...delivery,
          pull_request_detail: { ...(delivery.pull_request_detail ?? { reviews: [] }), checks },
        },
      },
    },
  };
}

export const gateChecksPassed = (): JobFixture =>
  withChecks({ kind: "all_passed", checks: 4, finished: 4 }, "awaiting_review, the forge's checks passed");

export const gateChecksRunning = (): JobFixture =>
  withChecks({ kind: "still_waiting", checks: 4, finished: 2 }, "awaiting_review, the forge's checks running");

export const gateChecksFailed = (): JobFixture =>
  withChecks(
    { kind: "some_failed", checks: 4, finished: 4, failed: ["ci / test", "ci / lint"] },
    "awaiting_review, two of the forge's checks failed",
  );
