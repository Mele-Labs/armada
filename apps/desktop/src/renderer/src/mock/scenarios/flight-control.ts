// The flight deck with two Jobs, for the walk `flight-control`: `flight-controls-custom`'s Job, with subagents, Checks and a bay, so most keys and views are live, and a Job just begun, which
// holds nothing and has run nothing, so most are off. The owner's ask of 25 Oct 2026.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import type { SessionsStore } from "../sessions/script";
import { flightControlsCustom } from "./flight-controls-custom";

const ARMADA = repository().manifest!.id;
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const clock = (seconds: number): string => new Date(Date.now() - seconds * 1000).toTimeString().slice(0, 8);

const BEGUN: Session = {
  id: "fc2",
  title: "Draft the release notes",
  manifestId: ARMADA,
  model: "opus",
  effort: "high",
  mode: "auto",
  turn: { state: "idle" },
  standing: "idle",
  lastTurnAt: ago(2),
  context: { tokens: 12_000, window: 200_000 },
  work: { workflow: { name: "Feature", steps: ["Plan the change", "Implement", "Review the change"], at: 0 } },
  rows: [{ id: "fc2-u", at: clock(120), kind: "message", from: { kind: "you" }, text: "Draft the release notes for this week." }],
  attachments: [],
};

export const flightControl: Scenario = {
  ...flightControlsCustom,
  name: "flight-control",
  says: "The flight deck with a Job whose keys and views are mostly live and a Job just begun whose are mostly off, for the walk of Flight control",
  draft: {
    ...flightControlsCustom.draft,
    sessions: (control, more = []) => flightControlsCustom.draft!.sessions!(control, [...more, BEGUN]) as SessionsStore,
  },
};
