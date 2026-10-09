// The agent's questions as a deck: one in front, the rest standing behind it as strips, the way the
// Cockpit stacks its calls. A single choice moves on as it is picked; the last one sends.

import { useState } from "react";

import { Button } from "../../primitives/Button/Button";
import { Checkbox } from "../../primitives/Checkbox/Checkbox";
import { Input } from "../../primitives/Input/Input";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
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
  const chosenFor = (index: number, from = picked): string[] => {
    const typed = words[index]?.trim() ?? "";
    return [...(from[index] ?? []), ...(using[index] === true && typed !== "" ? [typed] : [])];
  };
  const last = front === questions.length - 1;
  const complete = questions.every((_, index) => chosenFor(index).length > 0);
  const one = questions[front]!;
  const behind = questions.slice(front + 1, front + 1 + BEHIND);
  const send = (from = picked) =>
    onAnswer(
      "allow_once",
      questions.map((q, index) => ({ question: q.question, chosen: chosenFor(index, from) })),
    );

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
          <div
            key={q.question}
            className="armada-question-deck__behind"
            style={{ ["--i" as string]: index }}
            aria-hidden
          >
            {q.question}
          </div>
        ))}
        <div key={one.question} className="armada-question-deck__card" role="group" aria-label={one.question}>
          <div className="armada-session-questions__text">
            {one.header === "" ? null : <span className="armada-session-questions__header">{one.header}</span>}
            {one.question}
          </div>
          <RadioGroup>
            {one.options.map((option) => {
              const on = (picked[front] ?? []).includes(option.label);
              const control = one.multi_select ? (
                <Checkbox
                  checked={on}
                  onChange={() =>
                    setPicked(
                      at(picked, front, on ? (picked[front] ?? []).filter((x) => x !== option.label) : [...(picked[front] ?? []), option.label]),
                    )
                  }
                >
                  {option.label}
                </Checkbox>
              ) : (
                <Radio
                  name={`question-${front}`}
                  checked={on}
                  onChange={() => {
                    const next = at(picked, front, [option.label]);
                    setPicked(next);
                    setUsing(at(using, front, false));
                    if (!last) setFront(front + 1);
                  }}
                >
                  {option.label}
                </Radio>
              );
              return option.description === "" ? (
                <div key={option.label}>{control}</div>
              ) : (
                <Tooltip key={option.label} label={option.description}>
                  <div>{control}</div>
                </Tooltip>
              );
            })}
            {one.multi_select ? (
              <Checkbox checked={using[front] === true} onChange={() => setUsing(at(using, front, using[front] !== true))}>
                Other
              </Checkbox>
            ) : (
              <Radio
                name={`question-${front}`}
                checked={using[front] === true}
                onChange={() => {
                  setUsing(at(using, front, true));
                  setPicked(at(picked, front, []));
                }}
              >
                Other
              </Radio>
            )}
          </RadioGroup>
          {using[front] === true ? (
            <Input aria-label="Other" value={words[front] ?? ""} onChange={(event) => setWords(at(words, front, event.target.value))} />
          ) : null}
          <div className="armada-session-thread__answers" role="group" aria-label="Answers">
            {last ? (
              <Button size="sm" variant="secondary" disabled={!complete} onClick={() => send()}>
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
  );
}
