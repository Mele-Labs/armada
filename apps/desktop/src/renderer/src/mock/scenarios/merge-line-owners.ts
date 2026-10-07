// Who each entry on the merge line came from: one on a branch and pull request a Session holds,
// one on a Job's branch, and one nobody here owns. Over the `sessions` scenario with one more
// Session open. The walk `merge-line-owners` plays it.

import type { MergeLine } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";
import { review } from "@armada/jobs/fixtures/build/index";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { asRow } from "../holding";
import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const PULL = "https://git.example/armada/pull/";

const migrations = asRow(review(), 61, "order-store-migrations", "Order the store migrations");
const jobBranch = "fix/61-order-store-migrations";
const owned = { ...migrations, job: { ...migrations.job, branch: jobBranch } };

const more: Session[] = [
  {
    id: "s8",
    title: "Pin the store clock",
    turn: { state: "idle" },
    lastTurn: "14:20",
    rows: [],
    attachments: [
      { kind: "branch", name: "fix/pin-store-clock", slot: 2 },
      {
        kind: "pull_request",
        number: 1861,
        title: "Pin the store clock",
        branch: "fix/pin-store-clock",
        address: `${PULL}1861`,
        checks: { state: "passed" },
        state: "open",
        auto: false,
      },
    ],
  },
];

function line(): MergeLine {
  return {
    root: repository().root,
    line: [
      { place: 1, branch: "fix/pin-store-clock", pull_request: { number: 1861, url: `${PULL}1861` }, state: "waiting" },
      { place: 2, branch: jobBranch, pull_request: { number: 1862, url: `${PULL}1862` }, state: "waiting" },
      { place: 3, branch: "docs/typo-in-the-readme", pull_request: { number: 1863, url: `${PULL}1863` }, state: "waiting" },
    ],
    off: [],
    landed: [],
    sent_back: [],
  };
}

export const s203MergeLineOwners: Scenario = {
  ...s200Sessions,
  name: "merge-line-owners",
  says: "A merge line with an entry a Session owns, one a Job owns and one nobody does",
  state: { ...s200Sessions.state, jobs: [...s200Sessions.state.jobs, owned.job], mergeLines: { lines: [line()] } },
  reads: { ...s200Sessions.reads, [owned.job.id]: owned },
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, more) },
};
