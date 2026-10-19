// A Session in a terminal stopped on its harness's own permission prompt, which nobody is at the
// terminal to answer. Its mod put the prompt to Fleet, so it is a call in the Cockpit, answered from
// Bridge as a hosted Session's is: ▶ on its command is Allow once. The walk `terminalPermissionCall` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const SESSION: Session = {
  id: "t7",
  title: "Port the cache to sqlite",
  terminal: true,
  turn: { state: "working" },
  lastTurnAt: ago(6),
  rows: [
    { id: "t1", at: "21:48:02", kind: "message", from: { kind: "you" }, text: "Port the manifest cache to sqlite and build it." },
    { id: "t2", at: "21:52:40", kind: "message", from: { kind: "agent" }, text: "The store compiles. Building the workspace next." },
  ],
  attachments: [],
  asked: { command: "cargo build --release", call: "c7", offers: ["allow_once", "refuse"] },
  waitingFor: [
    {
      id: "perm:c7",
      text: "Bash cargo build --release",
      since: ago(5),
      source: "permission",
      act: { kind: "answer", target: "c7" },
      options: [{ label: "Allow once" }, { label: "Refuse" }],
    },
  ],
};

function build(): Scenario {
  const base = holding("terminal-permission-call", "The Cockpit with a terminal Session stopped on its own permission prompt");
  return {
    ...base,
    state: { ...base.state, repository: repository().root },
    draft: { sessions: () => sessionsStore([SESSION]) },
  };
}

export const s246TerminalPermissionCall: Scenario = build();
