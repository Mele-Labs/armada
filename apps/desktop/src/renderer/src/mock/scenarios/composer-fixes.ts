// Three fixes to a terminal Session: the `/` list laid out as a column and filtered by what is typed,
// a refused send raised as a toast, and a mark on a Session whose mod is out of date. The walk
// `composer-fixes` plays it.

import type { Session, SessionCommand } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { NOT_REACHABLE, askToTell } from "../../tell";
import { s200Sessions } from "./sessions";

const idle = { state: "idle" } as const;

const NAMES = ["review", "remember", "reload-plugins", "resume", "rename", "refactor", "release-notes", "pre-commit", "security-review"];
/** A terminal lists skills and commands by the hundred, and a few of them start with `re`. */
const COMMANDS: readonly SessionCommand[] = [
  ...NAMES.map((name) => ({ name, says: `What ${name} does, said at some length so that the line has to be cut short` })),
  ...Array.from({ length: 117 }, (_, at): SessionCommand => ({ name: `${["cmd", "plugin", "skill"][at % 3]}-${at}`, says: `Command ${at}` })),
];

const base = { terminal: true, model: "sonnet", effort: "medium", mode: "ask", commands: COMMANDS, turn: idle, attachments: [] } as const;

const stale: Session = {
  ...base,
  id: "s4",
  title: "CI timeout hunt",
  modOutOfDate: true,
  lastTurn: "14:02",
  rows: [
    { id: "s4-1", at: "14:01:12", kind: "message", from: { kind: "you" }, text: "Why does the store test fail only in CI?" },
    { id: "s4-2", at: "14:02:03", kind: "message", from: { kind: "agent" }, text: "It sleeps 50 ms and then reads the clock. CI is slower than that." },
  ],
};

const silent: Session = {
  ...base,
  id: "s5",
  title: "Notes check",
  lastTurn: "13:48",
  rows: [{ id: "s5-1", at: "13:47:00", kind: "message", from: { kind: "you" }, text: "Draft the release notes from the merged pull requests" }],
};

export const s203ComposerFixes: Scenario = {
  ...s200Sessions,
  name: "composer-fixes",
  says: "A terminal Session's `/` list filtered and picked, a refused send as a toast, and a mod out of date",
  draft: {
    sessions: (board) => {
      const store = s200Sessions.draft!.sessions!(board, [stale, silent]);
      // The second terminal's mod is not asking: Fleet refuses the send, as it does for one that is not listening.
      return { ...store, send: (id, sent) => (id === silent.id ? askToTell(NOT_REACHABLE) : store.send(id, sent)) };
    },
  },
};
