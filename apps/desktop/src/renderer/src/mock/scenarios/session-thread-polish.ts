// A terminal Session whose thread holds what a real one does: calls run one after another, a slash
// command, a compaction summary and plain messages. The walk `session-thread-polish` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { COMMANDS } from "../sessions/script";
import { s200Sessions } from "./sessions";

const polished: Session = {
  id: "s4",
  terminal: true,
  title: "CI timeout hunt",
  model: "sonnet",
  effort: "medium",
  mode: "ask",
  commands: COMMANDS,
  turn: { state: "idle" },
  lastTurn: "14:02",
  rows: [
    { id: "p1", at: "13:58:00", kind: "compaction", text: "This session is being continued from a previous conversation that ran out of context. The summary covers the store test that fails only in CI." },
    { id: "p2", at: "14:00:40", kind: "command", text: "/reload-plugins" },
    { id: "p3", at: "14:01:12", kind: "message", from: { kind: "you" }, text: "Why does the store test fail only in CI?" },
    { id: "p4", at: "14:01:20", kind: "tool", text: "Grep flaky in crates/store/src/tests" },
    { id: "p5", at: "14:01:31", kind: "tool", text: "Read crates/store/src/tests/ledger.rs" },
    { id: "p6", at: "14:01:40", kind: "tool", text: "Bash cat /private/tmp/claude-501/store-ci/out.txt" },
    { id: "p7", at: "14:02:03", kind: "message", from: { kind: "agent" }, text: "It sleeps 50 ms and then reads the clock. CI is slower than that, so the read lands before the write." },
    { id: "p8", at: "14:02:30", kind: "tool", text: "Edit crates/store/src/tests/ledger.rs" },
    { id: "p9", at: "14:02:50", kind: "message", from: { kind: "you" }, text: "Wait on the write instead of sleeping, and keep the test under a second." },
  ],
  attachments: [],
};

export const s203SessionThreadPolish: Scenario = {
  ...s200Sessions,
  name: "session-thread-polish",
  says: "A terminal Session's thread: grouped calls, a command, a compaction, messages with no bubble",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [polished]) },
};
