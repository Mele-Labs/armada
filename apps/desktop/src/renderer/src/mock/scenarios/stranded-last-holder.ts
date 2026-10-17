// Cleanup's pool with three stranded slots that name who held them last. Slot 5: a Session still open,
// told once at 10:42. Slot 6: a Session closed, its Scout's Finding already read. Slot 7: a Session closed,
// its Finding read. Slot 4 is a stranded slot whose holder Fleet never recorded and draws nothing.
// The walk `aStrandedSlotsLastHolder` plays it.

import type { SlotFinding, WorktreeSlot } from "@armada/protocol";
import { holding } from "../holding";
import { slotsHeld } from "@armada/screens/src/fixtures/slots";

import type { Scenario } from "../moment";
import { FakeSessionsFleet, hosted } from "../sessions-fleet";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** Today at 10:42 local time, or yesterday's where that is still to come. */
const told = (): string => {
  const at = new Date();
  at.setHours(10, 42, 0, 0);
  if (at.getTime() > Date.now()) at.setDate(at.getDate() - 1);
  return at.toISOString();
};

const finding = (rest: Partial<SlotFinding>): SlotFinding => ({
  state: "answered",
  commit: "e08c4d1a77",
  uncommitted: true,
  read: ["crates/fleet/src/slots.rs"],
  searched: ["lease_record in crates/fleet/src/"],
  ...rest,
});

const OPEN = "01SESSIONPRPANELDESIGN000";

const stranded = (n: number, rest: Partial<WorktreeSlot> & Pick<WorktreeSlot, "held">): WorktreeSlot => ({
  manifest_id: "armada",
  slot: n,
  path: `/Users/user/armada/.armada/slots/slot-${n}`,
  base: "main",
  warm: false,
  ...rest,
});

function pool() {
  const base = slotsHeld(Date.now());
  const slots = (base.slots ?? []).map((one) => {
    if (one.slot === 5) {
      return stranded(5, {
        held: { state: "stranded", why: "1 uncommitted, first crates/adapter-traits/src/pull_request_detail.rs" },
        branch: "session/pull-request-panel-design",
        since: ago(60),
        behind: 66,
        stranded: {
          uncommitted: ["crates/adapter-traits/src/pull_request_detail.rs"],
          commits: [],
          unpushed: 0,
          last_holder: { kind: "session", id: OPEN, title: "Pull Request Panel Design", open: true },
          told: told(),
        },
      });
    }
    if (one.slot === 6) {
      return stranded(6, {
        held: { state: "stranded", why: "2 uncommitted, first apps/desktop/src/renderer/src/mock/scenario.ts" },
        branch: "session/retry-the-manifest-read",
        since: ago(3 * 60),
        behind: 14,
        stranded: {
          uncommitted: ["apps/desktop/src/renderer/src/mock/scenario.ts", "notes/retry.md"],
          commits: [{ sha: "e08c4d1a77", subject: "Retry the manifest read once", home: "only_here" }],
          unpushed: 1,
          last_holder: { kind: "session", id: "01SESSIONCLOSEDRETRY00000", title: "Retry the manifest read", open: false },
        },
        rescue: finding({ verdict: "unfinished", items: ["Wire the retry into the manifest read", "Add the test for a short answer"] }),
      });
    }
    if (one.slot === 7) {
      return {
        ...one,
        stranded: {
          uncommitted: one.stranded?.uncommitted ?? [],
          commits: one.stranded?.commits ?? [],
          unpushed: one.stranded?.unpushed ?? 0,
          last_holder: { kind: "session" as const, id: "01SESSIONCLOSEDLEASE00000", title: "Write the lease record", open: false },
        },
        rescue: finding({ verdict: "scraps", items: ["Notes on the lease record, nothing to finish"] }),
      };
    }
    return one;
  });
  return { ...base, slots };
}

const sessions = () => [
  hosted(OPEN, { title: "Pull Request Panel Design" }),
];

const board = holding("cleanup/last-holder", "The worktree pool on Cleanup, stranded slots that name who held them last");

export const strandedLastHolder: Scenario = { ...new FakeSessionsFleet(sessions()).scenario(board), held: pool() };
