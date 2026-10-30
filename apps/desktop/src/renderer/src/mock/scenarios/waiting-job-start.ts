// A Job that waits on another and is not started: its page offers Then, Workflow, Model and Effort beside when it starts and from where.
// Over a Job, `Sketch Approval Needs You`, that has yet to land. The walk `waitingJobStart` plays it.

import type { Session, SessionWorkActs } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;
const ROOT = repository().root;

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const SKETCH = "waiting-sketch";
const DOCUMENT = "waiting-document";

const sketch: Session = {
  id: SKETCH,
  title: "Sketch Approval Needs You",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "working" },
  lastTurnAt: ago(2),
  standing: "working",
  rows: [{ id: "w-1", at: "13:50:00", kind: "message", from: { kind: "you" }, text: "Make a sketch approval need the owner." }],
  attachments: [{ kind: "branch", name: "armada/sketch-approval", slot: 1 }],
};

const document: Session = {
  id: DOCUMENT,
  title: "Document Approval",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "idle" },
  lastTurnAt: ago(1),
  standing: "idle",
  rows: [],
  attachments: [],
  waits: { on: { id: SKETCH, title: sketch.title! }, when: "lands", base: "main", needs: "", brief: "Do the same for a document approval.", by: "you" },
};

// The workflows a Job may follow, which the Workflow select offers; nothing here acts on them.
const work = {
  workflows: [
    { id: "feature", name: "Feature", steps: ["Plan", "Build", "Pull request"], manifestId: OWNER },
    { id: "bug", name: "Bug", steps: ["Reproduce", "Fix", "Pull request"], manifestId: OWNER },
  ],
  choose: () => {},
  unpick: () => {},
  approve: () => {},
  suggest: () => {},
  sendConcern: () => {},
  dismissConcern: () => {},
} satisfies SessionWorkActs;

function build(): Scenario {
  const base = holding("waiting-job-start", "A Job that waits on another: what it does when its condition is met, and the workflow, model and effort it starts on");
  return {
    ...base,
    state: { ...base.state, repository: ROOT },
    draft: { sessions: () => ({ ...sessionsStore([sketch, document], [], new Map([[ROOT, OWNER]])), work }) },
  };
}

export const waitingJobStart: Scenario = build();
