// Open pull requests under the repository's merge queue: each row's queue state is a mark with a
// tooltip, its place a small `#n`, and its owner the chip every other row on the line wears. One
// waits for ci to join the queue, one is queued second, one awaits checks first, one cannot merge,
// and one is not in the queue at all. Told twice: wide, and below the breakpoint.

import { NARROW } from "../sessions/walk-kit";
import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";

function steps(): Step[] {
  const line = region("Merge line");
  const row = (branch: string) => inside(line, role("listitem", new RegExp(`^${branch}`)));
  const mark = (branch: string, said: string) => inside(row(branch), role("img", said));
  return [
    { press: button("Merge line", { exact: true }), say: "The merge line: no armada land queue, only open pull requests" },
    { look: row("docs/typo-in-the-readme"), say: "Auto-merge is on and ci is running: it has not joined the queue" },
    { hover: mark("docs/typo-in-the-readme", "Waiting for ci to join the merge queue"), say: "The mark names it" },
    { look: row("fix/61-order-store-migrations"), say: "First in the queue, running its checks, a Job's pull request" },
    { hover: mark("fix/61-order-store-migrations", "In the merge queue, running its checks"), say: "Checks running in the queue" },
    { look: row("fix/pin-store-clock"), say: "Second, waiting its turn, a Session's pull request" },
    { hover: mark("fix/pin-store-clock", "In the merge queue, waiting its turn"), say: "Queued" },
    { look: row("chore/bump-the-lockfile"), say: "Third, and the queue cannot merge it" },
    { hover: mark("chore/bump-the-lockfile", "In the merge queue, cannot merge"), say: "Unmergeable" },
    { look: row("wip/not-in-the-queue"), say: "Not in the queue: its ci mark stands, no place" },
    { hover: inside(row("fix/pin-store-clock"), button("Pull request #1861")), say: "The owner chip takes a hover as it does on the other lists" },
  ];
}

const wide = walk("merge-queue", steps());
const narrow = walk("merge-queue", steps(), NARROW);

export { wide as "merge-queue", narrow as "merge-queue-narrow" };
