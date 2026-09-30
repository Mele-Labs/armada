import { useEffect, useId, useState, type ReactNode } from "react";
import type { Reach, WhenBlocked, WhenRefused } from "@armada/protocol";
import { Button, STILL_WAITING, useStillWaiting } from "../../primitives/Button/Button";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";
import { Select } from "../../primitives/Select/Select";
import { DestinationCard } from "../DestinationCard/DestinationCard";
import { ProposalField, ProposalFieldNote, ProposalFields } from "../JobProposal/ProposalFields";

/**
 * Every setting a person can change on a running Job, as a board of cards.
 *
 * **A destination since 28 September 2026, and a sheet before that** — the
 * owner took it into the strip, and the sheet frame went with the cards, since
 * a `Sheet` nothing opens keeps every card on it off the canvas.
 *
 * **`DestinationCard` is Overview's and Pulse's own card**, so the two cannot
 * drift apart again; `ProposalField` is the proposal's own row, so a value
 * reads the same before and after it freezes.
 *
 * **What the panel says is fixed; what it holds is the caller's** — the
 * figures, the model names and the three choices' words each have an owner
 * elsewhere, and the raise dialogs are the caller's too.
 */
export type JobSettingsProps = {
  /** The cost ceiling. Absent where Fleet sent no spend, and the card is not drawn. */
  costCap?: JobSettingsCeiling;
  /** The turn ceiling, on `costCap`'s terms. */
  turnCap?: JobSettingsCeiling;
  /** What `list_models` offers, in its order. */
  models: readonly string[];
  /** The model chosen for later steps, or `null` for the workflow's own. */
  model: string | null;
  /** The line under the model once a change took. */
  modelSaid?: ReactNode;
  /** The label of the step that writes Armada's review. Absent where the workflow has none, and the row is not drawn. #903. */
  reviewStep?: string;
  /** The model chosen for the review step, or `null` for the model the other steps run on. */
  reviewModel?: string | null;
  /** The line under the review model once a change took. */
  reviewModelSaid?: ReactNode;
  onReviewModel?: (model: string | null) => void;
  /** The three answers to a command the drone was not given, in the order to offer them. */
  choices: readonly JobSettingsChoice[];
  whenBlocked: WhenBlocked;
  /** The line under the choice once a change took. */
  whenBlockedSaid?: ReactNode;
  /**
   * The three answers to a judge criterion that refuses, in the order to
   * offer them. Absent where Fleet sent no `when_refused` — a Fleet older
   * than 11.3 — and then the card is not drawn at all.
   */
  judgeChoices?: readonly JobSettingsJudgeChoice[];
  whenRefused?: WhenRefused;
  /** The line under the choice once a change took. */
  whenRefusedSaid?: ReactNode;
  onWhenRefused?: (whenRefused: WhenRefused) => void;
  /** What a person allowed for this Job, oldest first. */
  allowed: readonly JobSettingsAllowed[];
  /** The line under the list once an allow was taken back. */
  allowedSaid?: ReactNode;
  /**
   * Every rule a person always-allowed for this repository, oldest first —
   * covering every job against it, not this one alone. **Read-only here.**
   * Removing one is the Manifest screen's, where it reaches every job at once.
   */
  repositoryAllowed?: readonly string[];
  /**
   * Every control is off — the reading is not live, or a change is already
   * out — as every control that sends is. The raise buttons go with them.
   */
  disabled?: boolean;
  /** Why they are off, said once under the lead rather than left to be guessed. */
  disabledNote?: ReactNode;
  /**
   * A removal on this Job is out. **Not necessarily this panel's own row** —
   * nothing here says which command, so a row is marked busy only where its
   * own press was the one made; every other row and field just disables. #1117.
   */
  pending?: boolean;
  onModel: (model: string | null) => void;
  onWhenBlocked: (whenBlocked: WhenBlocked) => void;
  onRemove: (run: string) => void;
};

/** One ceiling: the cap in force, what has gone against it, and the press that raises it. */
export type JobSettingsCeiling = {
  /** The cap, exactly — `$60.00`, or `1000`. Mono. */
  cap: string;
  /** What has gone against it — `~$29.63`, hedged because it is estimated, or `580`. Mono. */
  used: string;
  /** Opens the raise. The dialog that collects the figure is the caller's. */
  onRaise: () => void;
  /** A raise of this ceiling was sent and Fleet has not answered. Its Raise waits. #1117. */
  pending?: boolean;
  /** The line under the row once a raise took. */
  said?: ReactNode;
};

