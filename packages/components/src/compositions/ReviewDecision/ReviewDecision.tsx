import { useId, type ReactNode } from "react";
import { Button, STILL_WAITING, useStillWaiting, type ButtonAnswer } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { SplitButton, type SplitButtonItem } from "../../primitives/SplitButton/SplitButton";
import { Textarea } from "../../primitives/Textarea/Textarea";

/**
 * Review decision — the answers to a Job waiting at a human gate, and the note
 * one of them carries.
 *
 * **The note is on this surface, never behind a control.**
 * `docs/practices/bridge.md`: reviewing a Drone's output and replying to it is
 * one continuous interaction, and a design that puts the reply in a separate
 * route, tab or modal from the diff recreates v1's problem inside Electron. So
 * the field sits where the diff is, already open, with nothing to press to
 * reach it.
 *
 * **Four acts, in two pairs, one split button each** — the owner's own
 * arrangement, 30 Sep 2026. Three of the four are recoverable, and that is
 * carried by a sentence on hover rather than by a shade of red:
 *
 * | Act | Where it sits | What survives |
 * |---|---|---|
 * | Merge | the accent face, **only where there is a pull request** | the work lands, and Fleet runs the repository's after-merge checks |
 * | Approve | behind that caret | the work is taken and the pull request is left open |
 * | Request changes | the second face | the drone, the worktree and the step — it goes back to work |
 * | Reject | behind that caret, `danger`, last | **nothing. Terminal, and it ends the drone** |
 *
 * **Merge takes the primary fill from Approve when it is offered, and that is
 * the point of it.** A job holding an open pull request has one ordinary
 * ending, and it is not "record this done and leave the branch on the forge" —
 * that is the state the face exists to stop. Where there is no pull request
 * the prop is absent, no caret is drawn over Approve, and Approve is a primary
 * button on its own: a split button with nothing in its menu is a button.
 *
 * **Reject behind a caret reverses this component's own earlier rule**, and
 * the owner reversed it. It sat below a rule, alone, so that a split button
 * could not make a terminal act read as a variant of the face above it. What
 * carries that now is `danger` — last in the list, `--status-completed-failed`
 * — and its hover, which says the Job ends and the Drone is stopped.
 *
 * **Approve does not confirm; merge and reject do.** Approving is the ordinary
 * path — it is why the gate exists, and asking twice for the common case is a
 * gate in the wrong place. Rejecting ends two things. Merging is the one act
 * here that writes into a repository Fleet did not make, and nothing in Bridge
 * takes it back. Both are the caller's to confirm, and this only asks.
 *
 * **Request changes is refused with a blank note**, before the press, matching
 * the 422 Fleet gives it. It is a face, so `faceDisabled` refuses it and
 * Reject behind the same caret stays reachable — a blank note is a reason not
 * to send words, never a reason not to end a Job.
 *
 * **Merge can be drawn and disabled at once.** `#663`: a branch behind main
 * with conflicts is a pull request Fleet would refuse to merge, and
 * `mergeBlockedReason` says so under the row rather than hiding it. Again
 * `faceDisabled`, so Approve behind that caret is still one press away. Fleet
 * sends a Drone to clear the conflicts on its own, so nothing here presses
 * anything.
 *
 * **No glyph on any of them.** Primary and secondary are label-only by
 * contract, and the one mark here is each control's caret, which `SplitButton`
 * owns. The labels say what each does; the hovers say what survives — and
 * every one of the four carries one, because two of the acts no longer have a
 * face of their own to be read from.
 *
 * **A card on the canvas, so it takes `armada-glass`** — `src/glass.css`. It
 * is a sibling of the comments below it and of Overview's cards, and it was
 * the one region among them with no card treatment at all.
 */
