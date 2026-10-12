import { Box, LoaderCircle, SquareTerminal, X } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Card } from "../../primitives/Card/Card";
import { Kbd } from "../../primitives/Kbd/Kbd";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { OwnerChip } from "../OwnerChip/OwnerChip";

/**
 * A thought typed into Helm's dock, on its way to running work. Drawn at the end of the thread, over
 * the message box, where the dock's other asks are.
 *
 * **Three stages, one card.** While Helm reads where the words belong there is only a mark turning.
 * A match names the Session or Job and offers Send. Nothing matched, or No pressed, offers to start
 * a Session or a Job with the words. Every act is a button with its key on it, and the dock binds
 * the keys while the message box is empty.
 */
export type ThoughtTarget = {
  kind: "session" | "job";
  title: string;
  /** The repository's name, as the picker draws it. */
  repository: string;
  /** A Session that holds a slot: hovering its name draws the Session's card. */
  slot?: number;
};

export type ThoughtCardProps = {
  /** The owner's words, as typed. */
  words: string;
} & (
  | { stage: "routing" }
  | { stage: "match"; target: ThoughtTarget; onSend: () => void; onNo: () => void; onAskHelm: () => void }
  | { stage: "new"; onSession: () => void; onJob: () => void; onAskHelm: () => void; onDismiss: () => void }
);

/** The kind's glyph and the word for it, which are the Session card's and the Job chip's own. */
export const THOUGHT_KIND = {
  session: { Glyph: SquareTerminal, said: "Session" },
  job: { Glyph: Box, said: "Job" },
} as const;

/** Where a thought went or may go: the kind's glyph and the title, over the repository. */
export function ThoughtTargetLine({ target }: { target: ThoughtTarget }) {
  const { Glyph, said } = THOUGHT_KIND[target.kind];
  const name = (
    <span className="armada-thought-target">
      <Tooltip label={said}>
        <span className="armada-thought-target__kind" role="img" aria-label={said}>
          <Glyph size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      <span className="armada-thought-target__title">{target.title}</span>
    </span>
  );
  return (
    <div className="armada-thought-target-line">
      {target.kind === "session" && target.slot !== undefined ? (
        <OwnerChip chip={{ kind: "slot", slot: target.slot }} plain={name}>
          {name}
        </OwnerChip>
      ) : (
        name
      )}
      <span className="armada-thought-target__repository mono">{target.repository}</span>
    </div>
  );
}

function Act({ label, keys, onPress, tooltip }: { label: string; keys: string[]; onPress: () => void; tooltip?: string }) {
  const button = (
    <Button variant="secondary" size="sm" ground="sunken" onClick={onPress} aria-keyshortcuts={keys.join(" ")}>
      {label}
      <span className="armada-thought-card__keys" aria-hidden>
        {keys.map((key) => (
          <Kbd key={key}>{key}</Kbd>
        ))}
      </span>
    </Button>
  );
  return tooltip === undefined ? button : <Tooltip label={tooltip}>{button}</Tooltip>;
}

export function ThoughtCard(props: ThoughtCardProps) {
  const quote = <p className="armada-thought-card__words">{props.words}</p>;
  if (props.stage === "routing") {
    return (
      <Card className="armada-thought-card" flat role="status" aria-label="Finding where this goes" data-stage="routing">
        <Tooltip label="Finding where this goes">
          <span className="armada-thought-card__finding" aria-hidden>
            <LoaderCircle size={16} strokeWidth={2} />
          </span>
        </Tooltip>
        {quote}
      </Card>
    );
  }
  if (props.stage === "match") {
    return (
      <Card className="armada-thought-card" flat role="group" aria-label="Send to" data-stage="match">
        {quote}
        <span className="armada-thought-card__label">Send to</span>
        <ThoughtTargetLine target={props.target} />
        <div className="armada-thought-card__acts" role="group" aria-label="Answers">
          <Act label="Send" keys={["↵"]} onPress={props.onSend} />
          <Act label="No" keys={["N"]} onPress={props.onNo} />
          <Act label="Ask Helm" keys={["H"]} onPress={props.onAskHelm} tooltip="Put the words to Helm as they are" />
        </div>
      </Card>
    );
  }
  return (
    <Card className="armada-thought-card" flat role="group" aria-label="Start new" data-stage="new">
      {quote}
      <div className="armada-thought-card__head">
        <span className="armada-thought-card__label">Start new</span>
        <Tooltip label="Dismiss">
          <Button variant="ghost" size="sm" aria-label="Dismiss" aria-keyshortcuts="Escape" onClick={props.onDismiss}>
            <X size={12} strokeWidth={2} aria-hidden />
            <span className="armada-thought-card__keys" aria-hidden>
              <Kbd>Esc</Kbd>
            </span>
          </Button>
        </Tooltip>
      </div>
      <div className="armada-thought-card__acts" role="group" aria-label="Answers">
        <Act label="Session" keys={["S"]} onPress={props.onSession} />
        <Act label="Job" keys={["J"]} onPress={props.onJob} />
        <Act label="Ask Helm" keys={["H"]} onPress={props.onAskHelm} tooltip="Put the words to Helm as they are" />
      </div>
    </Card>
  );
}
