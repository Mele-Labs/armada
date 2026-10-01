// What a Job at its dispatch gate is being approved for, read under Overview's
// lead: what counts as done, what its workflow promises, and how each step
// gates.
//
// **Read-only, and off what Fleet already sends** — the owner, 1 Oct 2026. He
// approved Job 1 from the lead on his own Fleet with the request, the
// workflow's name and the title in front of him, and nothing else: the
// proposer had picked `refactor`, which promises nothing visible changes, for
// a visible change, and the Judge refused the plan. The editable proposal
// (`ProposalTab`) draws only off a draft, and no real Fleet serves one until
// #1545.
//
// **The proposal's own pieces, with no handler passed**, which is what freezes
// each of them — so the words at this gate and the words the editable proposal
// draws are one spelling, as `frozen-at-approval.tsx` keeps them for Settings.

import { ProposalDoneWhen, ProposalGates } from "@armada/components";
import type { JobDetail as JobWhole, ManifestSummary, WorkflowSummary } from "@armada/protocol";

import { criterionViewsOf } from "./draft/criterion";
import { gateViewOf } from "./draft/proposal";
import {
  criteriaRowsOf,
  gateRowsOf,
  repositorySaysOf,
  workflowChoicesOf,
} from "./tab-proposal-read";

export type ApprovingProps = {
  whole: JobWhole;
  /** Every workflow Fleet holds, for the name this Job's id is declared under. */
  workflows: readonly WorkflowSummary[];
  /** This Job's repository, for the policy a gate defers to. */
  manifest?: ManifestSummary | undefined;
};

/**
 * **Each step's gate is `gateViewOf` on the Job's own frozen step**, the same
 * read `proposalViewOf` makes — which also wants the machine's limits, and
 * nothing here draws them.
 *
 * **No criteria draws no region**, rather than a sentence saying so.
 */
export function Approving({ whole, workflows, manifest }: ApprovingProps) {
  const criteria = criteriaRowsOf(criterionViewsOf(whole));
  // **The catalogue as it is now, not as it froze at dispatch** — `JobDetail`
  // carries no promise of its own. A Job waiting to be approved has run
  // nothing the two could disagree about. Rows are per Manifest, so the id
  // alone could name another repository's workflow of the same id.
  const forRequests = workflows.find(
    (one) => one.id === whole.job.workflow_id && one.manifest_id === whole.job.owner_manifest_id,
  )?.for_requests;
  return (
    <section
      className="armada-overview-board__approving armada-glass"
      aria-label="What you are approving"
    >
      {criteria.length === 0 ? null : <ProposalDoneWhen criteria={criteria} />}
      <ProposalGates
        workflow={whole.job.workflow_id}
        workflowChoices={workflowChoicesOf(workflows, whole.job.owner_manifest_id)}
        {...(forRequests === undefined ? {} : { forRequests })}
        steps={gateRowsOf(whole.steps.map(gateViewOf), whole, undefined, repositorySaysOf(manifest))}
      />
    </section>
  );
}
