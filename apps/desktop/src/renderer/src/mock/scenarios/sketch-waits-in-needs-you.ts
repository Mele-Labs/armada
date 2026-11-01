// A feature Job on Propose, its turn running, whose agent showed a sketch he has not approved. The Job's row calls while the turn
// goes on, as a draft does, and the sketch is pinned above the composer as a plan to approve is. Approve settles it on the ledger
// and takes the item off. The walk `sketch-waits-in-needs-you` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { arrow, box, scene } from "../scenes";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const TITLE = "Pause on the row";
const SKETCH = `sketch:${TITLE}`;
const EARLIER_TITLE = "Pause on the row, first take";
const EARLIER_SKETCH = `sketch:${EARLIER_TITLE}`;

const DRAWING = scene(
  [
    box("row", 0, 60, "Job row", "a Pause press at its end", { icon: "layers", step: 1 }),
    box("fleet", 320, 60, "Fleet", "holds the Drone's next turn", { icon: "server", step: 2 }),
    box("row-paused", 640, 60, "Job row", "Paused, with Resume", { icon: "lock", step: 3 }),
  ],
  [arrow("row", "fleet", "pause"), arrow("fleet", "row-paused", "held")],
);

const SESSION: Session = {
  id: "s47",
  title: "Pause a Job from its row",
  turn: { state: "working" },
  standing: "working",
  lastTurnAt: ago(1),
  work: { workflow: { name: "Feature", steps: ["Propose", "Discuss", "Approve plan", "Walk", "Refine", "Deliver"], at: 0 } },
  rows: [
    { id: "k1", at: "09:12:00", kind: "message", from: { kind: "you" }, text: "Let me pause a Job from its row on the Board." },
    { id: "k1a", at: "09:12:04", kind: "tool", text: "Grep apps/desktop/src/renderer/src/cockpit -e pause" },
    { id: "k1b", at: "09:12:09", kind: "tool", text: "Read apps/desktop/src/renderer/src/cockpit/deck/Deck.tsx" },
    { id: "k1c", at: "09:12:15", kind: "tool", text: "Read crates/fleet/src/session_host/pausing.rs" },
    { id: "k1d", at: "09:12:30", kind: "message", from: { kind: "agent" }, text: "Pause exists as a press in the Job's controls (`pausing.rs`). The Board row has no press for it." },
    { id: "k1e", at: "09:12:41", kind: "message", from: { kind: "you" }, text: "Put it at the end of the row, where the state glyph is." },
    { id: "k1f", at: "09:12:44", kind: "tool", text: "Read packages/components/src/compositions/DeckRow/DeckRow.tsx" },
    { id: "k1g", at: "09:12:50", kind: "tool", text: "Grep packages/components/src -e data-state" },
    { id: "k1h", at: "09:12:58", kind: "message", from: { kind: "agent" }, text: "The glyph column is fixed width, so a Pause press fits there without moving the title." },
    { id: "k1i", at: "09:13:05", kind: "message", from: { kind: "you" }, text: "And it should show Resume once it is paused." },
    { id: "k1j", at: "09:13:09", kind: "tool", text: "Read crates/fleet/src/session_host/pausing.rs" },
    { id: "k1k", at: "09:13:14", kind: "message", from: { kind: "agent" }, text: "Fleet already holds the next turn on Pause and lifts it on Resume, so the row only swaps the press." },
    { id: "k1l", at: "09:13:16", kind: "tool", text: `show_sketch ${JSON.stringify({ title: EARLIER_TITLE, scene: DRAWING })}` },
    { id: "k2", at: "09:13:20", kind: "tool", text: `show_sketch ${JSON.stringify({ title: TITLE, scene: DRAWING })}` },
  ],
  // Fleet sends the ledger newest first: the sketch drawn last is the first. Drawing it took back the earlier one under another title.
  attachments: [
    { kind: "sketch", id: SKETCH, title: TITLE, by: "agent", drawing: DRAWING, asked: true, drawn: "drawn-2" },
    { kind: "sketch", id: EARLIER_SKETCH, title: EARLIER_TITLE, by: "agent", drawing: DRAWING, asked: true, withdrawn: true, drawn: "drawn-1" },
  ],
  waitingFor: [{ id: SKETCH, text: TITLE, since: ago(1), source: "sketch", act: { kind: "message", target: "s47" } }],
};

function build(): Scenario {
  const base = holding("sketch-waits-in-needs-you", "A feature Job on Propose, its turn running, with a sketch he has not approved in its Needs you");
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
        };
      },
    },
  };
}

export const sketchWaitsInNeedsYou: Scenario = build();
