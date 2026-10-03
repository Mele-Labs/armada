// What a Job at its dispatch gate is being approved for, read under Overview's
// lead: its title, what counts as done, how each step gates, and where it lands.
//
// **Under the lead, the owner's 1 Oct 2026 arrangement.** He approved Job 1
// with the title and the workflow's name in front of him and nothing else, and
// the Judge refused the plan the proposer had picked a wrong workflow for.
//
// **Yours to change until the press, since 23.8.** `approve_dispatch` takes
// the proposal as the person left it (#1641), so each region takes its handler
// and the press sends what moved — `approvalOf`. The proposal's own pieces, so
// the words here and on `ProposalTab` are one spelling.

import {
  Input,
  ProposalDoneWhen,
  ProposalField,
  ProposalFields,
  ProposalGates,
  ProposalLanding,
} from "@armada/components";
import type { GateBox, ProposalLandingValue } from "@armada/components";
import type { JobDetail as JobWhole, ManifestSummary, WorkflowSummary } from "@armada/protocol";

import { baseBranch } from "./draft/branches";
import type { BranchView } from "./draft/branches";
import {
  completeChoices,
  criteriaAdded,
  criteriaRowsOf,
  criteriaWith,
  criteriaWithout,
  gateRowsOf,
  gatesWith,
  landingValueOf,
  landingWith,
  proposalOnWorkflow,
  repositorySaysOf,
  stepsDeclaredOf,
  workflowChoicesOf,
} from "./tab-proposal-read";
import type { ProposalEdits } from "./tab-proposal-read";

export type ApprovingProps = {
  whole: JobWhole;
  /** What this Job is at, and what a person has moved since it arrived. */
  edits: ProposalEdits;
  /** One change, held by the screen until the press. Absent draws every region read. */
  onEdits?: (edits: ProposalEdits) => void;
  /** Every workflow Fleet holds, for the name this Job's id is declared under. */
  workflows: readonly WorkflowSummary[];
  /** This Job's repository, for the policy a gate defers to. */
  manifest?: ManifestSummary | undefined;
  /** The repository's branches (#1605). `null` is nothing having listed them. */
  branches: readonly BranchView[] | null;
};

/**
 * **No criteria draws no region where nothing may be added**, rather than a
 * sentence saying so; while the proposal is open, the region is where a line
 * is added.
 */
export function Approving({ whole, edits, onEdits, workflows, manifest, branches }: ApprovingProps) {
  const { proposal, landing, criteria } = edits;
  const moved =
    onEdits === undefined ? undefined : (change: Partial<ProposalEdits>) => onEdits({ ...edits, ...change });
  const base = baseBranch(branches);
  // **The catalogue as it is now** — `JobDetail` carries no promise of its
  // own. Rows are per Manifest, so the id alone could name another
  // repository's workflow of the same id.
  const forRequests = workflows.find(
    (one) => one.id === proposal.workflow_id && one.manifest_id === whole.job.owner_manifest_id,
  )?.for_requests;
  return (
    <section
      className="armada-overview-board__approving armada-glass"
      aria-label="What you are approving"
    >
      <ProposalFields>
        <ProposalField label="Title" bare={moved !== undefined}>
          {moved === undefined ? (
            proposal.title
          ) : (
            <Input
              aria-label="Title"
              value={proposal.title}
              onChange={(event) => moved({ proposal: { ...proposal, title: event.target.value } })}
            />
          )}
        </ProposalField>
      </ProposalFields>
      {criteria.length === 0 && moved === undefined ? null : (
        <ProposalDoneWhen
          criteria={criteriaRowsOf(criteria)}
          {...(moved === undefined
            ? {}
            : {
                onCriterion: (at: number, text: string) => moved({ criteria: criteriaWith(criteria, at, text) }),
                onAdd: () => moved({ criteria: criteriaAdded(criteria) }),
                onRemove: (at: number) => moved({ criteria: criteriaWithout(criteria, at) }),
              })}
        />
      )}
      <ProposalGates
        workflow={proposal.workflow_id}
        workflowChoices={workflowChoicesOf(workflows, whole.job.owner_manifest_id)}
        {...(forRequests === undefined ? {} : { forRequests })}
        steps={gateRowsOf(
          proposal.gates,
          whole,
          stepsDeclaredOf(workflows, proposal.workflow_id),
          repositorySaysOf(manifest),
        )}
        {...(moved === undefined
          ? {}
          : {
              // Another workflow rebuilds every gate, because a gate belongs to
              // a step — `proposalOnWorkflow` says why.
              onWorkflow: (workflowId: string) =>
                moved({ proposal: proposalOnWorkflow(proposal, workflows, workflowId) }),
              onGate: (stepId: string, box: GateBox, ticked: boolean) =>
                moved({ proposal: { ...proposal, gates: gatesWith(proposal.gates, stepId, { [box]: ticked }) } }),
              onOverride: (stepId: string, overridden: boolean) =>
                moved({ proposal: { ...proposal, gates: gatesWith(proposal.gates, stepId, { overridden }) } }),
            })}
      />
      <ProposalLanding
        landing={landingValueOf(landing, base)}
        {...(moved === undefined
          ? {}
          : {
              onLanding: (value: ProposalLandingValue) => moved({ landing: landingWith(landing, value, base) }),
            })}
        completeChoices={completeChoices()}
        branches={branches}
      />
    </section>
  );
}
