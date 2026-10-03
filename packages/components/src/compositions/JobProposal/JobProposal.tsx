import { Input } from "../../primitives/Input/Input";
import { Prose } from "../../primitives/Prose/Prose";
import { Select } from "../../primitives/Select/Select";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { GuideMark } from "../GuideMark/GuideMark";
import { GUIDE_TIERS } from "../../guides";
import { AUTO, TIERS } from "../DispatchSettings/TierModels";
import type { TierChoice } from "../DispatchSettings/TierModels";
import { ProposalDoneWhen } from "./ProposalDoneWhen";
import type { ProposalCriterion } from "./ProposalDoneWhen";
import { ProposalField, ProposalFieldNote, ProposalFields } from "./ProposalFields";
import { ProposalGates } from "./ProposalGates";
import type { GateBox, ProposalGateRow, WorkflowChoice } from "./ProposalGates";
import { ProposalLanding } from "./ProposalLanding";
import type { BranchOption } from "../BranchPicker/BranchPicker";
import type { CompleteChoice, ProposalLandingValue } from "./ProposalLanding";

export type { ProposalCriterion } from "./ProposalDoneWhen";
export type { GateBox, ProposalGateRow, WorkflowChoice } from "./ProposalGates";
export type { CompleteChoice, ProposalLandingValue } from "./ProposalLanding";

/**
 * What the Job is being run against, and the words it was asked in.
 *
 * **The base branch is not here** (`3m23`, 28 Sep). It was drawn as a chip
 * beside the repository and again under How it lands, where it sits next to
 * where the work lands — which is the pair a person reads it against. One
 * fact, one place, and the place is the one that can be changed.
 */
export type ProposalRequest = {
  /** The repository, by the name a person calls it. */
  repository: string;
  /**
   * The request, in the requester's own words. Empty is a Job given none.
   * Read after approval, it is drawn through `Prose`: whoever wrote it may have
   * written markdown.
   */
  said: string;
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
  /** The request rewritten. Absent is a request nobody may change any more. */
  onRequest?: (said: string) => void;
  /** The workflow it chose, by the id its own file declares. */
  workflow: string;
  /** Every workflow the picker offers. Empty draws the name and no control. */
  workflowChoices?: readonly WorkflowChoice[];
  onWorkflow?: (workflowId: string) => void;
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
  /** The repository's branches, for How it lands to offer. Absent takes a typed name. */
  branches?: readonly BranchOption[] | null;
  criteria: readonly ProposalCriterion[];
  onCriterion?: (at: number, text: string) => void;
  /** One more line, appended empty for somebody to write. */
  onAddCriterion?: () => void;
  /** One line taken off. */
  onRemoveCriterion?: (at: number) => void;
  /** Open the issue a criterion's words came from, where an address is known. */
  onOpenIssue?: (ref: string) => void;
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
  onRequest,
  workflow,
  workflowChoices,
  onWorkflow,
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
  branches,
  criteria,
  onCriterion,
  onAddCriterion,
  onRemoveCriterion,
  onOpenIssue,
  frozenAt,
}: JobProposalProps) {
  const frozen = frozenAt !== undefined;
  return (
    <div className="armada-proposal">
      <div className="armada-proposal__request-column">
        <ProposalAsked
          request={request}
          frozen={frozen}
          {...(onRequest === undefined ? {} : { onRequest })}
        />

        <ProposalDoneWhen
          criteria={criteria}
          {...(onCriterion === undefined ? {} : { onCriterion })}
          {...(onAddCriterion === undefined ? {} : { onAdd: onAddCriterion })}
          {...(onRemoveCriterion === undefined ? {} : { onRemove: onRemoveCriterion })}
          {...(onOpenIssue === undefined ? {} : { onOpenIssue })}
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
        </ProposalFields>

        <ProposalGates
          frozen={frozen}
          workflow={workflow}
          workflowChoices={workflowChoices ?? []}
          {...(onWorkflow === undefined ? {} : { onWorkflow })}
          steps={steps}
          {...(onGate === undefined ? {} : { onGate })}
          {...(onOverride === undefined ? {} : { onOverride })}
        />

        <div className="armada-proposal__region">
          <div className="armada-proposal__heading-row">
            <h3 className="armada-proposal__heading">{frozen ? "Model per tier, frozen" : "Model per tier"}</h3>
            <GuideMark guide={GUIDE_TIERS} />
          </div>
          <ProposalFields>
            {TIERS.map(([tier, label]) => (
              <ProposalField key={tier} label={label} bare={onTiers !== undefined}>
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
              run here at all.

              **It says what the number costs you** (`pojb`, 28 Sep). It read
              `This machine runs 4 at once, across every Job` — true, and it
              never said the thing that makes it worth reading, which is that
              asking for 4 here does not get you 4 while the other Jobs are
              working. */}
          <ProposalFieldNote>
            {machineCap === null
              ? "Fleet has not said how many this machine runs at once."
              : `This machine runs ${machineCap} Drones at once across every Job, so a busy machine gives this one fewer than you ask for here.`}
          </ProposalFieldNote>
        </div>

        <ProposalLanding
          landing={landing}
          {...(onLanding === undefined ? {} : { onLanding })}
          completeChoices={completeChoices}
          {...(branches === undefined ? {} : { branches })}
        />
      </section>
    </div>
  );
}

/**
 * What was asked, and — before approval — the words themselves to change.
 *
 * **Editable, since 28 Sep 2026** (`d9b3`). These words are what every Drone
 * is handed, and the gate is the last moment anybody reads them before one
 * does.
 *
 * **No Edit press and no Done** (the owner, 28 Sep): *"Just let me edit
 * without needing to click a button."* Before approval it is a field, after
 * approval it is a reading, and approval is the only thing that moves between
 * them — a press in the middle was a third state pretending to be the second.
 */
function ProposalAsked({
  request,
  frozen,
  onRequest,
}: {
  request: ProposalRequest;
  frozen: boolean;
  onRequest?: (said: string) => void;
}) {
  return (
    <section className="armada-proposal__card" aria-label="What was asked">
      <div className="armada-proposal__against">
        <span className="armada-proposal__eyebrow">Against</span>
        <span className="armada-proposal__chip">{request.repository}</span>
      </div>
      {onRequest === undefined ? (
        <div className="armada-proposal__said" data-absent={request.said === "" || undefined}>
          {request.said === "" ? request.absent : <Prose text={request.said} />}
        </div>
      ) : (
        <div className="armada-proposal__said-field">
          <Textarea
            aria-label="What was asked"
            value={request.said}
            placeholder="What this Job is for, in your own words"
            onChange={(event) => onRequest(event.target.value)}
          />
        </div>
      )}
      <p className="armada-proposal__foot">{frozen ? REQUEST_FOOT.frozen : REQUEST_FOOT.open}</p>
    </section>
  );
}
