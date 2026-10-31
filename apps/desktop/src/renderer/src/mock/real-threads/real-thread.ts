// A real Job's thread, drawn in the new thread look: the scenario and the walk both come from one place, so the three
// `real-thread-*` mocks differ only in the Job they hold. The look, the quick bar and the Aa panel are `livelier-thread`'s.

import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";

import { button, deckRow, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";
import type { Scenario } from "../moment";
import { s200Sessions } from "../scenarios/sessions";

type RealThread = {
  /** The scenario's name, which the walk is over. */
  name: string;
  says: string;
  /** The Job's id as Fleet kept it. */
  id: string;
  title: string;
  branch: string;
  slot: number;
  rows: readonly SessionRow[];
  /** Short replies to the agent's last message. */
  asked: readonly string[];
};

export function realThreadScenario(one: RealThread): Scenario {
  const last = one.rows[one.rows.length - 1]!.at;
  const job = (): Session => ({
    id: one.id,
    title: one.title,
    turn: { state: "idle" },
    lastTurnAt: last,
    rows: [...one.rows],
    attachments: [{ kind: "slot", slot: one.slot }, { kind: "branch", name: one.branch, slot: one.slot }],
  });
  return {
    ...s200Sessions,
    name: one.name,
    says: one.says,
    draft: {
      sessions: (board) => s200Sessions.draft!.sessions!(board, [job()]),
      threadLook: {
        look: {
          agentName: "",
          showTimes: true,
          showAgentName: true,
          textSize: "base",
          quick: [
            { id: "learnt:done", text: "Just get it done", kind: "learnt" },
            { id: "learnt:walk", text: "Walk me through it", kind: "learnt" },
            { id: "saved:land", text: "Land it", kind: "saved" },
          ],
          learn: true,
          voice: "apple",
        },
        asked: { [one.id]: one.asked },
        heard: "Go ahead",
      },
    },
  };
}

/**
 * The walk over it: open the Job, a stretch with his bubbles, a stretch of commands, the quick bar and the Aa panel.
 * `commands` are words the thread's calls line draws.
 */
export function realThreadWalk(name: string, title: string, commands: RegExp) {
  const thread = region("Thread");
  const steps: readonly Step[] = [
    toSessions,
    { press: deckRow(title), say: "A real Job's thread in the new look" },
    { look: inside(thread, region("Your message")), say: "Your own words, as a bubble on the right" },
    { look: inside(thread, text(commands)), say: "Commands, with their glyph and time" },
    { look: role("group", "Quick replies"), say: "The quick bar sits above the box, with replies to the last message" },
    { press: button("Thread look"), say: "Aa: how this thread reads" },
  ];
  return walk(name, steps);
}
