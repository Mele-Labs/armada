// What froze when the Job was approved, read on Settings.
//
// **Only what no other destination draws.** The request is the brief's and
// the criteria are Plan's lead, so what was left homeless when Overview
// stopped holding the proposal is how it lands, the model each tier froze to,
// and the gate on each step. The owner's call of 29 Sep 2026, against drawing
// the whole proposal here: a second copy is how the board's own rule, one
// destination one noun, gets broken.
//
// **The gates are here and not on Workflow**, though Workflow draws the same
// steps: its nodes carry how many Checks and criteria each declares, and
// nothing there says who has to look or that this Job took the decision off
// the repository for itself. That sentence lives in `ProposalGates` alone.

import {
  AUTO,
  DestinationCard,
  ProposalField,
  ProposalFields,
  ProposalGates,
  ProposalLanding,
  TIERS,
  Tooltip,
} from "@armada/components";

import type { JobDetail as JobWhole } from "@armada/protocol";

import type { LandingRule } from "./draft/landing";
import type { ProposalView, TierModels } from "./draft/proposal";
import { completeChoices, frozenAtOf, gateRowsOf, landingValueOf } from "./tab-proposal-read";

export type FrozenAtApprovalProps = {
  landing: LandingRule;
  proposal: ProposalView;
  whole: JobWhole | null;
};

/** What the instant in the card's head is, for a reader who only sees a date. */
const WHEN = "When you approved this Job.";

/** The one thing the reading says about itself, in the words the gate used. */
const HOLDS = "Nothing here changes while the Job runs.";

/**
 * Frozen because no handler is passed. `ProposalLanding` and `ProposalField`
 * already read that way at the gate, so the words a person approved and the
 * words they read back are one spelling.
 *
 * **One card, three regions.** Its head says these values froze, so each
 * region is named by what it is rather than by saying frozen again.
 */
export function FrozenAtApproval({ landing, proposal, whole }: FrozenAtApprovalProps) {
  const tiers: TierModels = proposal.tiers;
  // The instant, which the proposal's own heading carried while Overview drew
  // it. Nothing else on any destination says when the Job was approved.
  const frozenAt = frozenAtOf(proposal);
  return (
    <DestinationCard
      label="Frozen at approval"
      {...(frozenAt === undefined
        ? {}
        : {
            trailing: (
              <Tooltip label={WHEN}>
                <span className="armada-settings-tab__frozen-at">{frozenAt}</span>
              </Tooltip>
            ),
          })}
    >
      <p className="armada-settings-tab__note">{HOLDS}</p>
      <div className="armada-settings-tab__frozen">
        <ProposalGates
          workflow={proposal.workflow_id}
          workflowChoices={[]}
          steps={gateRowsOf(proposal.gates, whole, undefined, proposal.repository_says)}
        />
        <ProposalLanding landing={landingValueOf(landing)} completeChoices={completeChoices()} />
        <div className="armada-proposal__region">
          <div className="armada-proposal__heading-row">
            <h3 className="armada-proposal__heading">Model per tier</h3>
          </div>
          <ProposalFields>
            {TIERS.map(([tier, label]) => (
              <ProposalField key={tier} label={label}>
                {tiers[tier] ?? AUTO}
              </ProposalField>
            ))}
          </ProposalFields>
        </div>
      </div>
    </DestinationCard>
  );
}
