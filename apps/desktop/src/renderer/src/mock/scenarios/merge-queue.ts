// The repository's merge queue on the merge line hub: one pull request waiting for ci to join it
// with auto-merge on, one queued second, one awaiting checks first, and one the queue cannot
// merge. Owners as in `merge-line-owners`: a Session, a Job, and one nobody here holds. The walk
// `merge-queue` plays it.

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
        auto: true,
      },
    ],
  },
];

function line(): MergeLine {
  const pull = (number: number, branch: string) => ({ number, title: branch, branch, url: `${PULL}${number}` });
  return {
    root: repository().root,
    line: [],
    off: [],
    landed: [],
    sent_back: [],
    hub: {
      pull_requests: [
        { ...pull(1863, "docs/typo-in-the-readme"), ci: "running", queue: { state: "waiting_for_ci" } },
        { ...pull(1861, "fix/pin-store-clock"), ci: "passed", queue: { state: "queued", position: 2 } },
        { ...pull(1862, jobBranch), ci: "passed", queue: { state: "awaiting_checks", position: 1 } },
        { ...pull(1864, "chore/bump-the-lockfile"), ci: "passed", queue: { state: "unmergeable", position: 3 } },
        { ...pull(1865, "wip/not-in-the-queue"), ci: "failed" },
      ],
    },
  };
}

export const s204MergeQueue: Scenario = {
  ...s200Sessions,
  name: "merge-queue",
  says: "Open pull requests at each merge queue state, with their owners",
  state: { ...s200Sessions.state, jobs: [...s200Sessions.state.jobs, owned.job], mergeLines: { lines: [line()] } },
  reads: { ...s200Sessions.reads, [owned.job.id]: owned },
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, more) },
};
