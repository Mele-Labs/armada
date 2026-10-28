// Making a workflow. New workflow on the Workflows page opens the Dispatch sheet for a Job that makes one; Start opens that Job in the
// Cockpit, which works for a beat and then proposes the workflow: a card in its thread and an item in Needs you. Approve saves it under
// its scope, so the Workflows list shows it; Dismiss settles the card and keeps nothing. A mock: the Job is scripted. The walk `newWorkflow` plays it.

import type { WaitingItem } from "@armada/protocol";
import type { SessionRow } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { approveProposedWorkflow, workflowing } from "../workflows-fleet";
import { s200Sessions } from "./sessions";

const BEAT_MS = 1500;

type Scope = "kit" | "repository";

/** The name a workflow takes from the words: the first two, as an id. */
const nameOf = (words: string) =>
  words
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter((one) => one !== "")
    .slice(0, 2)
    .join("-") || "workflow";

const definitionOf = (name: string, words: string) =>
  JSON.stringify(
    {
      workflow_id: name,
      for_requests: words,
      steps: [
        { id: "draft", instructions: "Read the merged pull requests since the last tag and draft the notes." },
        { id: "deliver", instructions: "Open a pull request with the notes." },
      ],
    },
    null,
    2,
  );

const flow = workflowing();

export const newWorkflow: Scenario = {
  ...s200Sessions,
  name: "new-workflow",
  says: "The Workflows page with New workflow: the Dispatch sheet starts a Job that proposes a workflow, approved from Needs you into the list",
  state: { ...s200Sessions.state, repository: flow.state.repository, holds: flow.state.holds, health: flow.state.health },
  behaves: flow.behaves,
  draft: {
    sessions: (board) => {
      const store = s200Sessions.draft!.sessions!(board);
      const made = new Map<string, { name: string; scope: Scope; text: string }>();
      return {
        ...store,
        send: async (id, message) => {
          const sent = (await store.send(id, message)) ?? true;
          if (!message.text.startsWith("New workflow:")) return sent;
          const words = message.text.split("\n")[0]!.slice("New workflow:".length).trim();
          const scope: Scope = /Every repository/.test(message.text) ? "kit" : "repository";
          const name = nameOf(words);
          const item = `draft:${id}-proposal`;
          made.set(item, { name, scope, text: definitionOf(name, words) });
          store.edit(id, (one) => ({ ...one, title: `New workflow: ${name}`, turn: { state: "working" }, standing: "working" }));
          setTimeout(() => {
            const call: WaitingItem = {
              id: item,
              text: `${name}, ${scope === "kit" ? "every repository on this Mac" : "this repository"}`,
              since: new Date().toISOString(),
              source: "draft",
              act: { kind: "answer", target: item },
              options: [{ label: "Approve", description: definitionOf(name, words).replace(/\s+/g, " ") }, { label: "Dismiss" }],
            };
            const proposal: SessionRow = { id: `${id}-proposal`, at: new Date().toISOString(), kind: "draft", title: name, text: `\`\`\`json\n${made.get(item)!.text}\n\`\`\``, workflow: { name, scope } };
            store.edit(id, (one) => ({ ...one, turn: { state: "idle" }, standing: "replied", rows: [...one.rows, proposal], waitingFor: [...(one.waitingFor ?? []), call] }));
          }, BEAT_MS);
          return sent;
        },
        answerWaiting: (id, itemId, given) => {
          const proposed = made.get(itemId);
          if (proposed === undefined) return store.answerWaiting?.(id, itemId, given);
          const approved = given.choice !== 1;
          if (approved) approveProposedWorkflow(proposed.name, proposed.scope, proposed.text);
          store.edit(id, (one) => ({
            ...one,
            rows: one.rows.map((row) => (row.id === itemId.slice("draft:".length) && row.kind === "draft" ? { ...row, settled: { how: approved ? "approved" : "withdrawn", at: new Date().toISOString() } } : row)),
            waitingFor: (one.waitingFor ?? []).filter((it) => it.id !== itemId),
          }));
        },
      };
    },
  },
};
