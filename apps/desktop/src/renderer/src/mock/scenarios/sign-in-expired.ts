// A hosted Job whose turn failed because the Claude sign-in expired. Fleet raises one Needs you item on it,
// `Sign in to Claude`, a command to run. Run, it opens a terminal docked above the box: the browser sign-in,
// then its success line, and the item settles. Over the `sessions` scenario. The walk `sign-in-expired` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { startLogin, type Login } from "../terminal-run/claude-login";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const COMMAND = "claude auth login";
const ITEM = "signin:2026-10-25T09:14:02Z";

const failed: Session = {
  id: "s70",
  title: "Add the billing page",
  turn: { state: "idle" },
  lastTurnAt: minutesAgo(2),
  rows: [
    { id: "f1", at: "09:13:40", kind: "message", from: { kind: "you" }, text: "Add the billing page." },
    { id: "f2", at: "09:14:02", kind: "message", from: { kind: "agent" }, text: "Failed to authenticate: OAuth session expired and could not be refreshed" },
  ],
  attachments: [{ kind: "slot", slot: 3 }, { kind: "branch", name: "feat/billing-page", slot: 3 }],
  waitingFor: [{ id: ITEM, text: "Sign in to Claude", since: minutesAgo(2), source: "fleet_call", act: { kind: "run", target: COMMAND } }],
};

function sessionsOf(board: Parameters<NonNullable<NonNullable<Scenario["draft"]>["sessions"]>>[0]) {
  const store = s200Sessions.draft!.sessions!(board, [failed]);
  const running = new Set<Login>();
  return {
    ...store,
    terminalRuns: true as const,
    run: (id: string, command: string, on: Parameters<NonNullable<typeof store.run>>[2]) => {
      if (command !== COMMAND) return store.run!(id, command, (event) => on(event.kind === "out" ? { kind: "out", text: event.text.replaceAll("\n", "\r\n") } : event));
      const program = startLogin(
        command,
        (text) => on({ kind: "out", text }),
        (code) => {
          running.delete(program);
          on({ kind: "end", code });
        },
      );
      running.add(program);
      return { stop: program.stop, write: program.write };
    },
    // Settling the item is all this run owes.
    answerWaiting: (id: string, itemId: string, given: Parameters<NonNullable<typeof store.answerWaiting>>[2]) =>
      itemId === ITEM
        ? store.edit(id, (one) => ({ ...one, waitingFor: (one.waitingFor ?? []).filter((it) => it.id !== itemId) }))
        : store.answerWaiting?.(id, itemId, given),
    dispose: () => {
      running.forEach((one) => one.stop());
      store.dispose();
    },
  };
}

export const s257SignInExpired: Scenario = {
  ...s200Sessions,
  name: "sign-in-expired",
  says: "A hosted Job that failed on an expired Claude sign-in, with a Sign in to Claude command to run",
  draft: { sessions: sessionsOf },
};
