// A terminal Session whose agent message holds a fenced block with no language that reads as shell (a
// `&&` chain), two that do not (a code snippet, a diagram), and slash commands in inline code. Over the
// `sessions` scenario. The walk `runUntaggedAndSlash` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const TEXT = [
  "The slot is merged, so release it and drop the branch:",
  "",
  "```",
  "cd ~/Development/armada && \\",
  "  rm -r .armada/slots/slot-3/target && \\",
  "  armada worktree release slot-3 && \\",
  "  git branch -D sessions/old-spike",
  "```",
  "",
  "The filter I changed reads like this:",
  "",
  "```",
  "const rows = items.filter((it) => it.ok);",
  "const total = rows.length;",
  "```",
  "",
  "And how the pieces connect:",
  "",
  "```",
  "┌────────┐     ┌───────┐     ┌───────┐",
  "│ Bridge │────▶│ Fleet │────▶│ Agent │",
  "└────────┘     └───────┘     └───────┘",
  "```",
  "",
  "Then run `/update-armada` and, once it is done, `/reload-plugins`. Scratch files live in `/tmp`.",
].join("\n");

const running: Session[] = [
  {
    id: "s14",
    title: "Clean up the merged slot",
    terminal: true,
    turn: { state: "idle" },
    lastTurnAt: minutesAgo(1),
    rows: [
      { id: "r1", at: "14:39:50", kind: "message", from: { kind: "you" }, text: "That branch merged. What do I do now?" },
      { id: "r2", at: "14:40:02", kind: "message", from: { kind: "agent" }, text: TEXT },
    ],
    attachments: [],
    // What the terminal lists: `/tmp` is not one, so it is drawn as plain code.
    commands: [
      { name: "update-armada", says: "Move Fleet, Bridge and the mod onto latest main" },
      { name: "reload-plugins", says: "Reload the plugins" },
      { name: "model", says: "Set the model" },
    ],
  },
];

export const s206RunUntaggedAndSlash: Scenario = {
  ...s200Sessions,
  name: "run-untagged-and-slash",
  says: "A terminal Session whose message holds an untagged shell block, a code block, a diagram, and slash commands",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, running) },
};
