// A Session whose agent message holds two fenced blocks: a `bash` command, which gains Copy beside Run, and a
// block with no language that is not a command, which gains Copy alone. The clipboard is the walk's own, so
// the machine's is not touched. Over the `sessions` scenario. The walk `copy-from-a-session` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { useWalkClipboard } from "../sketch-share-view";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const TEXT = [
  "Run the one test that fails:",
  "",
  "```bash",
  "cargo test -p store store_open",
  "```",
  "",
  "It reads the clock twice, a second apart when it fails:",
  "",
  "```",
  "read 1 -> 12:00:00.99",
  "read 2 -> 12:00:01.01",
  "```",
].join("\n");

const copying: Session[] = [
  {
    id: "s14",
    title: "Find the flaky test",
    turn: { state: "idle" },
    lastTurnAt: minutesAgo(1),
    rows: [
      { id: "r1", at: "14:39:50", kind: "message", from: { kind: "you" }, text: "Why does store_open fail now and then?" },
      { id: "r2", at: "14:40:02", kind: "message", from: { kind: "agent" }, text: TEXT },
    ],
    attachments: [],
  },
];

export const s255CopyFromASession: Scenario = {
  ...s200Sessions,
  name: "copy-from-a-session",
  says: "A Session whose agent message holds a command and a plain block, each with Copy",
  behaves: (fleet) => {
    useWalkClipboard();
    return s200Sessions.behaves?.(fleet) ?? {};
  },
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, copying) },
};
