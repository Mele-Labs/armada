// Making a workflow. New workflow on the Workflows page opens the Dispatch sheet for a Job that makes one; Start opens that Job in the
// Cockpit, which works for a beat and then proposes the workflow: a card in its thread and an item in Needs you. Approve saves it under
// its scope, so the Workflows list shows it; Dismiss settles the card and keeps nothing. A mock: the Job is scripted. The walk `newWorkflow` plays it.

import type { WaitingItem } from "@armada/protocol";
import type { SessionRow } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import {
  approveProposedWorkflow,
  proposeWorkflowChange,
  settleProposedChange,
  workflowing,
} from "../workflows-fleet";
import { s200Sessions } from "./sessions";

const BEAT_MS = 1500;

type Scope = "kit" | "repository";

/** The workflow the Job proposes, whatever was asked: the annotations workflow, step by step as its card draws it. */
const ANNOTATIONS = {
  name: "annotations",
  title: "Annotations: quiz, change, refresh preview, your sign-off, land",
  forRequests:
    "A Job dispatched from a set of annotations the owner left on the app: small changes to what he sees, made quickly, put into his running build with a preview refresh, and landed once he approves them there.",
  steps: [
    {
      id: "quiz",
      label: "Quiz",
      about:
        "Asks you about each annotation, one question at a time, until you agree on what each asks for.",
      endsWhen: "you approve the plan",
      gate: "plan_approved",
      instructions:
        "Read every open annotation. Ask about one at a time, with what you would change and why, until he agrees on what each asks for. Then record the plan, one group a part.",
    },
    {
      id: "change",
      label: "Change and walk",
      about:
        "Makes the changes quickly and walks each part you would see. Left out when nothing would be seen.",
      endsWhen: "you approve the walk",
      gate: "walk_approved",
      instructions:
        "Make each change in the mock first. Show each part he would see as a walk, one walk a part, and change it until he approves it.",
    },
    {
      id: "refresh_preview",
      label: "Refresh preview and sign off",
      about:
        "Puts the changes in your running build with a preview refresh, then asks whether you are good with them.",
      endsWhen: "you press Approve",
      gate: "you_approved",
      asksDelivery: true,
      instructions:
        "Commit the changes, refresh the preview so his running build has them, and raise one item asking whether he is good with them. Ask how he wants it delivered on the same item.",
    },
    {
      id: "land",
      label: "Refine and land",
      about:
        "Refines the approved work, writes the tests and lands it through a pull request merged on a green ci.",
      endsWhen: "the pull request merges",
      gate: "merged",
      delivers: true,
      instructions:
        "Tidy what was approved without changing what he sees. Write the tests, open the pull request and land it once ci is green.",
    },
  ],
} as const;

/** The definition the Job proposes, as Fleet would keep it. */
const definitionOf = () =>
  JSON.stringify(
    {
      version: 1,
      workflow_id: ANNOTATIONS.name,
      name: ANNOTATIONS.title,
      for_requests: ANNOTATIONS.forRequests,
      steps: ANNOTATIONS.steps.map((step) => ({
        id: step.id,
        label: step.label,
        about: `${step.about} Ends when ${step.endsWhen}.`,
        gate: step.gate,
        ...("asksDelivery" in step
          ? {
              asks_delivery: true,
            }
          : {}),
        delivers: "delivers" in step,
        instructions: step.instructions,
      })),
    },
    null,
    2,
  );

const flow = workflowing();

export const newWorkflow: Scenario = {
  ...s200Sessions,
  name: "new-workflow",
  says: "The Workflows page with New workflow: the Dispatch sheet starts a Job that proposes a workflow, approved from Needs you into the list",
  state: {
    ...s200Sessions.state,
    repository: flow.state.repository,
    holds: flow.state.holds,
    health: flow.state.health,
  },
  behaves: flow.behaves,
  draft: {
    sessions: (board) => {
      const store = s200Sessions.draft!.sessions!(board);
      const made = new Map<
        string,
        {
          name: string;
          scope: Scope;
          text: string;
          change: number;
        }
      >();
      let changes = 100;
      return {
        ...store,
        send: async (id, message) => {
          const sent = (await store.send(id, message)) ?? true;
          if (!message.text.startsWith("New workflow:")) return sent;
          const scope: Scope = /^Scope: kit/m.test(message.text) ? "kit" : "repository";
          const name = ANNOTATIONS.name;
          const item = `draft:${id}-proposal`;
          const change = changes++;
          made.set(item, {
            name,
            scope,
            text: definitionOf(),
            change,
          });
          store.edit(id, (one) => ({
            ...one,
            title: `New workflow: ${name}`,
            turn: {
              state: "working",
            },
            standing: "working",
          }));
          setTimeout(() => {
            const call: WaitingItem = {
              id: item,
              text: `Workflow ${name}`,
              since: new Date().toISOString(),
              source: "draft",
              act: {
                kind: "answer",
                target: item,
              },
              options: [
                {
                  label: "Approve",
                  description: definitionOf().replace(/\s+/g, " "),
                },
                {
                  label: "Dismiss",
                },
              ],
            };
            const proposal: SessionRow = {
              id: `${id}-proposal`,
              at: new Date().toISOString(),
              kind: "draft",
              title: name,
              text: `\`\`\`json\n${made.get(item)!.text}\n\`\`\``,
              change,
              workflow: {
                ...ANNOTATIONS,
                scope,
              },
            };
            proposeWorkflowChange(change, name, scope, made.get(item)!.text);
            store.edit(id, (one) => ({
              ...one,
              turn: {
                state: "idle",
              },
              standing: "replied",
              rows: [
                ...one.rows,
                proposal,
              ],
              waitingFor: [
                ...(one.waitingFor ?? []),
                call,
              ],
            }));
          }, BEAT_MS);
          return sent;
        },
        answerWaiting: (id, itemId, given) => {
          const proposed = made.get(itemId);
          if (proposed === undefined) return store.answerWaiting?.(id, itemId, given);
          const approved = given.choice !== 1;
          settleProposedChange(proposed.change);
          if (approved) approveProposedWorkflow(proposed.name, proposed.scope, proposed.text);
          store.edit(id, (one) => ({
            ...one,
            rows: one.rows.map((row) =>
              row.id === itemId.slice("draft:".length) && row.kind === "draft"
                ? {
                    ...row,
                    settled: {
                      how: approved ? "approved" : "withdrawn",
                      at: new Date().toISOString(),
                    },
                  }
                : row,
            ),
            waitingFor: (one.waitingFor ?? []).filter((it) => it.id !== itemId),
          }));
        },
      };
    },
  },
};
