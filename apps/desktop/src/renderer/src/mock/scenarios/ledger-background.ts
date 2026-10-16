// A Session whose agent has things running in the background: two shell commands and a monitor still
// running, and two ended, one that exited 0 and one that was stopped. Over the `sessions` scenario.
// The walk `ledger-background` plays it.

import type { Session, SessionAttachment } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const background = (
  id: string,
  form: "command" | "monitor",
  task: string,
  command: string,
  started: number,
  rest: Partial<Extract<SessionAttachment, { kind: "background" }>>,
): SessionAttachment => ({
  kind: "background",
  id,
  form,
  task,
  command,
  started: minutesAgo(started),
  state: "running",
  output: `/tmp/armada/tasks/${id}.output`,
  ...rest,
});

const holding: Session = {
  id: "s73",
  terminal: true,
  title: "Serve the Bridge mock",
  turn: { state: "idle" },
  lastTurn: "14:02",
  lastTurnAt: minutesAgo(2),
  rows: [],
  attachments: [
    { kind: "branch", name: "ledger/background", slot: 6 },
    background("serve-mock", "command", "Serve the Bridge mock from slot 6", "pnpm --filter @armada/desktop exec vite --port 5176 --host 127.0.0.1", 18, {}),
    background("watch-tests", "command", "Rerun the session tests on save", "pnpm --filter @armada/desktop exec vitest watch src/renderer/src/sessions", 12, {}),
    background("watch-ci", "monitor", "Watch the CI run for #1862", "gh run watch 9321177 --exit-status", 7, {}),
    background("build-renderer", "command", "Build the Bridge renderer", "pnpm --filter @armada/desktop build", 41, { state: "done", exit: 0, ended: minutesAgo(40) }),
    background("tail-fleet", "command", "Follow the Fleet log", "tail -f ~/Library/Logs/Armada/fleet.log", 33, { state: "stopped", ended: minutesAgo(25) }),
  ],
};

export const s266LedgerBackground: Scenario = {
  ...s200Sessions,
  name: "ledger-background",
  says: "A Session with two background commands and a monitor running, and two ended: one that exited 0 and one stopped",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [holding]) },
};
