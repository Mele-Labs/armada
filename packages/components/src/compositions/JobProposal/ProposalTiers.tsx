import { Input } from "../../primitives/Input/Input";
import { Select } from "../../primitives/Select/Select";
import { GuideMark } from "../GuideMark/GuideMark";
import { GUIDE_TIERS } from "../../guides";
import { AUTO, TIERS } from "../DispatchSettings/TierModels";
import type { TierChoice } from "../DispatchSettings/TierModels";
import { ProposalField, ProposalFieldNote, ProposalFields } from "./ProposalFields";

/** A Job with no cap of its own, read: the machine's cap holds. */
export const CAP_UNSET = "As many as the machine allows";

export type ProposalTiersProps = {
  tiers: TierChoice;
  /** The map rewritten. Absent draws each tier as the model it names, frozen. */
  onTiers?: (tiers: TierChoice) => void;
  /** The models a tier may name. Empty until the connection answers. */
  models: readonly string[];
  /** How many Drones this Job may run at once. Absent is the machine's cap holding. */
  droneCap?: number;
  onDroneCap?: (cap: number | undefined) => void;
  /** How many the machine runs across every Job. `null` before Fleet said. */
  machineCap: number | null;
  /** Approved: the heading says the map is frozen. */
  frozen?: boolean;
};

/**
 * Which model each tier runs on, and how many Drones the Job may run at once.
 *
 * **One region, drawn on the proposal screen and on Overview's approval panel**
 * (the owner, 2 Oct 2026: the panel edits the tiers and the cap too). It was
 * written inline in `JobProposal`, which only a mock draft ever reached; a
 * second copy in the panel is how the two would come to say different things.
 */
export function ProposalTiers({
  tiers,
  onTiers,
  models,
  droneCap,
  onDroneCap,
  machineCap,
  frozen = false,
}: ProposalTiersProps) {
  return (
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
              CAP_UNSET
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
  );
}
