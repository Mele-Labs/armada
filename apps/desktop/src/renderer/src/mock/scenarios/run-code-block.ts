// A Session whose agent message holds shell blocks the person can run from the thread: a `bash` one-liner,
// a multi-line `sh` block that fails, and a `console` block that never ends until it is stopped. Over the
// `sessions` scenario. The walk `runCodeBlock` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const TEXT = [
  "Typecheck the desktop app first:",
  "",
  "```bash",
  "pnpm -C apps/desktop typecheck",
  "```",
  "",
  "Then the components tests:",
  "",
  "```sh",
  "cd packages/components",
  "pnpm vitest run",
  "```",
  "",
  "And watch the build log while it settles:",
  "",
  "```console",
  "$ tail -f build.log",
  "```",
].join("\n");

const running: Session[] = [
  {
    id: "s13",
    title: "Verify the build",
    turn: { state: "idle" },
    lastTurnAt: minutesAgo(1),
    rows: [
      { id: "r1", at: "14:39:50", kind: "message", from: { kind: "you" }, text: "What do I run to check the build?" },
      { id: "r2", at: "14:40:02", kind: "message", from: { kind: "agent" }, text: TEXT },
    ],
    attachments: [],
  },
];

export const s205RunCodeBlock: Scenario = {
  ...s200Sessions,
  name: "run-code-block",
  says: "A Session whose agent message holds shell blocks, run from the thread: passing, failing, stopped",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, running) },
};
