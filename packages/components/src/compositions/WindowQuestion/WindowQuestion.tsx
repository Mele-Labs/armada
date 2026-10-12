import type { CSSProperties, KeyboardEvent } from "react";

import { Button } from "../../primitives/Button/Button";
import { Textarea } from "../../primitives/Textarea/Textarea";
import type { CaptureBox } from "../StudioCapture/StudioCapture";

/**
 * A Session's question about a place on the page its walk window shows — `ask_in_window`,
 * `docs/concepts/session.md`. The owner, 11 Oct 2026: a question about the walk takes him where the
 * agent is asking about, highlighted like an annotation, with a card holding the question and his
 * options to answer.
 *
 * **Drawn by Bridge's bar over the page, never in it**, as Studio capture's outline is: the page
 * cannot read it, restyle it or forge it. It draws and it does not look; where the place is comes
 * from main.
 *
 * **Answering is one press.** An option sends at once, carrying any words typed beside it; words
 * alone go with Send. Look around folds the card into the bar so the page takes presses again.
 */

export type WindowQuestionOption = { label: string; description?: string };

export type WindowQuestionProps = {
  /** Who asks: the Session's title. */
  from: string;
  question: string;
  options: readonly WindowQuestionOption[];
  /** The place asked about. Absent while it is looked for; `null` where nothing on the page matched. */
  box?: CaptureBox | null;
  /** What is being typed. Controlled: the window owns the draft. */
  answer: string;
  onAnswer: (answer: string) => void;
  /** An option pressed, by its index. */
  onChoose: (choice: number) => void;
  /** The typed words, sent alone. */
  onSend: () => void;
  /** Fold the card into the bar so the page can be used. */
  onFold: () => void;
  /** Out to Fleet: nothing takes a second press. */
  sending?: boolean;
  /** Why the last answer did not land, or absent. */
  refused?: string;
};

const place = (box: CaptureBox): CSSProperties =>
  ({
    "--question-x": `${box.x}px`,
    "--question-y": `${box.y}px`,
    "--question-w": `${box.width}px`,
    "--question-h": `${box.height}px`,
  }) as CSSProperties;

/** Under the place where there is room below it, and over it where there is not: Studio capture's rule. */
function cardAt(box: CaptureBox, height: number): { style: CSSProperties; side: "below" | "above" } {
  const below = box.y + box.height < height * 0.6;
  return { style: place({ ...box, y: below ? box.y + box.height : box.y }), side: below ? "below" : "above" };
}

export function WindowQuestion(props: WindowQuestionProps) {
  const { from, question, options, box, answer, onAnswer, onChoose, onSend, onFold, sending = false, refused } = props;
  const sendable = !sending && answer.trim() !== "";
  const card = box === undefined || box === null ? null : cardAt(box, window.innerHeight);

  function keyed(event: KeyboardEvent<HTMLDivElement>): void {
    if (event.key === "Escape") {
      event.stopPropagation();
      onFold();
    } else if (event.key === "Enter" && event.metaKey && sendable) {
      event.preventDefault();
      event.stopPropagation();
      onSend();
    }
  }

  return (
    <div className="armada-window-question" data-armada-capture="" onKeyDown={keyed}>
      {/* A press beside the card is a person reaching for the page: the card folds and lets them. */}
      <div className="armada-window-question__catch" aria-hidden onClick={onFold} />
      {box === undefined || box === null ? null : (
        <div className="armada-window-question__outline" role="img" aria-label="The place asked about" style={place(box)} />
      )}
      <div
        className="armada-window-question__card"
        role="dialog"
        aria-label={`Question from ${from}`}
        {...(card === null ? { "data-side": "top" } : { style: card.style, "data-side": card.side })}
      >
        <p className="armada-window-question__from">{from} asks</p>
        <p className="armada-window-question__question">{question}</p>
        {box === null ? <p className="armada-window-question__lost">The place it asks about is not on this page.</p> : null}
        {options.length === 0 ? null : (
          <div className="armada-window-question__options" role="group" aria-label="Options">
            {options.map((one, at) => (
              <Button
                key={`${at}:${one.label}`}
                variant="secondary"
                size="sm"
                disabled={sending}
                onClick={() => onChoose(at)}
              >
                <span className="armada-window-question__label">{one.label}</span>
                {one.description === undefined ? null : (
                  <span className="armada-window-question__says">{one.description}</span>
                )}
              </Button>
            ))}
          </div>
        )}
        <Textarea
          autoFocus={options.length === 0}
          label={options.length === 0 ? "Answer" : "Or in your own words"}
          value={answer}
          onChange={(event) => onAnswer(event.target.value)}
        />
        {refused === undefined ? null : <p className="armada-window-question__refused">{refused}</p>}
        <div className="armada-window-question__acts">
          <Button variant="ghost" size="sm" onClick={onFold}>
            Look around
          </Button>
          <Button variant="primary" size="sm" pending={sending} disabled={!sendable} onClick={onSend}>
            {sending ? "Sending" : "Send"}
          </Button>
        </div>
      </div>
    </div>
  );
}
