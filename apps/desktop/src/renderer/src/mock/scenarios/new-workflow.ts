// The Workflows page with a New workflow form, and a Job that proposed a workflow, put to him as a card in its thread. A mock: nothing
// is written. The card settles on Approve or Dismiss. The walk `newWorkflow` plays it.

import type { WaitingItem } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { workflowing } from "../workflows-fleet";
import { s200Sessions } from "./sessions";

const JOB = "wf-proposal";
const ROW = "w-2";
const ITEM = `draft:${ROW}`;

const DEFINITION = [
  "```json",
  "{",
  '  "workflow_id": "release-notes",',
  '  "for_requests": "Write the release notes for a tag",',
  '  "steps": [',
  '    { "id": "draft", "instructions": "Read the merged pull requests since the last tag and draft the notes." },',
  '    { "id": "deliver", "instructions": "Open a pull request with the notes." }',
  "  ]",
  "}",
  "```",
].join("\n");

const COMPACT = DEFINITION.replace(/```(json)?\n?/g, "").replace(/\s+/g, " ").trim();

const call: WaitingItem = { id: ITEM, text: "release-notes, every repository on this Mac", since: new Date().toISOString(), source: "draft", act: { kind: "answer", target: ITEM }, options: [{ label: "Approve", description: COMPACT }, { label: "Dismiss" }] };

const job: Session = {
  id: JOB,
  title: "Write release notes",
  turn: { state: "idle" },
  standing: "replied",
  lastTurnAt: new Date().toISOString(),
  rows: [
    { id: "w-1", at: "10:02:00", kind: "message", from: { kind: "you" }, text: "Write the release notes for 2.4." },
    { id: ROW, at: "10:05:00", kind: "draft", title: "release-notes", text: DEFINITION, workflow: { name: "release-notes", scope: "kit" } },
  ],
  attachments: [],
  waitingFor: [call],
};

const flow = workflowing();

export const newWorkflow: Scenario = {
  ...s200Sessions,
  name: "new-workflow",
  says: "The Workflows page with a New workflow form, and a Job that proposed a workflow, waiting on Approve or Dismiss",
  state: { ...s200Sessions.state, repository: flow.state.repository, holds: flow.state.holds, health: flow.state.health },
  behaves: flow.behaves,
  draft: {
    sessions: (board) => {
      const store = s200Sessions.draft!.sessions!(board, [job]);
      return {
        ...store,
        answerWaiting: (id, itemId, given) => {
          if (itemId !== ITEM) return store.answerWaiting?.(id, itemId, given);
          const how = given.choice === 1 ? "withdrawn" : "approved";
          store.edit(id, (one) => ({
            ...one,
            rows: one.rows.map((row) => (row.id === ROW && row.kind === "draft" ? { ...row, settled: { how, at: new Date().toISOString() } } : row)),
            waitingFor: (one.waitingFor ?? []).filter((it) => it.id !== ITEM),
          }));
        },
      };
    },
  },
};
