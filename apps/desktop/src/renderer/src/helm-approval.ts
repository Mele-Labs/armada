// Helm's `ask_person_to_approve` call, drawn in the dock as the card it was before Helm was a Session.
//
// The call is a tool row of the Helm Session's thread, `ask_person_to_approve <job id>`, the tool
// name prefixed by the MCP server when Helm is a real Session's. Nothing new crosses the wire: the
// Job's facts and whether its gate is still waiting are the Board's, read when the row is drawn, and
// the press is the approval gate's own, `approve_dispatch`.

import type { HelmApprovalCard, HelmApprovalCardState, SessionThreadRow } from "@armada/components";
import type { JobSummary, WorkflowSummary } from "@armada/protocol";

/** A card's own press, held for as long as the dock is: a reload starts over. `pending` is Approve pressed and Fleet not yet answered. */
export type Pressed = "pending" | "approved" | "dismissed";

/** The Job a tool row asks the person to approve, by id; absent for any other row. The server's prefix is not part of the tool's name. */
export function approvalAskOf(text: string): string | undefined {
  return /^(?:mcp__[\w-]+__)?ask_person_to_approve (\S+)$/.exec(text)?.[1];
}

export type Approving = {
  jobs: readonly JobSummary[];
  workflows: readonly WorkflowSummary[];
  pressed: Readonly<Record<string, Pressed>>;
  /** The press on a Job's approval gate, answered. */
  onApprove: (jobId: string) => Promise<{ ok: boolean }>;
  /** Where a card's press is held, by the card's id. `undefined` drops it. */
  onPress: (cardId: string, pressed: Pressed | undefined) => void;
};

/** A thread's rows with each approval ask drawn as its card, and every other row as it was. */
export function withApprovals(rows: readonly SessionThreadRow[], approving: Approving): readonly SessionThreadRow[] {
  return rows.map((row) => {
    if (row.kind !== "tool") return row;
    const jobId = approvalAskOf(row.text);
    return jobId === undefined ? row : { id: row.id, at: row.at, kind: "approval", card: cardOf(row.id, jobId, approving) };
  });
}

function cardOf(id: string, jobId: string, { jobs, workflows, pressed, onApprove, onPress }: Approving): HelmApprovalCard {
  const job = jobs.find((one) => one.id === jobId);
  const workflow = job === undefined ? undefined : workflows.find((one) => one.id === job.workflow_id && one.manifest_id === job.owner_manifest_id);
  const state = stateOf(pressed[id], job);
  return {
    id,
    jobHandle: job?.handle ?? jobId,
    ...(workflow === undefined ? {} : { workflow: workflow.name, stepCount: workflow.steps.length }),
    state,
    ...(state === "ready" || state === "pending"
      ? {
          onApprove: () => {
            onPress(id, "pending");
            // Refused: the press is dropped, so the card reads the Board again and can be pressed again.
            void onApprove(jobId).then((answer) => onPress(id, answer.ok ? "approved" : undefined));
          },
          onDismiss: () => onPress(id, "dismissed"),
        }
      : {}),
  };
}

/**
 * **A press made here wins over the Board**, so a card this dock approved reads Approved and does not
 * snap to elsewhere the moment the Job leaves `awaiting_approval` because the press worked. With no
 * press, the Board decides: a Job still at its gate is ready, one that has moved by any other road is
 * elsewhere, and one the window has never heard of is unknown.
 */
function stateOf(pressed: Pressed | undefined, job: JobSummary | undefined): HelmApprovalCardState {
  if (pressed !== undefined) return pressed;
  if (job === undefined) return "unknown";
  return job.status === "awaiting_approval" ? "ready" : "elsewhere";
}
