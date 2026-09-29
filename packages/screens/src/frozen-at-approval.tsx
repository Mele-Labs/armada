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
  ProposalField,
  ProposalFields,
  ProposalGates,
  ProposalLanding,
  TIERS,
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

/**
 * Frozen because no handler is passed. `ProposalLanding` and `ProposalField`
 * already read that way at the gate, so the words a person approved and the
 * words they read back are one spelling.
 */
export function FrozenAtApproval({ landing, proposal, whole }: FrozenAtApprovalProps) {
  const tiers: TierModels = proposal.tiers;
  // The instant, which the proposal's own heading carried while Overview drew
  // it. Nothing else on any destination says when the Job was approved.
  const frozenAt = frozenAtOf(proposal);
  return (
    <section className="armada-settings-tab__frozen" aria-label="Frozen at approval">
      {/* The board's own sentence on the approval screen, said here in the
          past tense: the fields below read like the ones under them and only
          this says which of the two a person can move. */}
      <p className="armada-settings-tab__note">
        {frozenAt === undefined ? "Frozen when you approved." : `Frozen ${frozenAt}, when you approved.`}{" "}
        Nothing here changes while the Job runs.
      </p>
      <ProposalGates
        workflow={proposal.workflow_id}
        workflowChoices={[]}
        steps={gateRowsOf(proposal.gates, whole)}
        frozen
      />
      <ProposalLanding landing={landingValueOf(landing)} completeChoices={completeChoices()} />
      <div className="armada-proposal__region">
        <div className="armada-proposal__heading-row">
          <h3 className="armada-proposal__heading">Model per tier, frozen</h3>
        </div>
        <ProposalFields>
          {TIERS.map(([tier, label]) => (
            <ProposalField key={tier} label={label}>
              {tiers[tier] ?? AUTO}
            </ProposalField>
          ))}
        </ProposalFields>
      </div>
    </section>
  );
}
