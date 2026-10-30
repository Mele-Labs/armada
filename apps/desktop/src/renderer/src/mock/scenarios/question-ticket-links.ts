// A Job's Needs-you question names a Jira key, STORE-1234, in its text and in an option's description. The Jira key is taught, and the
// repository's Jira site is the one its issues are listed from, so the key draws as a link and a press on it opens the ticket in the
// side panel without picking the option. The walk `question-ticket-links` plays it; `mount.tsx` seeds the teaching.

import type { RepositorySummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sharedPanelReads } from "../shared-panel-reads";
import { sessionsStore } from "../sessions/script";
import { bindTeachFleet, teachedMods } from "../teach";

// The `ops` Manifest lists Jira-keyed issues at acme.atlassian.net, which is how Bridge learns the tracker's host.
const OPS: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "ops" } };

const job: Session = {
  id: "ticket-1",
  title: "Fix the store totals",
  manifestId: "ops",
  cwd: OPS.root,
  model: "opus",
  effort: "high",
  mode: "auto",
  turn: { state: "idle" },
  lastTurnAt: new Date(Date.now() - 2 * 60_000).toISOString(),
  standing: "idle",
  attachments: [],
  rows: [
    { id: "t-1", at: "13:20:00", kind: "message", from: { kind: "you" }, text: "Fix the store totals." },
    { id: "t-2", at: "13:21:02", kind: "message", from: { kind: "agent" }, text: "The totals round twice. STORE-1234 covers one fix, and there is a second way." },
  ],
  asked: {
    command: "Which fix should I take?",
    call: "ticket-q1",
    offers: ["allow_once", "refuse"],
    questions: [
      {
        question: "STORE-1234 rounds each line, but the receipt rounds the total. Which should I fix?",
        header: "Rounding",
        multi_select: false,
        options: [
          { label: "Round each line", description: "Follow STORE-1234: round every line once and sum the lines, so the receipt matches the basket." },
          { label: "Round the total", description: "Keep the lines exact and round the total once, which leaves the receipt a cent off the lines." },
        ],
      },
    ],
  },
};

const base = holding("question-ticket-links", "A Needs-you question names a Jira key, which draws as a link and opens the ticket in the side panel", [OPS]);

export const s300QuestionTicketLinks: Scenario = {
  ...base,
  state: { ...base.state, repository: null },
  draft: { sessions: () => sessionsStore([job], [], new Map(), []), sharedPanel: () => sharedPanelReads({ lines: [] }) },
  behaves: (handle) => {
    bindTeachFleet({ ask: () => undefined, kept: () => undefined }, handle);
    return { ...teachedMods };
  },
};
