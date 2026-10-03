import { Select } from "../../primitives/Select/Select";
import { BranchPicker } from "../BranchPicker/BranchPicker";
import type { BranchOption } from "../BranchPicker/BranchPicker";
import { ProposalField, ProposalFields } from "./ProposalFields";

/**
 * How the work reaches the repository.
 *
 * **Four controls, and no shape** (#1530, 22 Sep). Where it lands, what a
 * branch is cut per, what counts as finished, and whether the pull request is
 * offered or parked. Nothing here names Atomic, Complex or Convoy: a screen
 * says what a Job does, never what kind it is.
 *
 * **Base branch and Lands in are both here and they are not the same field.**
 * They differ when you start from an unmerged branch or land in a long-lived
 * one.
 *
 * **It read `From` until 28 Sep 2026** (`1hz0`). The composer's own picker
 * became `Base branch` the same day (#1627) and job detail kept the old word,
 * so the one field a person sets in two places had two names.
 */
export type ProposalLandingValue = {
  /** Where the work lands. Empty is the Manifest naming no base. */
  target: string;
  /** Where it starts. Empty reads as the same as `target`. */
  from: string;
  /** One branch per Job, or one per group. Per-task branches are dropped. */
  branching: "job" | "group";
  /** What has to happen before the Job counts as finished. */
  completeWhen: string;
  /** Whether the pull request is offered for review or parked as a draft. */
  prMode: "ready" | "draft";
};

/** One answer to "complete when", with whether Fleet can tell yet. */
export type CompleteChoice = {
  value: string;
  label: string;
  /**
   * Whether anything on the record answers it today. **Drawn, not hidden** —
   * an option a person may pick and Fleet cannot observe is a fact about this
   * milestone rather than a control to leave out.
   */
  served: boolean;
};

export type ProposalLandingProps = {
  landing: ProposalLandingValue;
  /** Absent draws every value frozen, which is what approval does to them. */
  onLanding?: (landing: ProposalLandingValue) => void;
  completeChoices: readonly CompleteChoice[];
  /**
   * The repository's branches, for the two branch fields to offer (#1605).
   * `null` or absent is nothing having listed them, and each field takes a
   * name typed by hand.
   */
  branches?: readonly BranchOption[] | null;
};

/** What each branching unit is called where it is read rather than chosen. */
const BRANCHING: Record<ProposalLandingValue["branching"], string> = {
  job: "One branch for the whole Job",
  group: "One branch per group",
};

const PR_MODE: Record<ProposalLandingValue["prMode"], string> = {
  ready: "Offered for review",
  draft: "Parked as a draft",
};

export function ProposalLanding({
  landing,
  onLanding,
  completeChoices,
  branches = null,
}: ProposalLandingProps) {
  const moved = (change: Partial<ProposalLandingValue>): void =>
    onLanding?.({ ...landing, ...change });
  const chosen = completeChoices.find((one) => one.value === landing.completeWhen);
  const open = onLanding !== undefined;
  return (
    <section className="armada-proposal__region" aria-label="How it lands">
      <h3 className="armada-proposal__heading">How it lands</h3>
      <ProposalFields>
        <ProposalField label="Base branch" bare={open}>
          {open ? (
            // The dispatch composer's own picker, so a branch is named the same
            // way wherever Armada asks for one (#1605).
            <BranchPicker
              label="Base branch"
              labelledByRow
              value={landing.from}
              onValue={(from) => moved({ from })}
              branches={branches}
            />
          ) : landing.from === "" ? (
            "The Manifest names no base"
          ) : (
            landing.from
          )}
        </ProposalField>
        <ProposalField label="Lands in" bare={open}>
          {open ? (
            <BranchPicker
              label="Lands in"
              labelledByRow
              value={landing.target}
              onValue={(target) => moved({ target })}
              branches={branches}
              offerNew
            />
          ) : landing.target === "" ? (
            "The Manifest names no base"
          ) : (
            landing.target
          )}
        </ProposalField>
        <ProposalField label="Branches" bare={open}>
          {open ? (
            <Select
              aria-label="Branches"
              value={landing.branching}
              onChange={(event) =>
                moved({ branching: event.target.value as ProposalLandingValue["branching"] })
              }
            >
              <option value="job">{BRANCHING.job}</option>
              <option value="group">{BRANCHING.group}</option>
            </Select>
          ) : (
            BRANCHING[landing.branching]
          )}
        </ProposalField>
        <ProposalField label="Complete when" bare={open}>
          {open ? (
            <Select
              aria-label="Complete when"
              value={landing.completeWhen}
              onChange={(event) => moved({ completeWhen: event.target.value })}
            >
              {completeChoices.map((choice) => (
                <option key={choice.value} value={choice.value}>
                  {choice.label}
                </option>
              ))}
            </Select>
          ) : (
            (chosen?.label ?? landing.completeWhen)
          )}
        </ProposalField>
        <ProposalField label="Pull request" bare={open}>
          {open ? (
            <Select
              aria-label="Pull request"
              value={landing.prMode}
              onChange={(event) =>
                moved({ prMode: event.target.value as ProposalLandingValue["prMode"] })
              }
            >
              <option value="ready">{PR_MODE.ready}</option>
              <option value="draft">{PR_MODE.draft}</option>
            </Select>
          ) : (
            PR_MODE[landing.prMode]
          )}
        </ProposalField>
      </ProposalFields>
    </section>
  );
}
