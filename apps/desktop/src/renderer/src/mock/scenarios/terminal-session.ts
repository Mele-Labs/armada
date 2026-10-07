// A Session run in a terminal, opened in Bridge, beside the `sessions` scenario's own. Its thread is
// long, so the walk lands at the newest message. The walk `terminal-session` plays it.

import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { COMMANDS } from "../sessions/script";
import { s200Sessions } from "./sessions";

const idle = { state: "idle" } as const;

const terminal: Session = {
  id: "s4",
  terminal: true,
  title: "CI timeout hunt",
  model: "sonnet",
  effort: "medium",
  mode: "ask",
  commands: COMMANDS,
  turn: idle,
  lastTurn: "14:02",
  rows: [
    ...Array.from({ length: 14 }, (_, at): SessionRow => ({ id: `s4-old${at}`, at: `13:${40 + at}:00`, kind: at % 2 === 0 ? "tool" : "message", ...(at % 2 === 0 ? { text: `Read crates/store/src/part${at}.rs` } : { from: { kind: "agent" as const }, text: `Part ${at} reads the same clock.` }) }) as SessionRow),
    { id: "s4-1", at: "14:01:12", kind: "message", from: { kind: "you" }, text: "Why does the store test fail only in CI?" },
    { id: "s4-2", at: "14:01:20", kind: "tool", text: "Grep flaky in crates/store/src/tests" },
    { id: "s4-3", at: "14:01:31", kind: "tool", text: "Read crates/store/src/tests/ledger.rs" },
    { id: "s4-4", at: "14:02:03", kind: "message", from: { kind: "agent" }, text: "It sleeps 50 ms and then reads the clock. CI is slower than that." },
  ],
  attachments: [],
};

export const s202TerminalSession: Scenario = {
  ...s200Sessions,
  name: "terminal-session",
  says: "A Session run in a terminal: its thread, model, effort and commands",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [terminal]) },
};
