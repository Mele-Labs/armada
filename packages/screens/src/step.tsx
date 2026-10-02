// What the panel says about the step you are looking at: the box a question is
// answered in, and the command a drone is waiting on.
//
// Split out of `JobDetail.tsx` at the 900-line line, for `chapters.tsx`'s
// reason and on the seam beside it: that file assembles a Job's screen and
// holds the open state of one reading, and everything here is a sentence about
// one step, decided from what arrived. Nothing here holds state, and nothing
// here knows which step is selected — it is handed the one that is.
//
// The header's counterpart is `heading.tsx`: what is there changes when the Job
// does, and what is here changes when the selection does.

import { DroneQuestion, Prose } from "@armada/components";
import type { Explaining, JobDetailField } from "@armada/components";
import type { GroupView } from "./draft/group";
import { Fragment, useState } from "react";
import type { ReactNode } from "react";

import type { CommandAnswer, CommandInFlight, JobDetail as JobWhole } from "@armada/protocol";
import type { CommandExplainedRead } from "./calls";
import { offeredOf, said } from "./copy";
import type { ActingAct } from "./pending";
import { shownOf } from "./refused";

/**
 * What the step's groups have got through — `2 of 8 done · 1 working`.
 *
 * **It counts tasks, never Drones** (`#1536`). A step running eight tasks read
 * `implement · running` for two hours and said nothing about which one was
 * moving; a Drone count says the same nothing with a smaller number.
 *
 * Absent where the step holds no group, which is every step but the one the
 * plan is worked at.
 */
export function tasksField(groups: readonly GroupView[]): JobDetailField | undefined {
  const tasks = groups.flatMap((group) => group.tasks);
  if (tasks.length === 0) return undefined;
  const done = tasks.filter((task) => task.state === "done").length;
  const working = tasks.filter((task) => task.state === "working").length;
  const rest = working === 0 ? "" : ` · ${working} working`;
  return { label: "Tasks", value: `${done} of ${tasks.length} done${rest}`, mono: true };
}

/** Why the answers are off, where the reading is not live. */
const STALE_NOTE = "This Job is not live, so nothing can be sent. The drone is still waiting.";

/**
 * What a command's answer still does when it comes late. **Fleet holds a
 * command a little under what the harness waits**, then tells the drone to
 * hold and carries the answer in as its next turn — so an answer is never too
 * late, and a person deciding slowly should not rush for fear it is.
 */
const LATE_ANSWER =
  "If the drone stops waiting before you answer, it is told to hold, and your answer reaches it as its next turn.";

/**
 * How a person's answer to a command the drone was not given is sent, and
 * whether it can be. **One value for both places a person meets one** — the
 * command a drone is waiting on and a refused row on a stopped job — because
 * the call id is what says which, and both are off for the same two reasons.
 */
export type Answering = {
  /**
   * The answer, the words typed with it where there are any, and the rule
   * picked where `always_allow` offered one. **Only a reject reads the words,
   * and only an always-allow reads the rule** — Fleet's rule rather than this
   * screen's.
   */
  send: (call: string, answer: CommandAnswer, note?: string, rule?: string) => void;
  /** What is shown is not live, so nothing may be sent against it. */
  stale: boolean;
  /** An act on this job is already out. */
  acting: boolean;
  /** Which act, where `acting` is true — `"answer_command"` is this box's own. #1117. */
  actingAct?: ActingAct;
};

export function answeringOf(
  jobId: string,
  stale: boolean,
  acting: boolean,
  onAnswerCommand: (
    jobId: string,
    call: string,
    answer: CommandAnswer,
    note?: string,
    rule?: string,
  ) => void,
  actingAct?: ActingAct,
): Answering {
  return {
    send: (call, answer, note, rule) => onAnswerCommand(jobId, call, answer, note, rule),
    stale,
    acting,
    actingAct,
  };
}

/**
 * The command a drone is waiting on a person to allow. `undefined` where
 * nothing waits, which is every job at Stop and wait for me or Run it, and most
 * at Ask me first.
 *
 * **The question's own box**, because it is the same moment — a drone stopped
 * inside a call, a closed set of answers, a person who has to pick one — and a
 * second composition would be two boxes for one kind of wait. The command is
 * what is asked, in mono because it is what the drone sent; each answer is one
 * Fleet offered, in its order, with what it commits to under it.
 *
 * **Aged by the lead above it**, on `questionOf`'s terms, and a command cut by
 * the wire says so on a line of its own, as a refused row does.
 */
export function commandOf(
  whole: JobWhole | null,
  answering: Answering,
  explain?: ExplainOne,
): ReactNode {
  const waiting = whole?.command_waiting;
  if (waiting === undefined) return undefined;
  // Keyed by the call, so a reading of one command never outlives it.
  return <CommandWaiting key={waiting.call} waiting={waiting} answering={answering} explain={explain} />;
}

/** Asking what this one command does. The screen is handed the way to ask. */
type ExplainOne = (call: string) => Promise<CommandExplainedRead>;

