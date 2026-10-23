// A Job asks a question whose options carry long descriptions, over the Cockpit with that one Job. The walk `jobLongQuestion` plays it: Back, the question and Answer stay in view in Needs you, and only the options scroll.

import type { Session } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;
const ROOT = repository().root;

const options = [
  {
    label: "Back off in Fleet",
    description:
      "Fleet asks for the bay again after a short wait that doubles each time, up to a cap of five tries. The Drone sees one lease call that takes longer, and nothing in its brief changes. The cap and the first wait become settings, and a Job that gives up says which bay it asked for and how long it waited. The wire types stay as they are, so Bridge needs no change.",
  },
  {
    label: "Back off in the Drone",
    description:
      "The Drone gets the refusal and decides when to ask again. Each Drone then carries its own wait and its own cap, and a Job's brief has to say so. A refused lease shows on the bay tile as a retry, which needs a new field on the wire and a change in Bridge. The Drone can give up early on a Job that has other work to do first.",
  },
  {
    label: "Queue the request",
    description:
      "Fleet holds the second Job in a queue per repository until a bay frees up. No Job gives up, and the order is the order they asked in. The queue has to survive a Fleet restart, so it is written to the store, and a Job waiting in it needs its own state on the row. A bay that never frees up leaves the Job waiting with no limit unless a timeout is added.",
  },
  {
    label: "Refuse and say so",
    description:
      "The second Job stays refused, as today, but the message names the Job that holds the bay and for how long. Nothing is retried. It is the smallest change and the easiest to remove, and it leaves the owner to dispatch the Job again by hand.",
  },
  {
    label: "Hand the bay to the second Job",
    description:
      "The first Job is asked to release its bay at its next commit, and the second takes it. This keeps every Job moving, but it interrupts work that was going well, and a release needs only a branch commit, so the first Job resumes on a new bay with its branch intact. The release is announced to both Jobs in one notice.",
  },
] as const;

const job: Session = {
  id: "long-1",
  title: "Retry the slot lease",
  manifestId: OWNER,
  cwd: ROOT,
  model: "opus",
  effort: "high",
  mode: "auto",
  turn: { state: "idle" },
  lastTurnAt: new Date(Date.now() - 2 * 60_000).toISOString(),
  standing: "idle",
  attachments: [],
  rows: [
    { id: "l-1", at: "13:20:00", kind: "message", from: { kind: "you" }, text: "Make the slot lease retry when two Jobs ask for the same bay at once." },
    { id: "l-2", at: "13:21:02", kind: "message", from: { kind: "agent" }, text: "Two Jobs can ask for the same bay between the read and the write. The second is refused and gives up at once. There are five ways to fix it." },
  ],
  asked: {
    command: "Where should the retry live?",
    call: "long-q1",
    offers: ["allow_once", "refuse"],
    questions: [{ question: "Where should the retry live?", header: "Lease", multi_select: false, options }],
  },
};

const base = holding("job-long-question", "A Job asks a question whose options have long descriptions");

export const s299JobLongQuestion: Scenario = {
  ...base,
  state: { ...base.state, repository: null },
  draft: { sessions: () => sessionsStore([job], [], new Map(), []) },
};