export type ReviewDecisionProps = {
  /** The reviewer's own words. Controlled — the caller holds the draft. */
  note: string;
  onNote: (note: string) => void;
  /** What should change, listed above the note. **The list is the note**: the caller sends both as one. #907. */
  changes?: DecisionChange[];
  /** Take one change off the list. Absent leaves the list as it stands. */
  onRemoveChange?: (id: string) => void;
  /**
   * Ask to merge the pull request. **The caller confirms**,
   * because this writes into a repository Fleet did not make and Bridge cannot
   * undo it — the same shape as `onReject`, and for the other of the two
   * reasons an answer here is worth a second press.
   *
   * **Absent is a job with no pull request to merge** — a workflow that
   * declares no delivering step opened none, and so did one whose push failed.
   * Presence is the whole of what decides this control, because a merge Fleet
   * would refuse must not be a button a person can reach.
   */
  onMerge?: () => void;
  /**
   * Why merging is blocked, shown under the row rather than folded into
   * `disabledNote` — the rest of the group stays live while this one alone is
   * not. **Presence disables the merge face regardless of `disabled`.**
   * `#663`: a branch behind main with conflicts is not a pull request Fleet
   * can land, and a button that fails on the press is worse than one that
   * says so first.
   */
  mergeBlockedReason?: ReactNode;
  /**
   * Merge is drawn above, as `MergeAct` beside the pull request (owner, 8 Oct 2026). Approve is then a
   * button of its own here, secondary, and the note and the reason belong to `MergeAct`.
   */
  mergeAbove?: boolean;
  /** Take the work, leaving the pull request where it is. Sent on the press. */
  onApprove: () => void;
  /** Send it back with the note. Refused while the note is blank. */
  onRequestChanges: () => void;
  /** Ask to reject. **The caller confirms**, because this ends two things. */
  onReject: () => void;
  /**
   * Every control off. A decision in flight, or nothing live to send it over —
   * the caller's sentence says which, since a disabled group with no reason is
   * a surface that looks broken.
   */
  disabled?: boolean;
  /** Why the controls are off, where they are. Never left to be guessed at. */
  disabledNote?: ReactNode;
  /**
   * The answer that was pressed and Fleet has not answered. The control
   * carrying that act waits and its face says which one is out — including an
   * act chosen behind the caret, which is what `SplitButton`'s `pendingLabel`
   * is for, and the face keeps that act's own label once `answered` takes
   * over. Every other control is off; absent is nothing out. #1117.
   */
  pending?: DecisionAct;
  /**
   * Fleet's answer to the last of the four pressed, drawn on that act's
   * control. **An act chosen behind a caret keeps its own label on the face
   * until this clears**, so the line and the words beside it name one act.
   * Pending wins.
   */
  answered?: ReviewDecisionAnswered;
  /** The label over the note field. Sentence case, no Wh- opener. */
  noteLabel?: string;
  /** The label over the listed changes, and the heading they are sent under. #907. */
  changesLabel?: string;
  /** What approving does, on hover over its entry. */
  approveNote?: ReactNode;
  /** What requesting changes does, on hover over its control. */
  requestChangesNote?: ReactNode;
  /** What merging does, on hover over its control. */
  mergeNote?: ReactNode;
  /** What rejecting costs, on hover over its entry. */
  rejectNote?: ReactNode;
  mergeLabel?: string;
  approveLabel?: string;
  requestChangesLabel?: string;
  rejectLabel?: string;
};

/** The four answers, named as the command that sends each. */
export type DecisionAct = "merge" | "approve" | "changes" | "reject";

/** Which of the four Fleet answered, and what it said. */
export type ReviewDecisionAnswered = { act: DecisionAct; answer: ButtonAnswer };

/** What each answer's control says while Fleet has not answered it. */
const UNDERWAY: Record<DecisionAct, string> = {
  merge: "Merging…",
  approve: "Approving…",
  changes: "Requesting changes…",
  reject: "Rejecting…",
};

/** What each caret is called, for a reader who cannot see it. */
const KEPT_MENU = "The other way to take this work";
const SENT_BACK_MENU = "The other way to end this review";

/** One thing that should change, and where it came from: `Small fix`, or a View. #907. */
export type DecisionChange = {
  id: string;
  from: string;
  text: string;
};

