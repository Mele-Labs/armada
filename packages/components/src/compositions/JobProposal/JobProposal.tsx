import type { ReactNode } from "react";

import { Input } from "../../primitives/Input/Input";
import { Select } from "../../primitives/Select/Select";
import { GuideMark } from "../GuideMark/GuideMark";
import { GUIDE_TIERS } from "../../guides";
import { AUTO, TIERS } from "../DispatchSettings/TierModels";
import type { TierChoice } from "../DispatchSettings/TierModels";
import { ProposalDoneWhen } from "./ProposalDoneWhen";
import type { ProposalCriterion } from "./ProposalDoneWhen";
import { ProposalField, ProposalFieldNote, ProposalFields } from "./ProposalFields";
import { ProposalGates } from "./ProposalGates";
import type { GateBox, ProposalGateRow } from "./ProposalGates";
import { ProposalLanding } from "./ProposalLanding";
import type { CompleteChoice, ProposalLandingValue } from "./ProposalLanding";

export type { ProposalCriterion } from "./ProposalDoneWhen";
export type { GateBox, ProposalGateRow } from "./ProposalGates";
export type { CompleteChoice, ProposalLandingValue } from "./ProposalLanding";

/** What the Job is being run against, and the words it was asked in. */
export type ProposalRequest = {
  /** The repository, by the name a person calls it. */
  repository: string;
  /** The ref the work is cut from. Absent is the Manifest naming no base. */
  from?: string;
  /** The request, in the requester's own words. */
  said?: ReactNode;
  /** Why there is nothing to read, where there is nothing. */
  absent: string;
};

/**
 * A Job at its approval gate, and the same Job one press later.
 *
 * **Two columns, not one.** The request and what the Job is held to read on
 * the left; every setting reads on the right. One column of regions put the
 * settings under a screen's worth of prose and made the laptop scroll to
 * reach the thing being approved.
 *
 * **The difference between the two moments is which callbacks arrive.** A
 * handler absent is what freezes its control, rather than a `readOnly` flag
 * beside each one.
 */
export type JobProposalProps = {
  /** The title the proposer answered, and yours until you approve. */
  title: string;
  onTitle?: (title: string) => void;
  /** What the Job runs against, and what was asked. */
  request: ProposalRequest;
  /** The workflow it chose, by the name its own file declares. */
  workflow: string;
  /** What the workflow's steps are gated by, in the order they run. */
  steps: readonly ProposalGateRow[];
  onGate?: (stepId: string, box: GateBox, ticked: boolean) => void;
  onOverride?: (stepId: string, overridden: boolean) => void;
  tiers: TierChoice;
  onTiers?: (tiers: TierChoice) => void;
  models: readonly string[];
  /** How many Drones this Job may run at once. Absent is the machine's cap holding. */
  droneCap?: number;
  onDroneCap?: (cap: number | undefined) => void;
  /** How many the machine runs across every Job. `null` before Fleet said. */
  machineCap: number | null;
  landing: ProposalLandingValue;
  onLanding?: (landing: ProposalLandingValue) => void;
  completeChoices: readonly CompleteChoice[];
  criteria: readonly ProposalCriterion[];
  onCriterion?: (at: number, text: string) => void;
  /**
   * When this was approved, written out. **Absent is a proposal nobody has
   * approved**, which is the whole of the difference between the two moments.
   */
  frozenAt?: string;
};

/** What the panel is called, at each of the two moments. */
const PANEL = {
  open: "Yours to change",
  frozen: "Approved, and running",
};

/** What the request's own foot says, at each of the two moments. */
const REQUEST_FOOT = {
  open: "Frozen into the job on approval.",
  frozen: "Frozen. This is what every Drone is given.",
};

