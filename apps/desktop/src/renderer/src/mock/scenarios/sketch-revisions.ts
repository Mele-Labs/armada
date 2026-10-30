// A feature Job whose agent drew one sketch three times under one title, and put the third to him. Fleet keeps each drawing as a
// file, `r1.json` to `r3.json`, and the pin and the thread's newest card offer each one. The walk `sketch-revisions` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { arrow, box, scene } from "../scenes";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const TITLE = "Pause on the row";
const SKETCH = `sketch:${TITLE}`;
const KEPT = "/armada/home/sketches/s48/pause-on-the-row";

const row = box("row", 0, 60, "Job row", "a Pause press at its end", { icon: "layers", step: 1 });
const fleet = box("fleet", 320, 60, "Fleet", "holds the Drone's next turn", { icon: "server", step: 2 });
const paused = box("paused", 640, 60, "Paused row", "Resume where Pause was", { icon: "lock", step: 3 });

/** The first drawing: the press and Fleet. */
const R1 = scene([row, fleet], [arrow("row", "fleet", "pause")]);
/** The second adds the row as it reads once paused. */
const R2 = scene([row, fleet, paused], [arrow("row", "fleet", "pause"), arrow("fleet", "paused", "held")]);
/** The third says what Fleet holds, and adds Resume. */
const R3 = scene(
  [row, { ...fleet, body: "holds the next turn until Resume" }, paused, box("resume", 640, 230, "Resume", "lifts the hold", { icon: "zap", step: 4 })],
  [arrow("row", "fleet", "pause"), arrow("fleet", "paused", "held"), arrow("paused", "resume", "resume")],
);

const DRAWN = [R1, R2, R3] as const;
const PATHS = DRAWN.map((_, at) => `${KEPT}/r${String(at + 1)}.json`);
const call = (drawing: (typeof DRAWN)[number]) => `show_sketch ${JSON.stringify({ title: TITLE, scene: drawing })}`;

const SESSION: Session = {
  id: "s48",
  title: "Pause a Job from its row",
  turn: { state: "working" },
  standing: "working",
  lastTurnAt: ago(1),
  work: { workflow: { name: "Feature", steps: ["Propose", "Discuss", "Approve plan", "Walk", "Refine", "Deliver"], at: 0 } },
  rows: [
    { id: "v1", at: "09:12:00", kind: "message", from: { kind: "you" }, text: "Let me pause a Job from its row on the Board." },
    { id: "v1a", at: "09:12:30", kind: "message", from: { kind: "agent" }, text: "Pause exists in the Job's controls. The Board row has no press for it." },
    { id: "v2", at: "09:12:40", kind: "tool", text: call(R1) },
    { id: "v3", at: "09:13:05", kind: "message", from: { kind: "you" }, text: "And it should show Resume once it is paused." },
    { id: "v4", at: "09:13:20", kind: "tool", text: call(R2) },
    { id: "v5", at: "09:13:50", kind: "message", from: { kind: "you" }, text: "Say what Fleet holds, and draw Resume." },
    { id: "v6", at: "09:14:10", kind: "tool", text: call(R3) },
  ],
  attachments: [{ kind: "sketch", id: SKETCH, title: TITLE, by: "agent", drawing: R3, asked: true, drawn: "drawn-3", revisions: PATHS }],
  waitingFor: [{ id: SKETCH, text: TITLE, since: ago(1), source: "sketch", act: { kind: "message", target: "s48" } }],
};

function build(): Scenario {
  const base = holding("sketch-revisions", "A feature Job whose sketch was drawn three times under one title, the third waiting for his approval");
  return {
    ...base,
    state: { ...base.state, repository: repository().root },
    draft: {
      sessions: () => {
        const store = sessionsStore([SESSION]);
        return {
          ...store,
          // Fleet settles the sketch on its ledger and takes the item off when he answers it.
          answerSketch: ({ session_id: id, sketch, how, words }) => {
            const at = new Date().toISOString();
            const settled = how === "sent_back" ? { how, words: words ?? "", at } : { how, at };
            store.edit(id, (one) => ({
              ...one,
              attachments: one.attachments.map((a) => (a.kind === "sketch" && a.id === sketch ? { ...a, settled } : a)),
              waitingFor: (one.waitingFor ?? []).filter((item) => item.id !== sketch),
            }));
          },
          // Each revision file holds the scene's JSON text, as Fleet writes it.
          readArtifact: async (id: string, path: string) => {
            const drawing = DRAWN[PATHS.indexOf(path)];
            return drawing === undefined ? (store.readArtifact?.(id, path) ?? { ok: false as const, why: "unreadable" as const }) : { ok: true as const, bytes: new TextEncoder().encode(JSON.stringify(drawing)), type: "application/json" };
          },
        };
      },
    },
  };
}

export const sketchRevisions: Scenario = build();
