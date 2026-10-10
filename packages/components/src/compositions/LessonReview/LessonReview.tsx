import { ChevronDown, ChevronRight } from "lucide-react";
import { useState, type ReactNode } from "react";

import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { Checkbox } from "../../primitives/Checkbox/Checkbox";
import { Input } from "../../primitives/Input/Input";
import { SkeletonText } from "../../primitives/Skeleton/Skeleton";

/** One turn of the thread under a card. */
export type ReviewTurn = { role: "person" | "fleet"; text: string };

/** The question thread under one card, held by the surface so it outlives a move to another card. */
export type ReviewThreadProps = {
  turns: readonly ReviewTurn[];
  draft: string;
  onDraft: (text: string) => void;
  onSend: () => void;
  /** A question is out. */
  pending?: boolean;
  /** Fleet's words for a question it refused or could not answer. */
  failure?: string;
};

/**
 * Questions about one card and Fleet's answers, oldest first. **Nothing is
 * kept past the review**: Fleet stores no thread, so Bridge carries it.
 */
export function ReviewThread({ turns, draft, onDraft, onSend, pending = false, failure }: ReviewThreadProps) {
  return (
    <section className="armada-review__thread" aria-label="Ask about this item">
      {turns.length === 0 ? null : (
        <ol className="armada-review__turns">
          {turns.map((turn, at) => (
            <li key={at} className="armada-review__turn" data-role={turn.role}>
              <span className="armada-review__who">{turn.role === "person" ? "You" : "Fleet"}</span>
              <p className="armada-review__said">{turn.text}</p>
            </li>
          ))}
        </ol>
      )}
      {pending ? <SkeletonText /> : null}
      {failure === undefined ? null : (
        <Alert tone="escalated" title="The question was not answered">
          {failure}
        </Alert>
      )}
      <form
        className="armada-review__ask"
        onSubmit={(event) => {
          event.preventDefault();
          if (!pending && draft.trim() !== "") onSend();
        }}
      >
        <Input
          aria-label="Ask about this item"
          placeholder="Ask about this item"
          value={draft}
          disabled={pending}
          onChange={(event) => onDraft(event.target.value)}
        />
        <Button type="submit" variant="secondary" pending={pending} disabled={pending || draft.trim() === ""}>
          Send
        </Button>
      </form>
    </section>
  );
}

/** A duplicate the card stands for. */
export type ReviewMerged = { id: string; label: string; title: string };

export type LessonReviewStepProps = {
  /** One-based place among the queue the review made, and how long that queue is. */
  at: number;
  of: number;
  /** The model's one line on why this one is worth a person's time. */
  reason: string;
  /** The duplicates this card stands for. Absent where it stands for none. */
  merged?: readonly ReviewMerged[];
  /** Whether the duplicates are rejected with it. Off until the person turns it on. */
  duplicates?: { checked: boolean; onChange: (checked: boolean) => void };
  /** Fleet's words for duplicates it did not reject. */
  duplicatesFailure?: string;
  /** Absent on the first card. */
  onBack?: () => void;
  onSkip: () => void;
  /** The card, drawn as a one-row `LessonList`. */
  children: ReactNode;
  thread: ReviewThreadProps;
};

/**
 * One item of the guided review: where it stands in the queue, why the model
 * put it here, the card itself and a thread to ask about it.
 *
 * **The card is the Lessons list's own**, passed in, so the answers read and
 * act as they do on the list. Counts here (`3 of 12`, `2 other Jobs`) stand
 * where the items are not drawn, which hard rule 7 allows.
 */
