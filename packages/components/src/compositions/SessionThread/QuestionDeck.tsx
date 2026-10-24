// The agent's questions as a deck, drawn as the Cockpit draws a call: one card in front with its
// band and its answers as tiles, the rest only peeking out behind it. A single choice moves on as
// it is picked; the last one sends.

import { Fragment, useLayoutEffect, useRef, useState } from "react";
import type { CSSProperties, KeyboardEvent, SyntheticEvent } from "react";
import { createPortal } from "react-dom";
import { Check, ChevronDown, ChevronRight, MessageCircleQuestion } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { useKept } from "../../keep";
import type { AskedQuestion, QuestionAnswer } from "./SessionThread";

const BEHIND = 3;

type Held = { front: number; picked: readonly (readonly string[])[]; using: readonly boolean[]; words: readonly string[] };

const GAP = 8;

/**
 * A preview as a card out of the panel's own box, which clips and scrolls: fixed to the window and level with its row, in the room beside the
 * whole panel so every answer stays readable. It is drawn once unseen to learn its width; then the right of the panel is preferred, then the left,
 * and with room for it on neither it lies over the row's trailing end.
 */
function PreviewCard({ text, row, frame }: { text: string; row: DOMRect; frame: DOMRect }) {
  const card = useRef<HTMLPreElement>(null);
  const [place, setPlace] = useState<CSSProperties>({ visibility: "hidden", insetBlockStart: 0, insetInlineStart: 0 });
  useLayoutEffect(() => {
    const width = card.current?.offsetWidth ?? 0;
    const down = row.top > window.innerHeight / 2 ? { insetBlockEnd: window.innerHeight - row.bottom } : { insetBlockStart: row.top };
    if (window.innerWidth - frame.right - 2 * GAP >= width) setPlace({ ...down, insetInlineStart: frame.right + GAP });
    else if (frame.left - 2 * GAP >= width) setPlace({ ...down, insetInlineEnd: window.innerWidth - frame.left + GAP });
    else setPlace({ ...down, insetInlineEnd: window.innerWidth - row.right + GAP });
  }, [text, row, frame]);
  return createPortal(
    <pre ref={card} className="armada-question-deck__preview" role="region" aria-label="Preview" style={place}>
      {text}
    </pre>,
    document.body,
  );
}

