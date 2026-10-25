// A Job's thread where Fleet sent the agent back for an answer it never wrote. Once at a turn's end, which Fleet's notice and
// the answer that followed fold into one row; once at a question asked after a lone status line, so the answer draws before the
// question waits on the owner. The walk `unanswered-question` plays it.

import type { HelmCallInFlight, RepositorySummary, SessionRow } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const JOB = "01UNANSWEREDQUESTIONJOBAA";
const FLEET = { kind: "session", id: "fleet", title: "Fleet" } as const;

const UNANSWERED_TURN =
  "Since his last message you have written him nothing but a status line, and he reads only what you write, never your reasoning. Write your answer to his message.";

const ASKED: HelmCallInFlight = {
  call: "unanswered-q1",
  manifest_id: "armada",
  asked_at: "2026-10-24T04:02:10.000Z",
  tool: "AskUserQuestion",
  detail: "",
  truncated: false,
  rule: "AskUserQuestion",
  offers: ["allow_once", "refuse"],
  holding_for_seconds: 840,
  questions: [
    {
      question: "Now that the Logbook covers it, what should happen to the command recording itself?",
      header: "Run memory",
      multi_select: false,
      options: [
        { label: "Remove it", description: "Delete the recording and its Bridge list in a separate change." },
        { label: "Leave it as it is", description: "Land this fix and leave the recording alone." },
      ],
    },
  ],
};

const records = () => [
  hosted(JOB, {
    title: "Keep run memory out of the Logbook",
    attachments: [held("slot", "3"), held("branch", "fix/logbook-no-run-fold")],
    hosted: { turn: { state: "idle" }, mode: "auto", running: false, asked: ASKED },
  }),
];

const threads = (): Record<string, SessionRow[]> => ({
  [JOB]: [
    { kind: "message", id: "y1", at: "2026-10-24T03:45:00.000Z", from: { kind: "you" }, text: "Is the fix pushed?" },
    { kind: "tool", id: "t1", at: "2026-10-24T03:45:05.000Z", text: "Bash git status --short --branch" },
    { kind: "message", id: "n1", at: "2026-10-24T03:45:08.000Z", from: FLEET, text: UNANSWERED_TURN, notice: "reply_claim" },
    { kind: "message", id: "m1", at: "2026-10-24T03:45:12.000Z", from: { kind: "agent" }, text: "Yes. It's pushed on fix/logbook-no-run-fold, and the pull request waits on your look at the Logbook." },
    { kind: "message", id: "y2", at: "2026-10-24T04:01:00.000Z", from: { kind: "you" }, text: "What is the purpose in recording commands?" },
    { kind: "message", id: "m2", at: "2026-10-24T04:01:21.000Z", from: { kind: "agent" }, text: "Status: answering your question." },
    { kind: "tool", id: "t2", at: "2026-10-24T04:01:25.000Z", text: "Bash git log --format=%s -- crates/fleet/src/run_memory.rs" },
    { kind: "tool", id: "t3", at: "2026-10-24T04:01:41.000Z", text: "AskUserQuestion" },
    {
      kind: "message",
      id: "m3",
      at: "2026-10-24T04:02:05.000Z",
      from: { kind: "agent" },
      text: "Commands were recorded so a folder with no armada.yml could still tell the next Job how to run its tests. It kept each command exactly as typed, so it filled up with one-off runs. The Logbook now learns the same thing from Jobs' records, with evidence, so the recording has nothing left to do for agents.",
    },
    { kind: "tool", id: "t4", at: "2026-10-24T04:02:10.000Z", text: "AskUserQuestion" },
    { kind: "ask", id: "ask-unanswered-q1", at: "2026-10-24T04:02:10.000Z", ask: ASKED, state: "waiting" },
  ],
});

function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  const served = new FakeSessionsFleet(records(), threads()).scenario(board);
  return {
    ...served,
    name: "unanswered-question",
    says: "A Job's thread where Fleet sent the agent back to write the answer it skipped, at a turn's end and at a question",
    behaves: (handle) => new FakeSessionsFleet(records(), threads()).scenario(board).behaves!(handle),
  };
}

export const s324UnansweredQuestion: Scenario = build();