export function JobProposal({
  title,
  onTitle,
  request,
  workflow,
  steps,
  onGate,
  onOverride,
  tiers,
  onTiers,
  models,
  droneCap,
  onDroneCap,
  machineCap,
  landing,
  onLanding,
  completeChoices,
  criteria,
  onCriterion,
  frozenAt,
}: JobProposalProps) {
  const frozen = frozenAt !== undefined;
  return (
    <div className="armada-proposal">
      <div className="armada-proposal__request-column">
        <section className="armada-proposal__card" aria-label="What was asked">
          <div className="armada-proposal__against">
            <span className="armada-proposal__eyebrow">Against</span>
            <span className="armada-proposal__chip">{request.repository}</span>
            {request.from === undefined ? null : (
              <>
                <span className="armada-proposal__against-word">from</span>
                <span className="armada-proposal__chip">{request.from}</span>
              </>
            )}
          </div>
          <div className="armada-proposal__said" data-absent={request.said === undefined || undefined}>
            {request.said ?? request.absent}
          </div>
          <p className="armada-proposal__foot">{frozen ? REQUEST_FOOT.frozen : REQUEST_FOOT.open}</p>
        </section>

        <ProposalDoneWhen
          criteria={criteria}
          {...(onCriterion === undefined ? {} : { onCriterion })}
        />
      </div>

      <section className="armada-proposal__settings" aria-label="How this job runs">
        <div className="armada-proposal__settings-head">
          <h3 className="armada-proposal__settings-name">{frozen ? PANEL.frozen : PANEL.open}</h3>
          <span className="armada-proposal__settings-meta">
            {frozen ? `Frozen ${frozenAt}` : "Nothing is frozen until you approve"}
          </span>
        </div>

        <ProposalFields>
          <ProposalField label="Title" bare={onTitle !== undefined}>
            {onTitle === undefined ? (
              title
            ) : (
              <Input
                aria-label="Title"
                value={title}
                onChange={(event) => onTitle(event.target.value)}
              />
            )}
          </ProposalField>
          <ProposalField label="Workflow">{`${workflow} — ${steps.length} steps`}</ProposalField>
        </ProposalFields>

        <div className="armada-proposal__region">
          <div className="armada-proposal__heading-row">
            <h3 className="armada-proposal__heading">{frozen ? "Model per tier, frozen" : "Model per tier"}</h3>
            <GuideMark guide={GUIDE_TIERS} />
          </div>
          <ProposalFields>
            {TIERS.map(([tier, label]) => (
              <ProposalField key={tier} label={label} indent bare={onTiers !== undefined}>
                {onTiers === undefined ? (
                  (tiers[tier] ?? AUTO)
                ) : (
                  <Select
                    aria-label={label}
                    value={tiers[tier] ?? ""}
                    onChange={(event) =>
                      onTiers({
                        ...tiers,
                        [tier]: event.target.value === "" ? null : event.target.value,
                      })
                    }
                  >
                    <option value="">{AUTO}</option>
                    {models.map((model) => (
                      <option key={model} value={model}>
                        {model}
                      </option>
                    ))}
                  </Select>
                )}
              </ProposalField>
            ))}
            <ProposalField label="Drones at once" bare={onDroneCap !== undefined}>
              {onDroneCap === undefined ? (
                droneCap === undefined ? (
                  "As many as the machine allows"
                ) : (
                  String(droneCap)
                )
              ) : (
                <Input
                  aria-label="Drones at once"
                  type="number"
                  min={1}
                  {...(machineCap === null ? {} : { max: machineCap })}
                  value={droneCap === undefined ? "" : String(droneCap)}
                  placeholder={AUTO}
                  onChange={(event) =>
                    onDroneCap(event.target.value === "" ? undefined : Number(event.target.value))
                  }
                />
              )}
            </ProposalField>
          </ProposalFields>
          {/* The machine's own cap, beside the Job's and never merged into it:
              one is how many Drones this Job may run, the other is how many
              run here at all. */}
          <ProposalFieldNote>
            {machineCap === null
              ? "Fleet has not said how many this machine runs at once."
              : `This machine runs ${machineCap} at once, across every Job.`}
          </ProposalFieldNote>
        </div>

        <ProposalGates
          frozen={frozen}
          steps={steps}
          {...(onGate === undefined ? {} : { onGate })}
          {...(onOverride === undefined ? {} : { onOverride })}
        />

        <ProposalLanding
          landing={landing}
          {...(onLanding === undefined ? {} : { onLanding })}
          completeChoices={completeChoices}
        />
      </section>
    </div>
  );
}