/** One answer to a command the drone was not given, with what it commits to. */
export type JobSettingsChoice = { value: WhenBlocked; label: string; means: string };

/** One answer to a judge criterion that refuses, with what it commits to. */
export type JobSettingsJudgeChoice = { value: WhenRefused; label: string; means: string };

/** One command allowed for this Job, and how far the allow reaches. */
export type JobSettingsAllowed = { run: string; reach: Reach };

/** The first option: no model chosen, so each step runs on the one its workflow gives it. */
const WORKFLOWS_CHOICE = "";

export function JobSettings({
  costCap,
  turnCap,
  models,
  model,
  modelSaid,
  reviewStep,
  reviewModel = null,
  reviewModelSaid,
  onReviewModel,
  choices,
  whenBlocked,
  whenBlockedSaid,
  judgeChoices,
  whenRefused,
  whenRefusedSaid,
  allowed,
  allowedSaid,
  repositoryAllowed = [],
  disabled = false,
  disabledNote,
  pending = false,
  onModel,
  onWhenBlocked,
  onWhenRefused,
  onRemove,
}: JobSettingsProps) {
  const group = useId();
  // Which row's own Remove `pending` is out for — remembered locally, since
  // nothing here says which command a removal was for. Cleared once Fleet
  // has answered.
  const [pressedRun, setPressedRun] = useState<string | null>(null);
  useEffect(() => {
    if (!pending) setPressedRun(null);
  }, [pending]);
  const mineAny = pending && pressedRun !== null;
  const stillWaiting = useStillWaiting(mineAny);
  // A chosen model Fleet holds and `list_models` no longer offers is still the
  // one in force, so it stays an option rather than the select falling back to
  // a first entry that says something untrue.
  const offered = model === null || models.includes(model) ? models : [model, ...models];
  const reviewOffered =
    reviewModel === null || models.includes(reviewModel) ? models : [reviewModel, ...models];
  const runsAll = whenBlocked === "allow_all";
  // The line a removal leaves behind keeps the last allow's answer on screen
  // after the row it was about has gone.
  const showsAllowed = allowed.length > 0 || allowedSaid !== undefined;

  return (
    <div className="armada-job-settings">
      <div className="armada-job-settings__opening">
        <p className="armada-job-settings__lead">
          For this job only. Each change applies from the next thing the drone does. Nothing
          restarts, and nothing already done is undone.
        </p>
        {stillWaiting ? (
          <p className="armada-job-settings__means" role="status">
            {STILL_WAITING}
          </p>
        ) : disabled && !mineAny && disabledNote !== undefined ? (
          <p className="armada-job-settings__means">{disabledNote}</p>
        ) : null}
      </div>

      <div className="armada-job-settings__cards">
        {costCap === undefined && turnCap === undefined ? null : (
          <DestinationCard label="Limits">
            <ProposalFields>
              {costCap === undefined ? null : (
                <Ceiling
                  name="cost"
                  label="Cost cap"
                  spent="Spent"
                  raiseLabel="Raise the cost cap"
                  ceiling={costCap}
                  disabled={disabled}
                />
              )}
              {turnCap === undefined ? null : (
                <Ceiling
                  name="turns"
                  label="Turn cap"
                  spent="Turns used"
                  raiseLabel="Raise the turn cap"
                  ceiling={turnCap}
                  disabled={disabled}
                />
              )}
            </ProposalFields>
            {/* Once for the card and not once per ceiling: it is the same fact
                about both, and twice it read as two rules. */}
            <ProposalFieldNote>A cap can only go up while the job runs.</ProposalFieldNote>
          </DestinationCard>
        )}

        <DestinationCard label="Model">
          <ProposalFields>
            <ProposalField
              label="Model for the next step"
              bare
              beneath={
                <>
                  <p className="armada-job-settings__means">
                    The step running now keeps its model. Later steps start on this one.
                  </p>
                  <Said>{modelSaid}</Said>
                </>
              }
            >
              <Select
                aria-label="Model for the next step"
                value={model ?? WORKFLOWS_CHOICE}
                disabled={disabled}
                onChange={(event) =>
                  onModel(event.target.value === WORKFLOWS_CHOICE ? null : event.target.value)
                }
              >
                <option value={WORKFLOWS_CHOICE}>The workflow's choice</option>
                {offered.map((name) => (
                  <option key={name} value={name}>
                    {name}
                  </option>
                ))}
              </Select>
            </ProposalField>
            {reviewStep === undefined || onReviewModel === undefined ? null : (
              <ProposalField
                label="Model for the review"
                bare
                beneath={
                  <>
                    <p className="armada-job-settings__means">
                      Only {reviewStep}, the step that writes Armada's review, starts on this one.
                    </p>
                    <Said>{reviewModelSaid}</Said>
                  </>
                }
              >
                <Select
                  aria-label="Model for the review"
                  value={reviewModel ?? WORKFLOWS_CHOICE}
                  disabled={disabled}
                  onChange={(event) =>
                    onReviewModel(
                      event.target.value === WORKFLOWS_CHOICE ? null : event.target.value,
                    )
                  }
                >
                  <option value={WORKFLOWS_CHOICE}>The same as the other steps</option>
                  {reviewOffered.map((name) => (
                    <option key={name} value={name}>
                      {name}
                    </option>
                  ))}
                </Select>
              </ProposalField>
            )}
          </ProposalFields>
        </DestinationCard>

        <DestinationCard label="Commands">
          <RadioGroup label="When the drone needs a command it wasn't given">
            {choices.map((choice) => (
              <div className="armada-job-settings__option" key={choice.value}>
                <Radio
                  name={`${group}-when-blocked`}
                  value={choice.value}
                  checked={whenBlocked === choice.value}
                  disabled={disabled}
                  aria-describedby={`${group}-${choice.value}`}
                  onChange={() => onWhenBlocked(choice.value)}
                >
                  {choice.label}
                </Radio>
                {/* Under the label, as a drone's question draws what each
                    answer commits to: read after the name, before the press. */}
                <p className="armada-job-settings__option-means" id={`${group}-${choice.value}`}>
                  {choice.means}
                </p>
              </div>
            ))}
          </RadioGroup>
          <Said>{whenBlockedSaid}</Said>

          {/* **No list, no heading.** An empty slot stays empty — the owner,
              29 Sep 2026 — and a heading kept so the list has a place a
              person recognises is the placeholder that rule refuses.

              **Dimmed and kept under Run it**, never emptied: a list that
              vanished would read as allows that were thrown away. Dimming is
              the token step, not an alpha. */}
          {!showsAllowed ? null : (
            <div className="armada-job-settings__allowed" data-dimmed={runsAll || undefined}>
              <span className="armada-job-settings__label" id={`${group}-allowed`}>
                Allowed for this job
              </span>
              {runsAll && allowed.length > 0 ? (
                <p className="armada-job-settings__means">
                  Not needed while every command runs. They're kept in case you switch back.
                </p>
              ) : null}
              {allowed.length === 0 ? null : (
                <ul className="armada-job-settings__commands" aria-labelledby={`${group}-allowed`}>
                  {allowed.map((row) => (
                    <li className="armada-job-settings__command" key={row.run}>
                      <div className="armada-job-settings__command-text">
                        <span className="armada-job-settings__mono">{row.run}</span>
                        {/* Said on the row, because that is where somebody
                            reads what Remove will do. */}
                        {row.reach === "repository" ? (
                          <span className="armada-job-settings__means">
                            Also always allowed in armada.yml. Removing it here leaves that in
                            place.
                          </span>
                        ) : null}
                      </div>
                      {/* The name carries the command, so a list of Removes is a
                          list of different acts to anything reading by name. */}
                      <Button
                        variant="secondary"
                        size="sm"
                        ground="card"
                        pending={pressedRun === row.run && pending}
                        disabled={disabled}
                        // `aria-label` wins the accessible name over the child
                        // text, so it carries the same "…ing" swap the label
                        // draws — otherwise a screen reader would never hear
                        // that this exact row is the one out.
                        aria-label={
                          pressedRun === row.run && pending
                            ? `Removing ${row.run}`
                            : `Remove ${row.run}`
                        }
                        onClick={() => {
                          setPressedRun(row.run);
                          onRemove(row.run);
                        }}
                      >
                        {pressedRun === row.run && pending ? "Removing…" : "Remove"}
                      </Button>
                    </li>
                  ))}
                </ul>
              )}
              <Said>{allowedSaid}</Said>
            </div>
          )}

          {/* Read-only: removing one reaches every job against this repository,
              so it is the Manifest screen's act. It is here so a person can see
              why a drone was let through without leaving the job. */}
          {repositoryAllowed.length === 0 ? null : (
            <div className="armada-job-settings__allowed">
              <span className="armada-job-settings__label" id={`${group}-repository-allowed`}>
                Allowed for every job in this repository
              </span>
              <ul
                className="armada-job-settings__commands"
                aria-labelledby={`${group}-repository-allowed`}
              >
                {repositoryAllowed.map((run) => (
                  <li className="armada-job-settings__command" key={run}>
                    <span className="armada-job-settings__mono">{run}</span>
                  </li>
                ))}
              </ul>
              <p className="armada-job-settings__means">
                Removing one is the Manifest screen's, where it reaches every job at once.
              </p>
            </div>
          )}
        </DestinationCard>

        {judgeChoices === undefined || whenRefused === undefined ? null : (
          <DestinationCard label="Judge">
            <RadioGroup label="When a judge refuses">
              {judgeChoices.map((choice) => (
                <div className="armada-job-settings__option" key={choice.value}>
                  <Radio
                    name={`${group}-when-refused`}
                    value={choice.value}
                    checked={whenRefused === choice.value}
                    disabled={disabled}
                    aria-describedby={`${group}-${choice.value}`}
                    onChange={() => onWhenRefused?.(choice.value)}
                  >
                    {choice.label}
                  </Radio>
                  <p className="armada-job-settings__option-means" id={`${group}-${choice.value}`}>
                    {choice.means}
                  </p>
                </div>
              ))}
            </RadioGroup>
            <Said>{whenRefusedSaid}</Said>
          </DestinationCard>
        )}
      </div>
    </div>
  );
}

