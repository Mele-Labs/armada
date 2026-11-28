// The cards a Job's thread ends in that are not artifacts: a question, a command to run, a press, and a page only the Mac
// shows. The thread draws them from its `waiting_for`, and To review from each item's Needs you row, so each takes the
// words and the answer and not either source.

import { useState } from "react";
import { AppWindow, Check, LoaderCircle } from "lucide-react";
import { Button } from "./kit/components/primitives/Button/Button";
import { Card, CardFooter, CardHeader, CardTitle } from "./kit/components/primitives/Card/Card";
import { Textarea } from "./kit/components/primitives/Textarea/Textarea";
import { Tooltip } from "./kit/components/primitives/Tooltip/Tooltip";

import { OPEN_ON_MAC } from "./thread";
import { Figure, Trouble } from "./ui";

/** An option to press, with its index on the wire. */
export type Choice = {
  label: string;
  description?: string;
  choice: number;
};

/** Something the owner is asked: its choices, and their own words where Fleet takes words. */
export function WaitingAsk({
  text,
  choices,
  words: takesWords,
  keep,
  making = false,
  refused = "",
  headed = false,
  answer,
}: {
  text: string;
  /** A head over it already says the text. */
  headed?: boolean;
  choices: readonly Choice[];
  /** Fleet takes words besides the choices. */
  words: boolean;
  keep: string;
  /** A slot is being made for it, so it asks for nothing yet. */
  making?: boolean;
  /** Why making the slot was refused. */
  refused?: string;
  answer: (given: { choice?: number; text?: string }) => Promise<boolean>;
}) {
  const [words, setWords] = useState("");
  return (
    <div className="pk-ask" role="group" aria-label={text}>
      <p className="pk-question" hidden={headed && !making}>
        {headed ? null : text}
        {!making ? null : (
          <>
            {" "}
            <Tooltip label="Making a slot">
              <span role="img" aria-label="Making a slot">
                <LoaderCircle className="pk-spin" size={14} aria-hidden="true" />
              </span>
            </Tooltip>
          </>
        )}
      </p>
      <Trouble says={refused} />
      {choices.length === 0 ? null : (
        <div className="pk-choices" role="group" aria-label="Options">
          {choices.map(({ label, description, choice }) => (
            <Button
              key={label}
              variant="secondary"
              onClick={() =>
                void answer({
                  choice,
                })
              }
            >
              <span className="pk-choice__words">
                {label}
                {description === undefined ? null : (
                  <span className="pk-choice__says">{description}</span>
                )}
              </span>
            </Button>
          ))}
        </div>
      )}
      {takesWords ? (
        <>
          <Textarea
            keep={keep}
            rows={2}
            aria-label="Answer"
            placeholder="Answer"
            value={words}
            onChange={(event) => setWords(event.target.value)}
          />
          <Button
            variant="primary"
            disabled={words.trim() === ""}
            onClick={() =>
              void answer({
                text: words.trim(),
              }).then((went) => went && setWords(""))
            }
          >
            Answer
          </Button>
        </>
      ) : null}
    </div>
  );
}

/**
 * A command the agent asked the owner to run on the Mac: run there from here, and its exit and tail come back as a `ran` row.
 * Once a run has ended the card says so, with its exit code, until the agent clears the item.
 */
export function RunCard({
  text,
  command,
  ran,
  run,
}: {
  text: string;
  command: string;
  ran?: {
    code?: number;
  };
  run: () => Promise<boolean>;
}) {
  const [going, setGoing] = useState(false);
  return (
    <Card className="pk-card" role="group" aria-label={text}>
      <CardHeader>
        <CardTitle>{text}</CardTitle>
      </CardHeader>
      <code className="pk-card__command">{command}</code>
      <CardFooter className="pk-card__acts">
        {ran !== undefined ? (
          <span className="pk-card__ran">
            {ran.code === undefined ? null : (
              <Figure name="Exit code" loose>{`exit ${ran.code}`}</Figure>
            )}
            <Tooltip label="Ran">
              <span role="img" aria-label="Ran">
                <Check size={16} strokeWidth={2} aria-hidden="true" />
              </span>
            </Tooltip>
          </span>
        ) : (
          <Button
            variant="primary"
            pending={going}
            onClick={() => {
              setGoing(true);
              void run().finally(() => setGoing(false));
            }}
          >
            Run
          </Button>
        )}
      </CardFooter>
    </Card>
  );
}

/** Something Fleet lists that is pressed and not answered: a Job to open, a call to dismiss. */
export function PressCard({
  text,
  open,
  dismiss,
}: {
  text: string;
  open?: () => void;
  dismiss?: () => Promise<boolean>;
}) {
  return (
    <div className="pk-ask" role="group" aria-label={text}>
      <p className="pk-question">{text}</p>
      <div className="pk-choices" role="group" aria-label="Options">
        {open === undefined ? null : (
          <Button variant="secondary" onClick={open}>
            Open
          </Button>
        )}
        {dismiss === undefined ? null : (
          <Button variant="secondary" onClick={() => void dismiss()}>
            Dismiss
          </Button>
        )}
      </div>
    </div>
  );
}

/** A walk, or a question asked in one, points at a page that is only on the Mac: the Job says so, and takes no answer here. */
export function OnMac({ text }: { text: string }) {
  return (
    <p className="pk-onmac" role="group" aria-label={text}>
      <AppWindow size={14} aria-hidden="true" />
      <span className="pk-onmac__says">{text}</span>
      <span className="pk-onmac__where">{OPEN_ON_MAC}</span>
    </p>
  );
}