export function LessonReviewStep({
  at,
  of,
  reason,
  merged = [],
  duplicates,
  duplicatesFailure,
  onBack,
  onSkip,
  children,
  thread,
}: LessonReviewStepProps) {
  const [showing, setShowing] = useState(false);
  return (
    <section className="armada-review" aria-label="Review">
      <div className="armada-review__head">
        <p className="armada-review__progress">
          {at} of {of}
        </p>
        <div className="armada-review__nav">
          <Button variant="ghost" size="sm" disabled={onBack === undefined} onClick={onBack}>
            Back
          </Button>
          <Button variant="ghost" size="sm" onClick={onSkip}>
            Skip
          </Button>
        </div>
      </div>
      <div className="armada-review__why">
        <span className="armada-review__label">Why this one</span>
        <p className="armada-review__reason">{reason}</p>
      </div>
      {merged.length === 0 ? null : (
        <div className="armada-review__merged">
          <Button variant="ghost" size="sm" aria-expanded={showing} onClick={() => setShowing((was) => !was)}>
            {showing ? <ChevronDown size={14} strokeWidth={2} aria-hidden /> : <ChevronRight size={14} strokeWidth={2} aria-hidden />}
            {seenIn(merged)}
          </Button>
          {!showing ? null : (
            <ul className="armada-review__twins">
              {merged.map((one) => (
                <li key={one.id} className="armada-review__twin">
                  <span className="armada-review__twin-label">{one.label}</span>
                  <span className="armada-review__twin-title">{one.title}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
      {children}
      {duplicates === undefined || merged.length === 0 ? null : (
        <Checkbox checked={duplicates.checked} onChange={(event) => duplicates.onChange(event.target.checked)}>
          Also reject the {merged.length} {merged.length === 1 ? "duplicate" : "duplicates"}
        </Checkbox>
      )}
      {duplicatesFailure === undefined ? null : (
        <Alert tone="escalated" title="Some duplicates were not rejected">
          {duplicatesFailure}
        </Alert>
      )}
      <ReviewThread {...thread} />
    </section>
  );
}

/** `Also seen in 2 other Jobs`: a duplicate from a Session is not a Job, so the words widen. */
function seenIn(merged: readonly ReviewMerged[]): string {
  const sessions = merged.some((one) => one.label.includes(" · "));
  return `Also seen in ${merged.length} other ${sessions ? "places" : merged.length === 1 ? "Job" : "Jobs"}`;
}

export type LessonReviewEndProps = {
  /** Why there is nothing to draw: nothing was open, or every one is answered. */
  title: string;
  /** Items passed over and still open, to go through again. */
  skipped?: number;
  onAgain?: () => void;
  onBack?: () => void;
  onList: () => void;
};

/** The end of the queue, or an empty one, with the way back to the list. */
export function LessonReviewEnd({ title, skipped = 0, onAgain, onBack, onList }: LessonReviewEndProps) {
  return (
    <section className="armada-review armada-review--end" aria-label="Review">
      <p className="armada-review__ended">{title}</p>
      <div className="armada-review__nav">
        {onBack === undefined ? null : (
          <Button variant="ghost" size="sm" onClick={onBack}>
            Back
          </Button>
        )}
        {skipped === 0 || onAgain === undefined ? null : (
          <Button variant="secondary" size="sm" onClick={onAgain}>
            Go through the {skipped} you skipped
          </Button>
        )}
        <Button variant="secondary" size="sm" onClick={onList}>
          Back to the list
        </Button>
      </div>
    </section>
  );
}

/** The review while Fleet reads, or when it could not. */
export function LessonReviewWait({ failure, onRetry }: { failure?: string; onRetry?: () => void }) {
  if (failure !== undefined) {
    return (
      <Alert
        tone="escalated"
        title="The review could not be made"
        action={
          onRetry === undefined ? undefined : (
            <Button size="sm" onClick={onRetry}>
              Retry
            </Button>
          )
        }
      >
        {failure}
      </Alert>
    );
  }
  return (
    <section className="armada-review" aria-label="Review" aria-busy="true">
      <p className="armada-review__label">Fleet is reading the open items</p>
      <SkeletonText />
    </section>
  );
}

/** One item the review left out of the queue. */
export type ReviewSetAsideItem = { id: string; title: string; why: string; onPutBack: () => void };

/** The items the review left out, folded, each with why and a way to put it back in the queue. */
export function ReviewSetAside({ items }: { items: readonly ReviewSetAsideItem[] }) {
  const [showing, setShowing] = useState(false);
  if (items.length === 0) return null;
  return (
    <section className="armada-review__aside" aria-label="Set aside">
      <Button variant="ghost" size="sm" aria-expanded={showing} onClick={() => setShowing((was) => !was)}>
        {showing ? <ChevronDown size={14} strokeWidth={2} aria-hidden /> : <ChevronRight size={14} strokeWidth={2} aria-hidden />}
        Set aside ({items.length})
      </Button>
      {!showing ? null : (
        <ul className="armada-review__set-aside">
          {items.map((one) => (
            <li key={one.id} className="armada-review__aside-item">
              <div className="armada-review__aside-text">
                <span className="armada-review__aside-title">{one.title}</span>
                <span className="armada-review__aside-why">{one.why}</span>
              </div>
              <Button variant="secondary" size="sm" onClick={one.onPutBack}>
                Put back in the queue
              </Button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
