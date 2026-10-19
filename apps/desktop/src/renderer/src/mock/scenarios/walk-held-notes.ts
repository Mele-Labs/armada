// Notes held on a Job's walk and sent as one turn — the owner, 19 Oct 2026: "leave batch annotations on a walk and then
// submit them back to the agent all at once". One Job's walk is in the walk window with its view beside the page, and a
// second Job is there to open on the Cockpit, which stays put while notes are taken. Draft data, nothing here reaches Fleet.
// The walk `walkHeldNotes` plays it. `docs/practices/capture-window.md`, *Held notes*.

import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";
import { closeWalkWindow, forgetHeldNotes, mockPage, queueWalk } from "@armada/screens/src/fixtures/walk-window";

import { askToOpenSession } from "../../open-session";
import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const OWNER = repository().manifest!.id;
const ROOT = repository().root;

const you = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "you" }, text });
const agent = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "agent" }, text });

/** The mock itself on another scenario, as the page the Job showed. */
const page = (scenario: string): string => {
  const url = new URL(mockPage());
  url.search = `?scenario=${scenario}&walked`;
  return url.toString();
};

const backoff: Session = {
  id: "hn-backoff",
  title: "Retry backoff cap",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "idle" },
  lastTurn: "14:02",
  lastTurnAt: ago(6),
  standing: "idle",
  rows: [
    you("hb-1", "13:50:00", "Cap the retry backoff at thirty seconds."),
    agent("hb-2", "14:02:10", "The cap is in. The backoff table is in a window."),
  ],
  attachments: [{ kind: "branch", name: "fleet/retry-backoff-cap", slot: 2 }, { kind: "artifact", form: "window", id: page("default"), title: "Backoff table" }],
};

const clock: Session = {
  id: "hn-clock",
  title: "Pin the store clock",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "idle" },
  lastTurn: "14:04",
  lastTurnAt: ago(4),
  standing: "idle",
  rows: [you("hc-1", "13:40:00", "Pin the clock in the store tests."), agent("hc-2", "14:04:30", "Every test reads the pinned clock.")],
  attachments: [{ kind: "branch", name: "fix/pin-store-clock", slot: 4 }],
};

function sessions() {
  const store = sessionsStore([backoff, clock], [], new Map([[ROOT, OWNER]]));
  const told = (id: string, text: string) => store.send(id, { text, files: [], sketches: [], tags: [] });
  if (typeof document !== "undefined") {
    closeWalkWindow();
    forgetHeldNotes();
    queueWalk(`${backoff.id}:1`, "Backoff table", page("default"), {
      title: "Backoff table",
      job: backoff.title!,
      into: "Session",
      entry: { name: backoff.title! },
      since: ago(10),
      panelFor: backoff.id,
      onOpenOwner: () => askToOpenSession(backoff.id),
      onNote: () => {},
      // One turn, the thread keeping the words alone, numbered in the order taken, as `send_session_notes` does.
      onSendNotes: async (notes) => {
        await told(backoff.id, notes.map((one, at) => `${at + 1}. ${one.said}`).join("\n"));
        return { ok: true };
      },
      onApprove: async (where) => {
        await told(backoff.id, `Approved: ${where}`);
        return { ok: true };
      },
      readiness: "ready",
    });
  }
  return store;
}

export const s298WalkHeldNotes: Scenario = {
  ...holding("walk-held-notes", "Notes taken on a Job's walk are held in its panel, edited or removed there, and sent to the Job as one turn"),
  draft: { sessions },
};