export function ReviewDecision({
  note,
  onNote,
  changes = [],
  onRemoveChange,
  onMerge,
  mergeBlockedReason,
  mergeAbove = false,
  onApprove,
  onRequestChanges,
  onReject,
  disabled: refused = false,
  disabledNote,
  pending,
  answered,
  noteLabel = "Notes",
  changesLabel = "What should change",
  approveNote = "Takes the work as the drone left it.",
  requestChangesNote = "Sends this note to the drone as a turn. It keeps the worktree and the step, and comes back running.",
  mergeNote = "Merges the pull request on its code host. Armada runs the repository's after-merge checks against what landed; merging it there yourself skips them.",
  rejectNote = "A verdict on the work, and the job ends there. The drone is stopped and nothing resumes it. Its branch stays where the drone left it.",
  mergeLabel = "Merge pull request",
  approveLabel = "Approve the work",
  requestChangesLabel = "Request changes",
  rejectLabel = "Reject the work",
}: ReviewDecisionProps) {
  const listed = changes.length > 0;
  const blank = note.trim() === "" && !listed;
  // Presence and never a flag: the caller has the pull request or it has not,
  // and a boolean beside a handler would let a surface offer an act with
  // nothing behind it.
  const merging = onMerge !== undefined && !mergeAbove;
  const disabled = refused || pending !== undefined;
  const stillWaiting = useStillWaiting(pending !== undefined);
  const mergeReasonId = useId();
  const answerOn = (act: DecisionAct) => (answered?.act === act ? answered.answer : undefined);
  // A control carries a pair, so it waits for either of its two acts and its
  // face says which of them is out.
  const waitingOn = (...acts: DecisionAct[]) => acts.some((act) => pending === act);
  const underway = pending === undefined ? undefined : UNDERWAY[pending];
  // What the face reads while Fleet's answer to the act behind the caret is
  // still drawn. **The answer keeps the label, not only the press** —
  // `pendingLabel` already holds it while the press is out, and the line is
  // drawn on the same face afterwards: a line meaning accepted beside a face
  // reading `Merge pull request` names the act that did not go out, at
  // the one moment a person is checking that the right one did. #1117's rule
  // is that the control pressed answers, and what was pressed was the entry.
  const faceOf = (lead: string, behind: DecisionAct, chosen: string) =>
    pending === undefined && answered?.act === behind ? chosen : lead;

  const approveEntry: SplitButtonItem = { label: approveLabel, note: approveNote, onSelect: onApprove };
  // `danger` is what keeps a terminal act from reading as a variant of the
  // recoverable one on the face above it: last, and in --status-completed-failed.
  const rejectEntry: SplitButtonItem = {
    label: rejectLabel,
    note: rejectNote,
    danger: true,
    onSelect: onReject,
  };

  return (
    <div className="armada-review-decision armada-glass">
      {listed ? (
        <div className="armada-review-decision__changes">
          <span className="armada-review-decision__label">{changesLabel}</span>
          <ul className="armada-review-decision__list" aria-label={changesLabel}>
            {changes.map((change) => (
              <li key={change.id} className="armada-review-decision__change">
                <div className="armada-review-decision__change-head">
                  <span className="armada-review-decision__from">{change.from}</span>
                  {onRemoveChange === undefined ? null : (
                    <Button
                      size="sm"
                      variant="ghost"
                      disabled={disabled}
                      aria-label={`Remove ${change.text}`}
                      onClick={() => onRemoveChange(change.id)}
                    >
                      Remove
                    </Button>
                  )}
                </div>
                <span className="armada-review-decision__text">{change.text}</span>
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {/* First, and always open. The reply is half of the loop this surface is,
          and a field behind a control is a second surface. */}
      <Textarea
        label={noteLabel}
        rows={4}
        value={note}
        disabled={disabled}
        onChange={(event) => onNote(event.target.value)}
      />

      <div className="armada-review-decision__acts">
        {/* The one accent fill on this surface, and it moves. A job with a pull
            request open has one ordinary ending and it is Merge, Approve behind
            its caret; a job with none draws Approve alone, because a split
            button with nothing in its menu is a button. */}
        {merging ? (
          <SplitButton
            variant="primary"
            note={mergeNote}
            items={[approveEntry]}
            menuLabel={KEPT_MENU}
            disabled={disabled}
            faceDisabled={mergeBlockedReason !== undefined}
            {...(mergeBlockedReason === undefined ? {} : { faceDescribedBy: mergeReasonId })}
            pending={waitingOn("merge", "approve")}
            pendingLabel={underway}
            answer={answerOn("merge") ?? answerOn("approve")}
            onAction={onMerge}
          >
            {faceOf(mergeLabel, "approve", approveLabel)}
          </SplitButton>
        ) : (
          <Button
            variant={mergeAbove ? "secondary" : "primary"}
            pending={pending === "approve"}
            answer={answerOn("approve")}
            disabled={disabled}
            onClick={onApprove}
          >
            {pending === "approve" ? UNDERWAY.approve : approveLabel}
          </Button>
        )}

        {/* The two answers that send the work back. The face is refused while
            the note is blank, which is what Fleet would answer; Reject behind
            the caret is not, because ending a Job never needed words. */}
        <SplitButton
          variant="secondary"
          note={requestChangesNote}
          items={[rejectEntry]}
          menuLabel={SENT_BACK_MENU}
          disabled={disabled}
          faceDisabled={blank}
          pending={waitingOn("changes", "reject")}
          pendingLabel={underway}
          answer={answerOn("changes") ?? answerOn("reject")}
          onAction={onRequestChanges}
        >
          {faceOf(requestChangesLabel, "reject", rejectLabel)}
        </SplitButton>
      </div>

      {/* Under the row so a long sentence never widens Merge's column;
          `faceDescribedBy` keeps it read as Merge's own reason. */}
      {mergeBlockedReason === undefined || mergeAbove ? null : (
        <p id={mergeReasonId} className="armada-review-decision__said" role="note">
          {mergeBlockedReason}
        </p>
      )}

      {stillWaiting ? (
        <p className="armada-review-decision__said" role="status">
          {STILL_WAITING}
        </p>
      ) : refused && disabledNote !== undefined ? (
        <p className="armada-review-decision__said" role="note">
          {disabledNote}
        </p>
      ) : null}
    </div>
  );
}

/**
 * Merge, drawn with the pull request it acts on rather than among the other answers. The same
 * act, props and answer as the face it replaces in `ReviewDecision`: pending, the line, the
 * reason it is off. **Only beside a pull request**, so `onMerge` is required here.
 */
export type MergeActProps = Pick<
  ReviewDecisionProps,
  "mergeBlockedReason" | "mergeNote" | "mergeLabel" | "disabled" | "disabledNote" | "pending" | "answered"
> & { onMerge: () => void };

export function MergeAct({
  onMerge,
  mergeBlockedReason,
  mergeNote = "Merges the pull request on its code host. Armada runs the repository's after-merge checks against what landed; merging it there yourself skips them.",
  mergeLabel = "Merge pull request",
  disabled: refused = false,
  disabledNote,
  pending,
  answered,
}: MergeActProps) {
  const reasonId = useId();
  const off = refused || pending !== undefined;
  const own = pending === "merge";
  return (
    <div className="armada-review-decision__merge">
      <Tooltip label={mergeNote}>
        <Button
          variant="primary"
          disabled={off || mergeBlockedReason !== undefined}
          pending={own}
          answer={answered?.act === "merge" ? answered.answer : undefined}
          {...(mergeBlockedReason === undefined ? {} : { "aria-describedby": reasonId })}
          onClick={onMerge}
        >
          {own ? UNDERWAY.merge : mergeLabel}
        </Button>
      </Tooltip>
      {mergeBlockedReason === undefined ? null : (
        <p id={reasonId} className="armada-review-decision__said" role="note">
          {mergeBlockedReason}
        </p>
      )}
      {refused && disabledNote !== undefined ? (
        <p className="armada-review-decision__said" role="note">
          {disabledNote}
        </p>
      ) : null}
    </div>
  );
}