export function QuestionDeck({
  questions,
  onAnswer,
  canSkip,
  flat,
  keep,
}: {
  questions: readonly AskedQuestion[];
  /** A promise holds Answer disabled until it settles, so a second press cannot send it again. */
  onAnswer: (answer?: string, answers?: QuestionAnswer[]) => void | Promise<unknown>;
  canSkip: boolean;
  /** Inside a panel that already frames it: no card, no band, the question and its answers alone. */
  flat?: boolean;
  /** What these questions are about: the picks and the question in front are kept under it, as a message not yet sent is, and go when it is answered or skipped. */
  keep?: string;
}) {
  const [held, setHeld, clearHeld] = useKept<Held>(keep === undefined ? undefined : `question:${keep}`, {
    front: 0,
    picked: questions.map(() => []),
    using: questions.map(() => false),
    words: questions.map(() => ""),
  });
  const { front, picked, using, words } = held;
  // Only what was kept is let go: a deck kept nowhere stays as it is while Fleet takes the answer.
  const letGo = () => (keep === undefined ? undefined : clearHeld());
  const patch = (change: Partial<Held>) => setHeld((was) => ({ ...was, ...change }));
  const setFront = (next: number) => patch({ front: next });
  const setPicked = (next: readonly (readonly string[])[]) => patch({ picked: next });
  const setUsing = (next: readonly boolean[]) => patch({ using: next });
  const setWords = (next: readonly string[]) => patch({ words: next });
  const [open, setOpen] = useState(true);
  // The option the person is pointing at or has focused, on the card it was on, and where its row and the box it sits in are, so the preview can stand beside that box.
  const [looking, setLooking] = useState<{ at: number; label: string; row: DOMRect; frame: DOMRect } | null>(null);
  const [sending, setSending] = useState(false);
  const at = <T,>(list: readonly T[], index: number, value: T): T[] => list.map((one, i) => (i === index ? value : one));
  const chosenFor = (index: number): string[] => {
    const typed = words[index]?.trim() ?? "";
    return [...(picked[index] ?? []), ...(using[index] === true && typed !== "" ? [typed] : [])];
  };
  const last = front === questions.length - 1;
  const complete = questions.every((_, index) => chosenFor(index).length > 0);
  const one = questions[front]!;
  const previewed = looking?.at === front ? one.options.find((option) => option.label === looking.label) : undefined;
  const look = (option: { label: string; preview?: string }, event: SyntheticEvent<HTMLElement>) =>
    option.preview === undefined || option.preview === "" ? undefined : setLooking({ at: front, label: option.label, row: event.currentTarget.getBoundingClientRect(), frame: (event.currentTarget.closest(".armada-needs-panel") ?? event.currentTarget.closest(".armada-question-deck") ?? event.currentTarget).getBoundingClientRect() });
  // Flat, there is no card for the strips to stand behind.
  const behind = flat ? [] : questions.slice(front + 1, front + 1 + BEHIND);

  const choose = (label: string) => {
    const mine = picked[front] ?? [];
    if (one.multi_select) {
      setPicked(at(picked, front, mine.includes(label) ? mine.filter((x) => x !== label) : [...mine, label]));
      return;
    }
    setPicked(at(picked, front, [label]));
    setUsing(at(using, front, false));
    if (!last) setFront(front + 1);
  };
  const other = () => {
    setUsing(at(using, front, one.multi_select ? using[front] !== true : true));
    if (!one.multi_select) setPicked(at(picked, front, []));
  };
  // A number picks the tile it is drawn on, as in the Cockpit.
  const key = (event: KeyboardEvent<HTMLDivElement>) => {
    if (!open || event.target instanceof HTMLInputElement) return;
    const n = Number(event.key);
    if (!Number.isInteger(n) || n < 1) return;
    if (n <= one.options.length) choose(one.options[n - 1]!.label);
    else if (n === one.options.length + 1) other();
    else return;
    event.preventDefault();
  };

  // Flat, in the panel there is no band to hold the pips: a node per question beside the buttons, the answered ones checked and named, the one in front ringed.
  const steps = !flat || questions.length === 1 ? null : (
    <div className="armada-question-deck__steps" role="group" aria-label="Questions">
      {questions.map((q, index) => {
        const name = q.header === "" ? q.question : q.header;
        const done = index !== front && chosenFor(index).length > 0;
        const said = done ? `${name}: ${chosenFor(index).join(", ")}` : name;
        return (
          <Fragment key={q.question}>
            {index === 0 ? null : <span className="armada-question-deck__link" data-done={done || chosenFor(index - 1).length > 0 || undefined} aria-hidden />}
            <Tooltip label={said}>
              <button
                type="button"
                className="armada-question-deck__step"
                aria-label={said}
                aria-current={index === front ? "step" : undefined}
                data-done={done || undefined}
                onClick={() => setFront(index)}
              >
                {done ? <Check size={12} strokeWidth={3} aria-hidden /> : index + 1}
              </button>
            </Tooltip>
          </Fragment>
        );
      })}
    </div>
  );

  return (
    <div className="armada-question-deck" data-flat={flat || undefined}>
      {flat ? null : front === 0 ? null : (
        <div className="armada-question-deck__done" role="group" aria-label="Answered">
          {questions.slice(0, front).map((q, index) => (
            <Button key={q.question} size="sm" variant="ghost" title={q.question} onClick={() => setFront(index)}>
              {q.header === "" ? q.question : q.header}: {chosenFor(index).join(", ")}
            </Button>
          ))}
        </div>
      )}
      <div className="armada-question-deck__stack" style={{ ["--behind" as string]: behind.length }}>
        {behind.map((q, index) => (
          <div key={q.question} className="armada-question-deck__behind" style={{ ["--i" as string]: index }} title={q.question} aria-hidden />
        ))}
        <div key={one.question} className="armada-question-deck__card" role="group" aria-label={one.question} onKeyDown={key}>
          <header className="armada-question-deck__band">
            <button
              type="button"
              className="armada-question-deck__toggle"
              aria-expanded={open}
              title={open ? undefined : one.question}
              onClick={() => setOpen(!open)}
            >
              <MessageCircleQuestion size={16} aria-hidden />
              <span className="armada-question-deck__title">{one.header === "" ? "Question" : one.header}</span>
              {questions.length === 1 ? null : (
                <span className="armada-question-deck__pips" title={`${front + 1} of ${questions.length}`}>
                  {questions.map((q, index) => (
                    <span key={q.question} className="armada-question-deck__pip" data-here={index === front || undefined} data-done={index < front || undefined} />
                  ))}
                </span>
              )}
              {open ? <ChevronDown size={16} aria-hidden /> : <ChevronRight size={16} aria-hidden />}
            </button>
          </header>
          <div className="armada-question-deck__body" hidden={flat ? false : !open}>
            <p className="armada-question-deck__ask">{one.question}</p>
            <div className="armada-question-deck__answers" role={one.multi_select ? "group" : "radiogroup"} aria-label="Options">
              {one.options.map((option, index) => {
                const on = (picked[front] ?? []).includes(option.label);
                return (
                  <button
                    key={option.label}
                    type="button"
                    role={one.multi_select ? "checkbox" : "radio"}
                    aria-checked={on}
                    aria-label={option.label}
                    aria-description={option.description === "" ? undefined : option.description}
                    className="armada-question-deck__answer"
                    data-on={on || undefined}
                    onClick={() => choose(option.label)}
                    onMouseEnter={(event) => look(option, event)}
                    onFocus={(event) => look(option, event)}
                    onMouseLeave={() => setLooking(null)}
                    onBlur={() => setLooking(null)}
                  >
                    <kbd>{index + 1}</kbd>
                    <span className="armada-question-deck__option">
                      <span>{option.label}</span>
                      {option.description === "" ? null : <span className="armada-question-deck__description">{option.description}</span>}
                    </span>
                  </button>
                );
              })}
              <button
                type="button"
                role={one.multi_select ? "checkbox" : "radio"}
                aria-checked={using[front] === true}
                aria-label="Other"
                className="armada-question-deck__answer"
                data-on={using[front] === true || undefined}
                onClick={other}
              >
                <kbd>{one.options.length + 1}</kbd>
                <span className="armada-question-deck__option">
                  <span>Other</span>
                </span>
              </button>
            </div>
            {previewed?.preview === undefined || looking === null ? null : <PreviewCard text={previewed.preview} row={looking.row} frame={looking.frame} />}
            {using[front] === true ? (
              <Input aria-label="Other" autoFocus value={words[front] ?? ""} onChange={(event) => setWords(at(words, front, event.target.value))} />
            ) : null}
            <div className="armada-session-thread__answers" role="group" aria-label="Answers">
              {last ? (
                <Button
                  size="sm"
                  variant="primary"
                  disabled={!complete || sending}
                  onClick={() => {
                    const sent = onAnswer("allow_once", questions.map((q, index) => ({ question: q.question, chosen: chosenFor(index) })));
                    if (sent === undefined) return letGo();
                    setSending(true);
                    void sent.then(letGo, () => undefined).finally(() => setSending(false));
                  }}
                >
                  Answer
                </Button>
              ) : (
                <Button size="sm" variant="secondary" disabled={chosenFor(front).length === 0} onClick={() => setFront(front + 1)}>
                  Next
                </Button>
              )}
              {canSkip ? (
                <Button size="sm" variant="ghost" onClick={() => { letGo(); void onAnswer("refuse"); }}>
                  Skip
                </Button>
              ) : null}
              {steps}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