/** The field a refusal carries, and what becomes of the words typed in it. */
const REFUSAL_NOTE = "Note (optional)";
const REFUSAL_NOTE_SAYS = "Your words go to the drone with the refusal.";

/** What is said where the reading did not arrive and the refusal named no reason. */
const NO_READING = "Fleet did not explain this command.";

/**
 * The box, and the one thing in this file that holds state: what came back from
 * asking what the command does.
 *
 * **Held here rather than on the published state**, on `useCallArguments`'s
 * terms — one person asks about one call, the answer does not move once it has
 * arrived, and the window does not re-render because somebody read a paragraph.
 */
function CommandWaiting({
  waiting,
  answering,
  explain,
}: {
  waiting: CommandInFlight;
  answering: Answering;
  explain?: ExplainOne;
}): ReactNode {
  const [reading, setReading] = useState<Explaining>({ state: "ready" });
  const offered = offeredOf(waiting.offers, { rules: waiting.rules, suggestedRule: waiting.suggested_rule });
  const cut = shownOf(waiting);

  function ask(): void {
    if (explain === undefined) return;
    setReading({ state: "asking" });
    void explain(waiting.call).then(
      (answer) =>
        setReading(
          answer.ok
            ? {
                state: "read",
                explanation: answer.explained.explanation,
                model: answer.explained.model,
              }
            : { state: "failed", why: said(answer.outcome) || NO_READING },
        ),
      // A rejected call is main gone, which is the window closing. Recorded as
      // an absence so the control is not left reading for the rest of its life.
      () => setReading({ state: "failed", why: NO_READING }),
    );
  }

  return (
    <DroneQuestion
      // **No head and no restatement.** The lead above says a Drone wants to
      // run a command, names it and carries the elapsed; this box is inside
      // that panel now, so all three here would be the same thing twice. What
      // the wire cut is not on the lead, so it stays.
      label={null}
      {...(cut === undefined ? {} : { question: cut.size })}
      options={offered.map(({ offer, label, means, rules, suggestedRule }) => ({
        label,
        consequence: means,
        // Which answer reads words is Fleet's rule, so it is read off the
        // wire's own spelling and never off the words on the control.
        ...(offer === "reject" ? { noteLabel: REFUSAL_NOTE, noteSays: REFUSAL_NOTE_SAYS } : {}),
        ...(rules === undefined ? {} : { rules }),
        ...(suggestedRule === undefined ? {} : { suggestedRule }),
      }))}
      disabled={answering.stale || answering.acting}
      disabledNote={answering.stale ? STALE_NOTE : undefined}
      pending={answering.acting && answering.actingAct === "answer_command"}
      redirectNote={LATE_ANSWER}
      answersLabel="Your answers"
      explain={explain === undefined ? undefined : reading}
      onExplain={ask}
      onAnswer={(label, note, rule) => {
        const chose = offered.find((one) => one.label === label);
        if (chose !== undefined) answering.send(waiting.call, chose.offer, note, rule);
      }}
    />
  );
}

/**
 * What is waiting on a person, in the slot under Overview's lead. A Drone's
 * own question, a command it was not given, the verdict at a review gate, or
 * several at once: a Drone held inside a permission call is rarely asking as
 * well, and when it is, neither box may hide the other.
 *
 * **Every one of them, never the first.** The lead names the one that releases
 * the most and the rest are still open — dropping them here would be the
 * screen deciding on his behalf which of two things he answers.
 */
export function waitingOf(...slots: readonly ReactNode[]): ReactNode {
  const drawn = slots.filter((slot) => slot !== undefined && slot !== null && slot !== false);
  if (drawn.length === 0) return undefined;
  if (drawn.length === 1) return drawn[0];
  return (
    <>
      {drawn.map((slot, at) => (
        <Fragment key={at}>{slot}</Fragment>
      ))}
    </>
  );
}

/**
 * The question itself. `undefined` where nothing is outstanding, which is every
 * drone that knows what it is doing.
 *
 * **The elapsed is `leadOf`'s now**, since the panel around this one draws it.
 *
 * **Stale and in-flight both disable, and each says which.** A window showing a
 * reading it knows is not live must not send an answer against it.
 */
export function questionOf(
  whole: JobWhole | null,
  jobId: string,
  stale: boolean,
  acting: boolean,
  onAnswer: (jobId: string, questionId: string, chose: string) => void,
  actingAct?: ActingAct,
): ReactNode {
  const asking = whole?.asking;
  if (asking === undefined) return undefined;
  return (
    <DroneQuestion
      // The lead says the Drone asked, quotes it and ages it — `commandOf`'s
      // reason for drawing neither head nor question here.
      label={null}
      // What each answer commits to is the Drone's own writing, so markdown.
      options={asking.options.map((option) => ({ ...option, consequence: <Prose text={option.consequence} /> }))}
      disabled={stale || acting}
      disabledNote={stale ? STALE_NOTE : undefined}
      pending={acting && actingAct === "answer"}
      onAnswer={(label) => onAnswer(jobId, asking.question_id, label)}
    />
  );
}
