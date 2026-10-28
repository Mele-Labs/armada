// A Job whose agent started two Jobs, each held on its start card. The parent's Needs you lists a row for each: its press opens that
// Job, and its Start starts it from there. The second Job's request is still being read, so its Start is queued and shows Starting
// until the reading ends, a few seconds on; then it starts with the delivery the reading picked, its row goes, and a linked row
// lands in the parent's thread. The walk `child-job-start` plays it, over a Fleet that holds the starts as Fleet would (`../sessions-fleet.ts`).

import type { RepositorySummary, SessionRow } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const AT = "2026-10-12T09:14:02.000Z";
const PARENT = "01DEBUGCAPAAAAAAAAAAAAAAAA";

/** How long the proposer reads the second Job's request once Start is queued on it. */
const READS_MS = 2_500;

const LOADER = { title: "Settings loader and the cap", message: "Read crates/settings/src/loader.rs and find out whether a cap set for one repository is ever read. Report where the value is dropped." };
const PER_REPO = { title: "Retry cap per repository", message: "Make the retry loop read the cap on every attempt, so a cap set for one repository takes effect. Add a test that sets it to 3." };

const records = () => [
  hosted(PARENT, { title: "Debug the retry cap", attachments: [held("slot", "3"), held("branch", "fix/retry-cap")] }),
  hosted("01SESSIONBBBBBBBBBBBBBBBBB", { title: "Fix the flaky store test", attachments: [held("slot", "4"), held("branch", "fix/flaky-store")] }),
];

const thread = (): SessionRow[] => [
  { kind: "message", id: "m1", at: AT, from: { kind: "you" }, text: "The retry loop ran 5 attempts with the cap set to 3. Find out why, and start a Job for each fix." },
  { kind: "tool", id: "t1", at: AT, text: "Read crates/retry/src/loop.rs" },
  {
    kind: "message",
    id: "m2",
    at: AT,
    from: { kind: "agent" },
    text: "The loop reads the cap once at startup, so the lowered value never reaches the step. The settings loader also seems to ignore a cap set for one repository. I started a Job for each.",
  },
];

/** A Fleet whose agent has started both Jobs: the first read, the second still being read until Start is pressed on it. */
function fleet(): FakeSessionsFleet {
  const served = new FakeSessionsFleet(records(), { [PARENT]: thread() });
  served.startsNow = true;
  const loader = served.agentStarts(PARENT, LOADER);
  served.proposes(loader, { workflow: "bug", picked: ["workflow"], landing: { choice: "open_pr", from: "usual" } });
  const perRepo = served.agentStarts(PARENT, PER_REPO);
  served.proposes(perRepo, { reading: true });
  served.row(PARENT, { kind: "started", id: "a1", at: AT, session: { id: loader, title: LOADER.title }, fork: false });
  served.row(PARENT, { kind: "started", id: "a2", at: AT, session: { id: perRepo, title: PER_REPO.title }, fork: false });
  served.onQueued = (id) => {
    if (id !== perRepo) return;
    setTimeout(() => served.proposes(perRepo, { workflow: "feature", picked: ["workflow"], landing: { choice: "auto_land", from: "usual" } }), READS_MS);
  };
  return served;
}

function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  const served = fleet().scenario(board);
  return {
    ...served,
    name: "child-job-start",
    says: "A Job's agent started two Jobs; the parent's Needs you opens each and starts each, one still being read",
    // A Fleet of its own for each window, so a walk told twice starts from the same Jobs.
    behaves: (handle) => fleet().scenario(board).behaves!(handle),
  };
}

export const childJobStart: Scenario = build();
