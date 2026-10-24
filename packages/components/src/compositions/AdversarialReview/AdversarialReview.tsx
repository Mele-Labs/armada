import { useEffect, useRef } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import { ArrowRight, Ban, CircleCheck, ClockAlert, LoaderCircle, ScanSearch, X } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { ConcernClaim } from "../SessionThread/ConcernClaim";

/**
 * An adversary reads a thing and says how it breaks. **It knows nothing of what it read**: a plan today, a diff or a
 * design tomorrow. The host hands it a state and the callbacks, and draws the two parts where it likes: the press
 * (`AdversarialReviewPress`) in a row of acts, the findings (`AdversarialFindings`) in the card, or in a list of
 * calls. `AdversarialReview` draws both, stacked.
 *
 * **A finding is a claim**, drawn as a judge's concern is: the consequence heads it, then Expected and Found, each
 * clamped to two lines. Its acts sit in one column, so a list of them reads down. A finding sent or dismissed folds
 * to one muted line. No count of findings is drawn anywhere.
 */
export type Finding = {
  id: string;
  /** What breaks, in a line. Heads the finding. */
  consequence: string;
  expected: string;
  found: string;
  /** What it cites, as `path:line`. */
  cites?: readonly string[];
  /** What it is about: a task id, a group id, or nothing for the plan as a whole. A host that draws the plan pins the finding there (`AnchoredFindings`). */
  about?: string;
  /** Acted on: the finding folds to a line. */
  settled?: "sent" | "dismissed";
};

export type AdversarialReviewState =
  | { state: "idle" }
  | { state: "running" }
  /** No findings is an empty list: one quiet mark and no sentence. */
  | { state: "done"; findings: readonly Finding[] }
  /** Fleet's words, as it said them. */
  | { state: "failed"; said: string }
  /** The review read an older version of the thing. `read` is that version; its findings stay where they are. */
  | { state: "stale"; read: number; findings?: readonly Finding[] };

export type AdversarialReviewProps = {
  review: AdversarialReviewState;
  onRun: () => void;
  onSend: (id: string) => void;
  onDismiss: (id: string) => void;
  /** The key that runs it, shown in the tooltip. The host listens for it. */
  shortcut?: string;
};

const RUNNING = "Adversarial review running";

/** The findings a state carries, none where the state has none. */
export function findingsOf(review: AdversarialReviewState): readonly Finding[] {
  return review.state === "done" || review.state === "stale" ? (review.findings ?? []) : [];
}

function Mark({ said, children, turning }: { said: string; children: ReactNode; turning?: true }) {
  return (
    <Tooltip label={said}>
      <span className="armada-adversarial__mark" role="img" aria-label={said} data-turning={turning}>
        {children}
      </span>
    </Tooltip>
  );
}

/**
 * The press, and what stands in its place: an idle review is a button; running, done, failed and stale are one mark
 * with a tooltip, and a failed or stale one offers Run again beside it.
 */
export function AdversarialReviewPress({ review, onRun, shortcut }: Pick<AdversarialReviewProps, "review" | "onRun" | "shortcut">) {
  const again = (
    <Tooltip label="Reads it again" {...(shortcut === undefined ? {} : { shortcut })}>
      <Button size="sm" variant="ghost" onClick={onRun}>
        Run again
      </Button>
    </Tooltip>
  );
  return (
    <span className="armada-adversarial__press" role="group" aria-label="Adversarial review">
      {review.state === "idle" ? (
        <Tooltip label="An adversary reads it for ways it breaks" {...(shortcut === undefined ? {} : { shortcut })}>
          <Button size="sm" variant="secondary" onClick={onRun}>
            Adversarial review
          </Button>
        </Tooltip>
      ) : review.state === "running" ? (
        <Mark said={RUNNING} turning>
          <LoaderCircle size={16} strokeWidth={2} aria-hidden />
        </Mark>
      ) : review.state === "done" ? (
        review.findings.length === 0 ? (
          <Mark said="No findings">
            <CircleCheck size={16} strokeWidth={2} aria-hidden />
          </Mark>
        ) : (
          <Mark said="Adversarial review done">
            <ScanSearch size={16} strokeWidth={2} aria-hidden />
          </Mark>
        )
      ) : review.state === "failed" ? (
        <>
          <Mark said={review.said}>
            <X size={16} strokeWidth={2} aria-hidden />
          </Mark>
          {again}
        </>
      ) : (
        <>
          <Mark said={`Read v${review.read}`}>
            <ClockAlert size={16} strokeWidth={2} aria-hidden />
          </Mark>
          {again}
        </>
      )}
    </span>
  );
}