/**
 * One ceiling: the cap in a box of its own with `Raise` against it, and what
 * has gone against the cap on the row beneath.
 *
 * **The spend is not boxed and the cap is.** A box says a person set this
 * value; a spend is a reading, and drawing the two alike is how
 * `$20.00, ~$1.80 spent` came out as one run of grey text.
 */
function Ceiling({
  name,
  label,
  spent,
  raiseLabel,
  ceiling,
  disabled,
}: {
  /**
   * `data-ceiling` on the act — how a press elsewhere finds it. Pulse's Spend
   * and Turns open Settings on the cap they read, and bring its row into view.
   */
  name: "cost" | "turns";
  label: string;
  /** What the row beneath calls what has gone against the cap. */
  spent: string;
  raiseLabel: string;
  ceiling: JobSettingsCeiling;
  disabled: boolean;
}) {
  return (
    <>
      <ProposalField
        label={label}
        beneath={<Said>{ceiling.said}</Said>}
        trailing={
          // The mark goes on the wrapper rather than the button, because what
          // reads it looks for a button inside.
          <span data-ceiling={name}>
            {/* `Raise` on the face and the ceiling in the name: the row's label
                already says which, and the dialog it opens names the act. */}
            <Button
              variant="secondary"
              size="sm"
              ground="card"
              pending={ceiling.pending === true}
              disabled={disabled}
              aria-label={ceiling.pending === true ? `${raiseLabel}, waiting on Fleet` : raiseLabel}
              onClick={ceiling.onRaise}
            >
              {ceiling.pending === true ? "Raising…" : "Raise"}
            </Button>
          </span>
        }
      >
        <span className="armada-job-settings__figure">{ceiling.cap}</span>
      </ProposalField>
      <ProposalField label={spent} bare>
        <span className="armada-job-settings__against">{ceiling.used}</span>
      </ProposalField>
    </>
  );
}

