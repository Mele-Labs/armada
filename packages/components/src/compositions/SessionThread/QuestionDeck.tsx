// The agent's questions as a deck, drawn as the Cockpit draws a call: one card in front with its
// band and its answers as tiles, the rest only peeking out behind it. A single choice moves on as
// it is picked; the last one sends.

import { useState } from "react";
import type { KeyboardEvent } from "react";
import { MessageCircleQuestion } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import type { AskedQuestion, QuestionAnswer } from "./SessionThread";

const BEHIND = 3;

export function QuestionDeck({
  questions,
  onAnswer,
  canSkip,
}: {
  questions: readonly AskedQuestion[];
  onAnswer: (answer?: string, answers?: QuestionAnswer[]) => void;
  canSkip: boolean;
}) {
  const [front, setFront] = useState(0);
  const [picked, setPicked] = useState<readonly (readonly string[])[]>(() => questions.map(() => []));
  const [using, setUsing] = useState<readonly boolean[]>(() => questions.map(() => false));
  const [words, setWords] = useState<readonly string[]>(() => questions.map(() => ""));
  const at = <T,>(list: readonly T[], index: number, value: T): T[] => list.map((one, i) => (i === index ? value : one));
  const chosenFor = (index: number): string[] => {
    const typed = words[index]?.trim() ?? "";
    return [...(picked[index] ?? []), ...(using[index] === true && typed !== "" ? [typed] : [])];
  };
  const last = front === questions.length - 1;
  const complete = questions.every((_, index) => chosenFor(index).length > 0);
  const one = questions[front]!;
  const behind = questions.slice(front + 1, front + 1 + BEHIND);

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
    if (event.target instanceof HTMLInputElement) return;
    const n = Number(event.key);
    if (!Number.isInteger(n) || n < 1) return;
    if (n <= one.options.length) choose(one.options[n - 1]!.label);
    else if (n === one.options.length + 1) other();
    else return;
    event.preventDefault();
  };

  return (
    <div className="armada-question-deck">
      {front === 0 ? null : (
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
            <MessageCircleQuestion size={16} aria-hidden />
            <span>{one.header === "" ? "Question" : one.header}</span>
            {questions.length === 1 ? null : (
              <span className="armada-question-deck__pips" title={`${front + 1} of ${questions.length}`}>
                {questions.map((q, index) => (
                  <span key={q.question} className="armada-question-deck__pip" data-here={index === front || undefined} data-done={index < front || undefined} />
                ))}
              </span>
            )}
          </header>
          <div className="armada-question-deck__body">
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
                    className="armada-question-deck__answer"
                    data-on={on || undefined}
                    onClick={() => choose(option.label)}
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
            {using[front] === true ? (
              <Input aria-label="Other" autoFocus value={words[front] ?? ""} onChange={(event) => setWords(at(words, front, event.target.value))} />
            ) : null}
            <div className="armada-session-thread__answers" role="group" aria-label="Answers">
              {last ? (
                <Button
                  size="sm"
                  variant="secondary"
                  disabled={!complete}
                  onClick={() => onAnswer("allow_once", questions.map((q, index) => ({ question: q.question, chosen: chosenFor(index) })))}
                >
                  Answer
                </Button>
              ) : (
                <Button size="sm" variant="secondary" disabled={chosenFor(front).length === 0} onClick={() => setFront(front + 1)}>
                  Next
                </Button>
              )}
              {canSkip ? (
                <Button size="sm" variant="ghost" onClick={() => onAnswer("refuse")}>
                  Skip
                </Button>
              ) : null}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
