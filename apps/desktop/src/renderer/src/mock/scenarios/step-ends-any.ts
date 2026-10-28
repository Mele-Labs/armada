// A feature Job on Propose whose agent showed a sketch and put it to him. Approve settles the card and the Job moves on to
// Discuss; Send back settles it with his note. The press settles the sketch in the store, as Fleet's
// `POST /sessions/sketch/answer` settles it on the ledger. The Workflows page's feature Propose ends on any of four (`workflows-managed.ts`). The walk `step-ends-any` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { arrow, box, scene } from "../scenes";
import { s200Sessions } from "./sessions";

const JOB = "pause-from-row";
const TITLE = "Pause on the row";
const SKETCH = `sketch:${TITLE}`;
const STEPS = ["Propose", "Discuss", "Approve plan", "Walk", "Refine", "Deliver"];

const DRAWING = scene(
  [
    box("row", 0, 60, "Job row", "a Pause press at its end", { icon: "layers", step: 1 }),
    box("fleet", 320, 60, "Fleet", "holds the Drone's next turn", { icon: "server", step: 2 }),
    box("row-paused", 640, 60, "Job row", "Paused, with Resume", { icon: "lock", step: 3 }),
  ],
  [arrow("row", "fleet", "pause"), arrow("fleet", "row-paused", "held")],
);

const job: Session = {
  id: JOB,
  title: "Pause a Job from its row",
  turn: { state: "idle" },
  standing: "replied",
  lastTurnAt: new Date().toISOString(),
  work: { workflow: { name: "Feature", steps: STEPS, at: 0 } },
  rows: [
    { id: "p-1", at: "09:12:00", kind: "message", from: { kind: "you" }, text: "Let me pause a Job from its row on the Board." },
    { id: "p-2", at: "09:13:20", kind: "tool", text: `show_sketch ${JSON.stringify({ title: TITLE, scene: DRAWING })}` },
    { id: "p-3", at: "09:13:30", kind: "message", from: { kind: "agent" }, text: "A Pause press at the end of the row. Fleet holds the next turn, and the row says Paused with Resume." },
  ],
  attachments: [{ kind: "sketch", id: SKETCH, title: TITLE, by: "agent", drawing: DRAWING, asked: true, drawn: "drawn-1" }],
};

export const stepEndsAny: Scenario = {
  ...s200Sessions,
  name: "step-ends-any",
  says: "A feature Job on Propose with a sketch put to him: Approve moves it to Discuss, Send back takes a note",
  draft: {
    sessions: (board) => {
      const store = s200Sessions.draft!.sessions!(board, [job]);
      return {
        ...store,
        answerSketch: ({ session_id: id, sketch, how, words }) => {
          const at = new Date().toISOString();
          const settled = how === "sent_back" ? { how, words: words ?? "", at } : { how, at };
          store.edit(id, (one) => ({
            ...one,
            attachments: one.attachments.map((a) => (a.kind === "sketch" && a.id === sketch ? { ...a, settled } : a)),
            rows: [
              ...one.rows,
              {
                id: "p-4",
                at: "09:15:00",
                kind: "message",
                from: { kind: "agent" },
                text: settled.how === "approved" ? "On to Discuss." : "I will redraw it with your note.",
              },
            ],
            ...(settled.how === "approved" && one.work?.workflow !== undefined ? { work: { ...one.work, workflow: { ...one.work.workflow, at: 1 } } } : {}),
          }));
        },
      };
    },
  },
};