/**
 * The line under a row once a change took, and when it applies. A live region,
 * because the change was made with a press and the answer arrives a round trip
 * later — a person who pressed and looked away is told without looking back.
 */
function Said({ children }: { children: ReactNode }) {
  if (children === undefined || children === null) return null;
  return (
    <p className="armada-job-settings__said" role="status">
      {children}
    </p>
  );
}

/**
 * The header's way into the panel, and how much on it differs from how a Job
 * starts.
 *
 * **Secondary, and beside the acts rather than among them.** It ends nothing
 * and it is not the screen's decision, so it takes no accent; it sits left of
 * the act group because that is where the header's controls are, and the
 * count is what lets a person see a Job was changed without opening anything.
 * **The count is quiet**: a changed Job is not a problem, so it is `--fg-muted`
 * text inside the button rather than a badge, which is a Job state and nothing
 * else.
 */
export function JobSettingsButton({
  changed,
  onOpen,
}: {
  /** How many settings differ from a new Job's. Nothing is drawn at zero. */
  changed: number;
  onOpen: () => void;
}) {
  return (
    <Button variant="secondary" onClick={onOpen}>
      Job settings
      {changed > 0 ? (
        <span className="armada-job-settings-button__count">{`${changed} changed`}</span>
      ) : null}
    </Button>
  );
}
