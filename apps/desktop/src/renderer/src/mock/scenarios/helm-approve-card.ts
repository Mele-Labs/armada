// Helm asks the person to approve a Job, and the dock draws the ask as a card in the thread, as it did
// before Helm was a Session. Helm's thread holds two `ask_person_to_approve` calls: one for a Job at its
// approval gate, whose Approve is the gate's own press, and one for a Job that has since moved on, which
// reads that it is no longer waiting. The card is the Board's: pressing Approve here, or on the Job,
// settles it. Over `helm-is-a-session`. The walk `helm-approve-card` plays it.

import { featureAtApproval } from "@armada/jobs/fake";

import { asRow } from "../holding";
import type { Scenario } from "../moment";
import { s237HelmIsASession } from "./helm-is-a-session";

const AT_GATE = asRow(featureAtApproval(), 81, "retire-the-legacy-poke-path", "Retire the legacy poke path");

function build(): Scenario {
  const board = s237HelmIsASession.state;
  const gate = { ...AT_GATE.job, owner_manifest_id: "armada" };
  const moved = board.jobs[0]!;
  const workflows = AT_GATE.workflows.map((one) => ({ ...one, manifest_id: "armada" }));
  const ask = (id: string, jobId: string) => ({ id, at: "13:58:10", kind: "tool" as const, text: `mcp__armada-fleet__ask_person_to_approve ${jobId}` });
  return {
    ...s237HelmIsASession,
    name: "helm-approve-card",
    says: "Helm asks for an approval: the dock draws a card for the Job at its gate, and one for a Job that has moved on",
    state: { ...board, jobs: [...board.jobs, gate], holds: { ...board.holds, workflows: [...board.holds.workflows, ...workflows] } },
    reads: { ...s237HelmIsASession.reads, [gate.id]: { ...AT_GATE, job: gate } },
    draft: {
      sessions: (control) => {
        const store = s237HelmIsASession.draft!.sessions!(control);
        store.edit("helm-armada", (one) => ({
          ...one,
          rows: [
            ...one.rows,
            { id: "a1", at: "13:59:02", kind: "message", from: { kind: "you" }, text: "what is waiting on me?" },
            { id: "a2", at: "13:59:10", kind: "tool", text: "mcp__armada-fleet__list_jobs" },
            ask("a3", moved.id),
            ask("a4", gate.id),
            { id: "a5", at: "13:59:18", kind: "message", from: { kind: "agent" }, text: `${gate.handle} is at its approval gate.` },
          ],
        }));
        return store;
      },
    },
  };
}

export const s238HelmApproveCard: Scenario = build();
