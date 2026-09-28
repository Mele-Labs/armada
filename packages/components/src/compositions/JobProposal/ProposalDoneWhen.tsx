import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { GuideMark } from "../GuideMark/GuideMark";
import { GUIDE_CRITERIA } from "../../guides";

/**
 * One thing the Job is held to, with where its words came from.
 *
 * **Where they came from is not what will decide them.** A criterion read out
 * of an issue may be decided by a Check, and one you typed may be decided by
 * the Judge; the two facts are drawn as two, because collapsing them is what
 * would make "from the issue" sound like a verdict.
 */
export type ProposalCriterion = {
  /** Stable for the life of the proposal, where anything has minted one. */
  id?: string;
  text: string;
  /** Where the words came from, as a person reads it. */
  origin: string;
  /**
   * The issue those words came from, where they came from one.
   *
   * **`ref` draws as a link only where `url` is there.** A reference on its
   * own is text — `armada/1162` is a repository, a path and a branch as
   * readily as an issue, which is what the word beside it in `origin` answers.
   */
  issue?: { ref: string; url?: string };
  /**
   * What will decide whether this is met — a Check, the Judge, or you.
   *
   * **A future, and it reads as one.** Nothing on this screen has run.
   */
  decidedBy: string;
  /**
   * The issue these words came from has been edited since the Job froze them.
   *
   * **The Job keeps what it froze** (#1530, 22 Sep). This says the source has
   * moved; it never replaces the words, and nothing here re-reads the issue.
   */
  movedSince?: string;
};

export type ProposalDoneWhenProps = {
  criteria: readonly ProposalCriterion[];
  /** One criterion reworded. Absent draws them frozen, which is after approval. */
  onCriterion?: (at: number, text: string) => void;
  /** One more line, appended empty for somebody to write. */
  onAdd?: () => void;
  /** One line taken off. */
  onRemove?: (at: number) => void;
  /** Open the issue a criterion's words came from, where an address is known. */
  onOpenIssue?: (ref: string) => void;
};

/**
 * What the Job is held to.
 *
 * **Nothing here says what a criterion is.** A line under the list said the
 * Judge marks against these words, and said it two ways — one before approval
 * and one after. Both were true of a Job that never ran, so they are guide 10
 * and the `?` on the heading (#1602). Where each line came from and what will
 * decide it stays on the row, because that is this Job's own.
 *
 * **A card, and the rows inside it are not boxed** (`xma8`, 28 Sep). Each row
 * was a filled well holding a filled field, on a ground the same colour again,
 * so nothing separated the thing you type in from the thing around it. The
 * card is `--bg-raised` and the field is the `--bg-sunken` it always was,
 * which is the one relationship the contract states for an input.
 */
export function ProposalDoneWhen({
  criteria,
  onCriterion,
  onAdd,
  onRemove,
  onOpenIssue,
}: ProposalDoneWhenProps) {
  return (
    <section className="armada-proposal__done-when" aria-label="Done when">
      <div className="armada-proposal__heading-row">
        <h3 className="armada-proposal__heading">Done when</h3>
        <GuideMark guide={GUIDE_CRITERIA} />
      </div>
      {criteria.length === 0 ? (
        <p className="armada-proposal__absent">
          {onAdd === undefined
            ? "Nothing was read out of a request or an issue, so this Job is held to the workflow alone."
            : "Nothing was read out of a request or an issue. Add a line, or leave this Job held to the workflow alone."}
        </p>
      ) : (
        <ul className="armada-proposal__criteria">
          {criteria.map((criterion, at) => (
            <li className="armada-proposal__criterion" key={criterion.id ?? at}>
              {onCriterion === undefined ? (
                <p className="armada-proposal__criterion-text">{criterion.text}</p>
              ) : (
                <div className="armada-proposal__criterion-row">
                  <Input
                    aria-label={`Criterion ${at + 1}`}
                    placeholder="What has to be true"
                    value={criterion.text}
                    onChange={(event) => onCriterion(at, event.target.value)}
                  />
                  {onRemove === undefined ? null : (
                    <Button
                      variant="ghost"
                      size="sm"
                      aria-label={`Remove criterion ${at + 1}`}
                      onClick={() => onRemove(at)}
                    >
                      Remove
                    </Button>
                  )}
                </div>
              )}
              <p className="armada-proposal__criterion-origin">
                <Origin criterion={criterion} {...(onOpenIssue === undefined ? {} : { onOpenIssue })} />
                <span className="armada-proposal__criterion-dot" aria-hidden="true">
                  ·
                </span>
                <span>{criterion.decidedBy}</span>
              </p>
              {criterion.movedSince === undefined ? null : (
                <p className="armada-proposal__moved" role="note">
                  {`The issue has been edited since these words were frozen — last on ${criterion.movedSince}. The Job is held to the words above.`}
                </p>
              )}
            </li>
          ))}
        </ul>
      )}
      {onAdd === undefined ? null : (
        <div className="armada-proposal__criteria-foot">
          <Button variant="secondary" size="sm" onClick={onAdd}>
            Add a criterion
          </Button>
        </div>
      )}
    </section>
  );
}

/**
 * Where one criterion's words came from.
 *
 * **The reference opens the issue where an address is known, and reads as
 * text where none is.** Bridge opens nothing it was handed as a string from a
 * click handler — `apps/desktop/src/main/forge.ts` — so the press names the
 * reference and main resolves it, which is why this is a control and not an
 * anchor. Nothing on the wire carries the address today.
 */
function Origin({
  criterion,
  onOpenIssue,
}: {
  criterion: ProposalCriterion;
  onOpenIssue?: (ref: string) => void;
}) {
  const issue = criterion.issue;
  if (issue === undefined) return <span>{criterion.origin}</span>;
  const ref =
    issue.url === undefined || onOpenIssue === undefined ? (
      <span className="armada-proposal__criterion-ref">{issue.ref}</span>
    ) : (
      <button
        type="button"
        className="armada-proposal__criterion-link"
        onClick={() => onOpenIssue(issue.ref)}
      >
        {issue.ref}
      </button>
    );
  return (
    <span className="armada-proposal__criterion-from">
      {criterion.origin} {ref}
    </span>
  );
}
