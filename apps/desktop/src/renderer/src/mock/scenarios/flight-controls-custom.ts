// The flight deck with one Session whose tiles have facts to show, for the walk `flight-controls-custom`: its context window is 84% full,
// two subagents and two Checks are running, it holds slot 7 with a branch and an open pull request, and it can be paused. The ship's other
// Sessions are as they are. The tiles are edited in place, and what is kept stands in for `~/.armada/flight-controls.toml`
// (`cockpit/deck/flight-controls.ts`). The owner's ask of 18 Oct 2026.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import type { SessionsStore } from "../sessions/script";
import { s252CockpitShipFlightdeck as ship } from "./cockpit-ship";

const ARMADA = repository().manifest!.id;
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const clock = (seconds: number): string => new Date(Date.now() - seconds * 1000).toTimeString().slice(0, 8);

const SLOT = 7;
const BRANCH = "mock/flight-controls-custom";

const TUNE: Session = {
  id: "fc1",
  title: "Tune the flight controls",
  manifestId: ARMADA,
  model: "opus",
  effort: "high",
  mode: "auto",
  turn: { state: "working" },
  standing: "working",
  lastTurn: clock(20).slice(0, 5),
  lastTurnAt: ago(1),
  context: { tokens: 168_000, window: 200_000 },
  // A Job as every Job is now, so the deck draws the unified layout around it.
  work: { workflow: { name: "Feature", steps: ["Plan the change", "Implement", "Review the change"], at: 1 } },
  rows: [
    { id: "fc-u", at: clock(900), kind: "message", from: { kind: "you" }, text: "Make the flight controls editable." },
    { id: "fc-t", at: clock(300), kind: "tool", text: "Edit apps/desktop/src/renderer/src/cockpit/deck/SessionTiles.tsx" },
    { id: "fc-c1", at: clock(200), kind: "check", name: "desktop:typecheck", run: 41, state: "running" },
    { id: "fc-c2", at: clock(120), kind: "check", name: "desktop_test", run: 42, state: "running", target: "flight-controls" },
    { id: "fc-c3", at: clock(600), kind: "check", name: "lint", run: 40, state: "passed" },
  ],
  attachments: [
    { kind: "slot", slot: SLOT },
    { kind: "branch", name: BRANCH, slot: SLOT },
    { kind: "pull_request", number: 1901, title: "Make the flight controls editable", branch: BRANCH, address: "https://git.example/armada/pull/1901", checks: { state: "pending" }, state: "open", auto: false },
    { kind: "subagent", id: "fc-sa1", task: "Read the deck's key handler", state: "running", agentType: "explorer" },
    { kind: "subagent", id: "fc-sa2", task: "Check the tiles against the registry", state: "running", agentType: "armada:implementer" },
    { kind: "subagent", id: "fc-sa3", task: "List the lucide names", state: "done", agentType: "explorer" },
  ],
};

function build(): Scenario {
  return {
    ...ship,
    name: "flight-controls-custom",
    says: "The flight deck with a Session whose tiles carry facts: its context fill, two subagents and two Checks running, a bay, and Pause. Edit changes the tiles in place",
    held: {
      slots: [
        ...(ship.held?.slots ?? []),
        {
          manifest_id: ARMADA,
          slot: SLOT,
          path: `/Users/user/armada/.armada/slots/slot-${SLOT}`,
          base: "main",
          warm: true,
          behind: 0,
          held: { state: "hosted_session", session_id: TUNE.id, session_title: TUNE.title ?? TUNE.id },
          branch: BRANCH,
          since: ago(20),
        },
      ],
    },
    draft: {
      ...ship.draft,
      sessions: (control, more = []) => {
        const store = ship.draft!.sessions!(control, [...more, TUNE]) as SessionsStore;
        return {
          ...store,
          // Pause parks the turn and keeps the bay on show; Resume starts it again.
          pause: async (id) => {
            store.edit(id, ({ paused: _, ...one }) => ({ ...one, paused: true, turn: { state: "idle" }, standing: "idle" }));
            return true;
          },
          resume: async (id) => {
            store.edit(id, ({ paused: _, ...one }) => ({ ...one, turn: { state: "working" }, standing: "working" }));
            return true;
          },
        };
      },
    },
  };
}

export const flightControlsCustom: Scenario = build();
