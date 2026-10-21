// A Job whose thread holds Fleet's behind-base notice. A right press on it offers Disable for job, repo or 1 hour.
// The walk `mute-fleet-notice` plays it.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet } from "../sessions-fleet";
import { noticeSessions, noticeThreads } from "../sessions/mute-notice";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  const served = new FakeSessionsFleet(noticeSessions(), noticeThreads()).scenario(board);
  return {
    ...served,
    name: "mute-fleet-notice",
    says: "A right press on Fleet's behind-base notice offers Disable for job, Disable for repo and Disable for 1 hour",
    // A Fleet of its own for each window, so a walk told twice starts from the same Job.
    behaves: (handle) => new FakeSessionsFleet(noticeSessions(), noticeThreads()).scenario(board).behaves!(handle),
  };
}

export const muteFleetNotice: Scenario = build();
