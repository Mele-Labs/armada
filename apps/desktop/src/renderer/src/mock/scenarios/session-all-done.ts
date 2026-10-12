// A Session whose agent says it has nothing left to work on: its thread ends in a callout with End
// session and Keep going, over a list of the pull requests it landed. Over the `sessions` scenario. The
// walk `session-all-done` plays it. An open pull request is held too, and is not listed.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const finished: Session = {
  id: "s14",
  title: "Pin the store clock",
  turn: { state: "idle" },
  lastTurn: "14:40",
  lastTurnAt: minutesAgo(2),
  rows: [
    { id: "d1", at: "14:38:02", kind: "message", from: { kind: "you" }, text: "Pin the clock in the store tests." },
    { id: "d2", at: "14:40:11", kind: "message", from: { kind: "agent" }, text: "The change is merged." },
  ],
  attachments: [
    {
      kind: "pull_request",
      number: 1843,
      title: "Pin the store clock",
      branch: "fix/pin-store-clock",
      address: "https://example.com/pull/1843",
      checks: { state: "passed" },
      state: "merged",
      auto: false,
    },
    {
      kind: "pull_request",
      number: 1844,
      title: "Name the clock in the store log",
      branch: "fix/store-log-clock",
      address: "https://example.com/pull/1844",
      checks: { state: "passed" },
      state: "merged",
      auto: false,
    },
    {
      kind: "pull_request",
      number: 1845,
      title: "Drop the retry shim",
      branch: "fix/drop-retry-shim",
      address: "https://example.com/pull/1845",
      checks: { state: "passed" },
      state: "open",
      auto: false,
    },
  ],
  done: { since: minutesAgo(2) },
};

export const s207SessionAllDone: Scenario = {
  ...s200Sessions,
  name: "session-all-done",
  says: "A Session whose agent says it has nothing left to do",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [finished]) },
};
