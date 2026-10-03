// What a Job at its dispatch gate is being approved for, read under Overview's
// lead: its title and request, what counts as done, how each step gates, which
// model each tier runs on and how many Drones at once, and where it lands.
//
// **Under the lead, the owner's 1 Oct 2026 arrangement.** He approved Job 1
// with the title and the workflow's name in front of him and nothing else, and
// the Judge refused the plan the proposer had picked a wrong workflow for.
//
// **Yours to change until the press, since 23.8.** `approve_dispatch` takes
// the proposal as the person left it (#1641), so each region takes its handler
// and the press sends what moved — `approvalOf`. The proposal's own pieces, so
// the words here and on `ProposalTab` are one spelling. **The request, tiers
// and cap too** (the owner, 2 Oct 2026): only `ProposalTab`, which draws only
// off a mock draft, offered them.

import {
  Input,
  Prose,
  ProposalDoneWhen,
  ProposalField,
  ProposalFields,
  ProposalGates,
  ProposalLanding,
  ProposalTiers,
  Textarea,
} from "@armada/components";
import type { GateBox, ProposalLandingValue } from "@armada/components";
import type {
  JobDetail as JobWhole,
  ManifestSummary,
  WorkflowSummary,
} from "@armada/protocol";

import { baseBranch } from "./draft/branches";
import type { BranchView } from "./draft/branches";
import type { ProposalView } from "./draft/proposal";
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
  withoutCap,
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
  /** The models a tier may name. Empty until the connection answers. */
  models: readonly string[];
  /** How many Drones the machine runs across every Job. `null` before Fleet said. */
  machineCap: number | null;
};

/**
 * **No criteria draws no region where nothing may be added**, rather than a
 * sentence saying so; while the proposal is open, the region is where a line
 * is added.
 */
export function Approving({
  whole,
  edits,
  onEdits,
  workflows,
  manifest,
  branches,
  models,
  machineCap,
}: ApprovingProps) {
  const { proposal, landing, criteria } = edits;
  const moved =
    onEdits === undefined
      ? undefined
      : (change: Partial<ProposalEdits>) => onEdits({ ...edits, ...change });
  const base = baseBranch(branches);
  // **The catalogue as it is now** — `JobDetail` carries no promise of its
  // own. Rows are per Manifest, so the id alone could name another
  // repository's workflow of the same id.
  const forRequests = workflows.find(
    (one) =>
      one.id === proposal.workflow_id &&
      one.manifest_id === whole.job.owner_manifest_id,
  )?.for_requests;
  return (
    <section
      className="armada-overview-board__approving armada-glass"
      aria-label="What you are approving"
    >
      <div className="armada-overview-board__approving-column">
        <ProposalFields>
          <ProposalField label="Title" bare={moved !== undefined}>
            {moved === undefined ? (
              proposal.title
            ) : (
              <Input
                aria-label="Title"
                value={proposal.title}
                onChange={(event) =>
                  moved({
                    proposal: { ...proposal, title: event.target.value },
                  })
                }
              />
            )}
          </ProposalField>
          {/* The words every Drone is handed, and the gate is the last moment
              anybody reads them before one does. `JobDetail.facts`, sent as
              `facts` where it moved. Nothing drawn where read and empty. */}
          {moved === undefined && (proposal.asked ?? "") === "" ? null : (
            <ProposalField label="Request" bare={moved !== undefined}>
              {moved === undefined ? (
                <Prose text={proposal.asked ?? ""} />
              ) : (
                <Textarea
                  aria-label="What was asked"
                  value={proposal.asked ?? ""}
                  onChange={(event) =>
                    moved({ proposal: { ...proposal, asked: event.target.value } })
                  }
                />
              )}
            </ProposalField>
          )}
        </ProposalFields>
        {criteria.length === 0 && moved === undefined ? null : (
          <ProposalDoneWhen
            criteria={criteriaRowsOf(criteria)}
            {...(moved === undefined
              ? {}
              : {
                  onCriterion: (at: number, text: string) =>
                    moved({ criteria: criteriaWith(criteria, at, text) }),
                  onAdd: () => moved({ criteria: criteriaAdded(criteria) }),
                  onRemove: (at: number) =>
                    moved({ criteria: criteriaWithout(criteria, at) }),
                })}
          />
        )}
      </div>
      <div className="armada-overview-board__approving-column">
        <ProposalGates
          workflow={proposal.workflow_id}
          workflowChoices={workflowChoicesOf(
            workflows,
            whole.job.owner_manifest_id,
          )}
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
                  moved({
                    proposal: proposalOnWorkflow(
                      proposal,
                      workflows,
                      workflowId,
                    ),
                  }),
                onGate: (stepId: string, box: GateBox, ticked: boolean) =>
                  moved({
                    proposal: {
                      ...proposal,
                      gates: gatesWith(proposal.gates, stepId, {
                        [box]: ticked,
                      }),
                    },
                  }),
                onOverride: (stepId: string, overridden: boolean) =>
                  moved({
                    proposal: {
                      ...proposal,
                      gates: gatesWith(proposal.gates, stepId, { overridden }),
                    },
                  }),
              })}
        />
        <ProposalTiers
          tiers={proposal.tiers}
          models={models}
          {...(proposal.drone_cap === undefined ? {} : { droneCap: proposal.drone_cap })}
          machineCap={machineCap}
          {...(moved === undefined
            ? {}
            : {
                onTiers: (tiers: ProposalView["tiers"]) => moved({ proposal: { ...proposal, tiers } }),
                onDroneCap: (cap: number | undefined) =>
                  moved({ proposal: cap === undefined ? withoutCap(proposal) : { ...proposal, drone_cap: cap } }),
              })}
        />
        <ProposalLanding
          landing={landingValueOf(landing, base)}
          {...(moved === undefined
            ? {}
            : {
                onLanding: (value: ProposalLandingValue) =>
                  moved({ landing: landingWith(landing, value, base) }),
              })}
          completeChoices={completeChoices()}
          branches={branches}
        />
      </div>
    </section>
  );
}