/** One finding: the claim on the left, its two acts in a column on the right. Focusable; Enter sends, Backspace or D dismisses. */
export function FindingRow({ finding, onSend, onDismiss, hold }: { finding: Finding; onSend: () => void; onDismiss: () => void; hold?: (el: HTMLElement | null) => void }) {
  return (
    <li ref={hold} onClick={(event) => event.stopPropagation()} className="armada-adversarial__finding" role="group" aria-label={`Finding: ${finding.consequence}`} tabIndex={0} data-finding={finding.id}>
      <div className="armada-adversarial__claim">
        <ConcernClaim concern={{ text: finding.consequence, consequence: finding.consequence, expected: finding.expected, produced: finding.found, ...(finding.cites === undefined ? {} : { cites: finding.cites }) }} />
      </div>
      <div className="armada-adversarial__acts" role="group" aria-label="Answers">
        <Tooltip label="Goes to the agent as your message" shortcut="Enter">
          <Button size="sm" variant="secondary" onClick={onSend}>
            Send to agent
          </Button>
        </Tooltip>
        <Tooltip label="Leaves it unsent" shortcut="D">
          <Button size="sm" variant="ghost" onClick={onDismiss}>
            Dismiss
          </Button>
        </Tooltip>
      </div>
    </li>
  );
}

/** A finding acted on, as one muted line: a mark with its tooltip, then the consequence. */
export function SettledRow({ finding }: { finding: Finding }) {
  const sent = finding.settled === "sent";
  const said = sent ? "Sent" : "Dismissed";
  return (
    <li onClick={(event) => event.stopPropagation()} className="armada-adversarial__settled" role="group" aria-label={`Finding ${said.toLowerCase()}: ${finding.consequence}`}>
      <Mark said={said}>{sent ? <ArrowRight size={12} strokeWidth={2} aria-hidden /> : <Ban size={12} strokeWidth={2} aria-hidden />}</Mark>
      <span className="armada-adversarial__settled-text">{finding.consequence}</span>
    </li>
  );
}

/**
 * Where the findings go. A failed review's words stand here in one line. Arrow keys and Tab move through the findings
 * not yet acted on; the one in focus is sent with Enter and dismissed with Backspace or D, and focus passes to the next.
 */
export function AdversarialFindings({ review, onSend, onDismiss }: Pick<AdversarialReviewProps, "review" | "onSend" | "onDismiss">) {
  const found = findingsOf(review);
  const held = useRef(new Map<string, HTMLElement>());
  const next = useRef<string | undefined>(undefined);
  useEffect(() => {
    const id = next.current;
    next.current = undefined;
    if (id !== undefined) held.current.get(id)?.focus();
  }, [found]);
  if (review.state === "failed") {
    return (
      <p className="armada-adversarial__failed" role="status">
        {review.said}
      </p>
    );
  }
  if (found.length === 0) return null;
  const live = found.filter((one) => one.settled === undefined);
  const act = (id: string, run: (id: string) => void) => {
    const at = live.findIndex((one) => one.id === id);
    next.current = (live[at + 1] ?? live[at - 1])?.id;
    run(id);
  };
  const onKeyDown = (event: KeyboardEvent<HTMLUListElement>) => {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    const target = (event.target as HTMLElement).closest<HTMLElement>("[data-finding]");
    // A press on one of the buttons keeps its own key; the finding's keys are for the finding itself.
    if (target === null || event.target !== target) return;
    const id = target.dataset.finding!;
    const at = live.findIndex((one) => one.id === id);
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      event.stopPropagation();
      held.current.get(live[Math.min(live.length - 1, Math.max(0, at + (event.key === "ArrowDown" ? 1 : -1)))]!.id)?.focus();
    } else if (event.key === "Enter") {
      event.preventDefault();
      event.stopPropagation();
      act(id, onSend);
    } else if (event.key === "Backspace" || event.key === "d" || event.key === "D") {
      event.preventDefault();
      event.stopPropagation();
      act(id, onDismiss);
    }
  };
  return (
    <ul className="armada-adversarial__findings" aria-label="Adversarial findings" onKeyDown={onKeyDown}>
      {found.map((one) =>
        one.settled === undefined ? (
          <FindingRow
            key={one.id}
            finding={one}
            onSend={() => act(one.id, onSend)}
            onDismiss={() => act(one.id, onDismiss)}
            hold={(el) => {
              if (el === null) held.current.delete(one.id);
              else held.current.set(one.id, el);
            }}
          />
        ) : (
          <SettledRow key={one.id} finding={one} />
        ),
      )}
    </ul>
  );
}

/** The press and the findings together, findings over the press. */
export function AdversarialReview(props: AdversarialReviewProps) {
  return (
    <div className="armada-adversarial">
      <AdversarialFindings review={props.review} onSend={props.onSend} onDismiss={props.onDismiss} />
      <AdversarialReviewPress review={props.review} onRun={props.onRun} {...(props.shortcut === undefined ? {} : { shortcut: props.shortcut })} />
    </div>
  );
}
