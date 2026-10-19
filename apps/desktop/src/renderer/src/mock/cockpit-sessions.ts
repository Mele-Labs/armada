// The Cockpit's Sessions on `dashboard-cockpit`, which `bays-outside-prs` builds on. Kept out of
// `scenarios/`, where every export is read as a scenario row.

import type { Session } from "@armada/screens/src/draft/sessions";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const quiet = { state: "idle" } as const;
const said = (id: string, text: string) => [{ id, at: "14:00:00", kind: "message", from: { kind: "agent" }, text } as const];

/** Two at work, one that will ask for a command, two idle beside them. */
export const COCKPIT_SESSIONS: Session[] = [
  { id: "s9", title: "Flaky store test", turn: { state: "working" }, lastTurnAt: ago(2), rows: [
      { id: "s9-u", at: "13:58:00", kind: "message", from: { kind: "you" }, text: "The store test is flaky on CI. Find out why and fix it, then push the branch when it is green." },
      { id: "s9-t", at: "13:59:10", kind: "tool", text: "Read crates/store/tests/store_test.rs" },
      ...said("s9-0", "The flake is the 200ms sleep in store_test.rs. Replacing it with a wait on the channel."),
    ], attachments: [{ kind: "branch", name: "fix/flaky-store", slot: 3 }] },
  { id: "s10", title: "Migration notes", turn: { state: "working" }, lastTurnAt: ago(1), rows: said("s10-0", "Reading the migrations since 0042."), attachments: [] },
  { id: "s11", title: "Release script", dead: "ended", turn: quiet, lastTurnAt: ago(150), rows: said("s11-1", "Done."), attachments: [] },
  { id: "s12", title: "Theme token audit", turn: quiet, lastTurnAt: ago(14), rows: said("s12-0", "Eleven tokens have no caller."), attachments: [] },
];
