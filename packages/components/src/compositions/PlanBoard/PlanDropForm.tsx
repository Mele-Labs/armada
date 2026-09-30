import { useState } from "react";
import { Button, type ButtonAnswer } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { patternFor, useHaptics } from "../../haptics";

/**
 * Dropping a task. **The reason is asked for in place**, under the acts: a
 * drop is a short act on the task being read, and a dialog over the panel
 * would hide the very task the reason is about.
 */
export type PlanTaskDrop = {
  /**
   * Sends the drop. Resolves to `null` where it was taken, or to what the
   * refusal says where it was not — and the reason typed stays.
   */
  onDrop: (reason: string) => Promise<string | null>;
  /** Nothing is live to send it over. */
  disabled?: boolean;
};

/**
 * The reason, asked in place once a drop is pressed — Drop this task in a
 * task's panel, and Remove on a group, which drops each of its tasks with the
 * one reason (owner, 30 Sep 2026). **One form for both**, so the two read as
 * the same act. The words are the caller's.
 */
export function PlanDropForm({
  drop,
  label,
  send,
  sending,
  onClose,
}: {
  drop: PlanTaskDrop;
  /** The region's name — the act that opened it. */
  label: string;
  /** The send button, and what it says while the drop is out. */
  send: string;
  sending: string;
  onClose: () => void;
}) {
  const [reason, setReason] = useState("");
  const [dropping, setDropping] = useState(false);
  const [refused, setRefused] = useState<string | null>(null);
  // Only a refusal is drawn on the control: a drop that is taken closes the form.
  const [answer, setAnswer] = useState<ButtonAnswer>();
  // **A blank field is a hint until a drop is tried with it**, and an error
  // from then on: a red field nobody has typed in reads as a mistake already made.
  const [triedBlank, setTriedBlank] = useState(false);
  const tap = useHaptics();
  const blank = reason.trim() === "";

  async function submit(): Promise<void> {
    if (blank) {
      setTriedBlank(true);
      return;
    }
    setDropping(true);
    setRefused(null);
    setAnswer(undefined);
    try {
      const said = await drop.onDrop(reason.trim());
      // The tap answers the press: a drop that is taken closes the form, with
      // no control left to draw the answer on. #1326.
      tap(patternFor(said === null ? "accepted" : "refused"));
      if (said === null) {
        onClose();
        return;
      }
      setRefused(said);
      setAnswer("refused");
    } finally {
      setDropping(false);
    }
  }

  return (
    <section className="armada-task-sheet__drop" aria-label={label}>
      <Input
        label="Reason"
        value={reason}
        invalid={triedBlank && blank}
        disabled={dropping}
        onChange={(event) => setReason(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") void submit();
        }}
      />
      {blank ? (
        <span className="armada-task-sheet__hint" data-tone={triedBlank ? "error" : "muted"}>
          A reason is needed.
        </span>
      ) : null}
      {refused === null ? null : <span className="armada-task-sheet__refused">{refused}</span>}
      <div className="armada-task-sheet__drop-acts">
        <Button variant="secondary" size="sm" ground="sunken" disabled={dropping} onClick={onClose}>
          Cancel
        </Button>
        <Button
          variant="secondary"
          size="sm"
          ground="sunken"
          pending={dropping}
          answer={answer}
          disabled={blank || drop.disabled}
          onClick={() => void submit()}
        >
          {dropping ? sending : send}
        </Button>
      </div>
    </section>
  );
}
