// A Job holding pull request #1847, its checks still running, over a Fleet that refuses every press on it: Enable auto-merge
// comes back as `fleet.auto_merge_refused`, "Auto merge is not allowed", which the panel says in its own sheet. The walk
// `pr-panel-refusal` plays it.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted, pullRequest } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const JOB = "01SESSIONPRPANELREFUSALAA";
const AT = "2026-10-23T09:00:00.000Z";

function refusing(): FakeSessionsFleet {
  const fleet = new FakeSessionsFleet(
    [hosted(JOB, { title: "Fix the flaky store test", attachments: [held("slot", "3"), held("branch", "fix/flaky-store"), pullRequest(1847, { checks: "pending" })] })],
    { [JOB]: [{ kind: "message", id: "m1", at: AT, from: { kind: "you" }, text: "Fix the flaky store test, and open a pull request." }] },
  );
  fleet.refusesPress = { code: "fleet.auto_merge_refused", message: "Auto merge is not allowed" };
  return fleet;
}

const build = (fleet: FakeSessionsFleet): Scenario => fleet.scenario(onBoard({ repositories: [ARMADA], picked: ARMADA.root }));

export const prPanelRefusal: Scenario = {
  ...build(refusing()),
  name: "pr-panel-refusal",
  says: "A Job's pull request with its checks running, over a Fleet that refuses Enable auto-merge",
  // A Fleet of its own each time, so a walk told twice starts from the same Job.
  behaves: (handle) => build(refusing()).behaves!(handle),
};
