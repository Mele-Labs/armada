// The Retros page's guided review: the open items one at a time, in the order
// Fleet's model put them, each with the answers the list gives it
// (`docs/concepts/retro.md`, *Reviewing*).
//
// **It answers nothing of its own.** The card's buttons are `useAnswers`', so
// Create Job, Update Kit, Accept and Reject change read and act as they do on
// the list, and a refusal (`fleet.kit_change_refused`) draws the card's alert.
// What this adds is the order, Skip and Back, a thread of questions per item,
// and the items the review set aside.
//
// **It holds nothing of its own that must outlive it.** The order, the place in it, the threads,
// what was put back and the duplicate boxes are a `ReviewHold`'s, per repository pick, so leaving
// Retros and coming back resumes the same card and asks the model nothing.

import { useEffect, useRef, useState, useSyncExternalStore } from "react";

import {
  LessonList,
  LessonReviewEnd,
  LessonReviewStep,
  LessonReviewWait,
  ReviewHeld,
  ReviewSetAside,
  type LessonRow,
} from "@armada/components";
import type { Lesson, RetroSubject } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";
import { said } from "@armada/screens/src/copy";

import {
  citesOf,
  lessonRowsOf,
  retroSubjectOf,
  useAnswers,
  useJobRetros,
  type AnswerLesson,
  type ListedRow,
  type ReadRetro,
  type SessionStarts,
} from "./retro";
import { ageWords, resolved, type QueueEntry } from "./lesson-review";
import type { Thread, ReviewHold } from "./review-hold";
import type { AskLessonRead, AskTurn, LessonReviewRead } from "./review-wire";

/** Ask Fleet for the review of the open items, and put one question about one item. */
export type ReviewLessons = () => Promise<LessonReviewRead>;
export type AskLesson = (lessonId: string, question: string, history: AskTurn[]) => Promise<AskLessonRead>;

export type LessonsReviewProps = {
  /** The page's open items as last read; `undefined` before the first answer. */
  lessons: readonly Lesson[] | undefined;
  onReview: ReviewLessons;
  onAsk: AskLesson;
  /** What the review keeps while the window is open, and the pick it is kept under. */
  hold: ReviewHold;
  holdKey: string;
  onReadRetro: ReadRetro;
  onAgreeLesson: AnswerLesson;
  onDisagreeLesson: AnswerLesson;
  /** Start Session beside Create Job, as the list has it. */
  sessions?: SessionStarts;
  onOpenJob?: (jobId: string) => void;
  /** A press on the Job a card came from. The page opens that retro over itself. */
  onOpenRetro: (subject: RetroSubject, label: string) => void;
  /** Back to the list. */
  onList: () => void;
  /** Something else is over the page, so the arrow keys are not the review's. */
  paused?: boolean;
};

const NONE: Thread = { turns: [], draft: "", pending: false };
/** Re-draw this often so `Reviewed 12 minutes ago` does not stand still while the page is open. */
const AGE_TICK_MS = 30_000;

export function LessonsReview({
  lessons,
  onReview,
  onAsk,
  hold,
  holdKey,
  onReadRetro,
  onAgreeLesson,
  onDisagreeLesson,
  sessions,
  onOpenJob,
  onOpenRetro,
  onList,
  paused = false,
}: LessonsReviewProps) {
  const held = useSyncExternalStore(hold.subscribe, () => hold.get(holdKey));
  const latest = useRef(onReview);
  latest.current = onReview;
  // Asked on entering unless this pick already has a review, held or on its way. The page does not
  // ask again on focus: a model call is not free.
  useEffect(() => {
    hold.begin(holdKey, () => latest.current());
  }, [hold, holdKey]);
  const again = () => hold.again(holdKey, () => latest.current());

  // Where the person is in the remaining queue, the ones put back, the threads and the duplicates to reject.
  const made = held?.made ?? { state: "loading" as const };
  const at = held?.at ?? 0;
  const putBack = held?.putBack ?? [];
  const threads = held?.threads ?? {};
  const also: ReadonlySet<string> = new Set(held?.also ?? []);
  const generation = held?.generation;
  const keep = (change: (was: NonNullable<typeof held>) => NonNullable<typeof held>) => hold.update(holdKey, change, generation);
  const setAt = (next: number) => keep((was) => ({ ...was, at: next }));
  const [twinsFailure, setTwinsFailure] = useState<string | undefined>(undefined);

  // The age is read off the clock when drawn, so it is drawn again now and then.
  const [, setTicks] = useState(0);
  useEffect(() => {
    const tick = window.setInterval(() => setTicks((was) => was + 1), AGE_TICK_MS);
    return () => window.clearInterval(tick);
  }, []);

  const { queue, aside } = made.state === "ready" ? resolved(made.review, lessons ?? []) : { queue: [], aside: [] };
  const back = putBack.flatMap((id) => {
    const one = aside.find((entry) => entry.lesson.id === id);
    return one === undefined
      ? []
      : [{ lesson: one.lesson, reason: `The review set this aside: ${one.why}`, merged: [] } satisfies QueueEntry];
  });
  const entries = [...queue, ...back];
  const away = aside.filter((one) => !putBack.includes(one.lesson.id));

  // Reject on a card that stands for duplicates rejects them too, where the box is ticked and Fleet took the first.
  const alsoRef = useRef(also);
  alsoRef.current = also;
  const twinsRef = useRef(new Map<string, Lesson[]>());
  twinsRef.current = new Map(entries.map((one) => [one.lesson.id, one.merged]));
  const disagree: AnswerLesson = async (id) => {
    const answer = await onDisagreeLesson(id);
    const twins = alsoRef.current.has(id) ? (twinsRef.current.get(id) ?? []) : [];
    if (!answer.ok || twins.length === 0) return answer;
    const refused: string[] = [];
    for (const twin of twins) {
      const one = await onDisagreeLesson(twin.id);
      if (!one.ok) refused.push(refusalWords(one.outcome));
    }
    setTwinsFailure(refused[0]);
    return answer;
  };
  const answered = useAnswers(onAgreeLesson, disagree, onOpenJob, sessions);
  const retros = useJobRetros(onReadRetro);
  const [pressed, setPressed] = useState<ReadonlySet<string>>(new Set());

  /** A card's row with its answers and the Evidence control, as the list draws it. */
  const rows = lessonRowsOf(entries.map((one) => one.lesson)).map((row) => {
    const lesson = entries.find((one) => one.lesson.id === row.id)!.lesson;
    const view = answered(row.id, row.landsIn, { state: "open", ...(row.change === undefined ? {} : { change: row.change }) });
    return { row: withEvidence(row, lesson, retros, pressed, setPressed), view };
  });
  const remaining = entries.flatMap((one, place) => {
    const shown = rows[place]!;
    return shown.view.gone || shown.view.settled !== undefined ? [] : [{ entry: one, shown, place }];
  });
  const index = Math.min(at, remaining.length);
  const current = remaining[index];

  const skip = () => setAt(index + 1);
  const previous = index > 0 ? () => setAt(index - 1) : undefined;
  // Left and Right move; no letter answers anything. A field's own arrows are its own.
  const moves = useRef({ skip, previous });
  moves.current = { skip, previous };
  useEffect(() => {
    if (paused) return;
    const keys = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
      const target = event.target;
      if (target instanceof HTMLElement && (target.isContentEditable || /^(input|textarea|select)$/i.test(target.tagName))) return;
      if (event.key === "ArrowRight") moves.current.skip();
      else if (event.key === "ArrowLeft") moves.current.previous?.();
      else return;
      event.preventDefault();
    };
    window.addEventListener("keydown", keys);
    return () => window.removeEventListener("keydown", keys);
  }, [paused]);

  if (made.state === "loading" || (made.state === "ready" && lessons === undefined)) return <LessonReviewWait />;
  if (made.state === "failed") {
    return <LessonReviewWait failure={refusalWords(made.outcome)} onRetry={again} />;
  }

  // How old the review is and the way to make it again, over whichever of the three endings below is drawn.
  const heldBar = (
    <ReviewHeld age={ageWords(Date.now() - (held?.reviewedAt ?? Date.now()))} onAgain={again} />
  );

  const setAsideList = (
    <ReviewSetAside
      items={away.map((one) => ({
        id: one.lesson.id,
        title: one.lesson.title ?? one.lesson.statement,
        why: one.why,
        onPutBack: () => keep((was) => ({ ...was, putBack: [...was.putBack, one.lesson.id] })),
      }))}
    />
  );

  if (entries.length === 0 && away.length === 0) {
    return (
      <>
        {heldBar}
        <LessonReviewEnd title="Nothing to review" onList={onList} />
      </>
    );
  }
  if (current === undefined) {
    return (
      <>
        {heldBar}
        <LessonReviewEnd
          title={remaining.length === 0 ? "Nothing left to review" : "That was the last one"}
          skipped={remaining.length}
          onAgain={() => setAt(0)}
          {...(remaining.length === 0 ? {} : { onBack: () => setAt(remaining.length - 1) })}
          onList={onList}
        />
        {setAsideList}
      </>
    );
  }

  const id = current.entry.lesson.id;
  const thread = threads[id] ?? NONE;
  const change = (id: string, next: (was: Thread) => Thread) =>
    keep((was) => ({ ...was, threads: { ...was.threads, [id]: next(was.threads[id] ?? NONE) } }));

  async function send() {
    const question = thread.draft.trim();
    if (question === "" || thread.pending) return;
    const history: AskTurn[] = thread.turns.map((turn) => ({ role: turn.role, text: turn.text }));
    change(id, (was) => ({ turns: [...was.turns, { role: "person", text: question }], draft: "", pending: true }));
    const answer = await onAsk(id, question, history);
    change(id, (was) =>
      answer.ok
        ? { ...was, pending: false, turns: [...was.turns, { role: "fleet", text: answer.answer.answer }] }
        : // A refused question is not part of the thread: it goes back to the field to be fixed or sent again.
          { turns: was.turns.slice(0, -1), draft: question, pending: false, failure: refusalWords(answer.outcome) },
    );
  }

  const twins = current.entry.merged;
  return (
    <>
      {heldBar}
      <LessonReviewStep
        // Keyed by the item, so the fold of its duplicates closes on a new card.
        key={id}
        at={current.place + 1}
        of={entries.length}
        reason={current.entry.reason}
        {...(twins.length === 0
          ? {}
          : {
              merged: lessonRowsOf(twins).map((one) => ({ id: one.id, label: one.job, title: one.title ?? one.statement })),
              duplicates: {
                checked: also.has(id),
                onChange: (on: boolean) =>
                  keep((was) => ({ ...was, also: on ? [...new Set([...was.also, id])] : was.also.filter((one) => one !== id) })),
              },
            })}
        {...(twinsFailure === undefined ? {} : { duplicatesFailure: twinsFailure })}
        {...(previous === undefined ? {} : { onBack: previous })}
        onSkip={skip}
        thread={{
          turns: thread.turns,
          draft: thread.draft,
          onDraft: (text) => change(id, (was) => ({ ...was, draft: text })),
          onSend: () => void send(),
          pending: thread.pending,
          ...(thread.failure === undefined ? {} : { failure: thread.failure }),
        }}
      >
        <LessonList
          rows={[
            {
              ...current.shown.row,
              ...(current.shown.view.answers === undefined ? {} : { answers: current.shown.view.answers }),
            },
          ]}
          onOpen={() => onOpenRetro(retroSubjectOf(current.entry.lesson), current.shown.row.job)}
        />
      </LessonReviewStep>
      {setAsideList}
    </>
  );
}

/** The list's Evidence rule for one card: the rows the item cites, read once per Job on the first press. */
function withEvidence(
  row: ListedRow,
  lesson: Lesson,
  retros: ReturnType<typeof useJobRetros>,
  pressed: ReadonlySet<string>,
  setPressed: (next: (was: ReadonlySet<string>) => ReadonlySet<string>) => void,
): LessonRow {
  const ids = lesson.evidence;
  if (ids.length === 0) return row;
  const subject = retroSubjectOf(lesson);
  const got = retros.of(subject);
  if (got?.state === "read") return { ...row, cites: citesOf(got.retro, ids) };
  return {
    ...row,
    evidence: {
      onAsk: () => {
        setPressed((was) => new Set(was).add(row.id));
        retros.ask(subject);
      },
      ...(pressed.has(row.id) && got?.state === "pending" ? { pending: true } : {}),
      ...(pressed.has(row.id) && got?.state === "failed" ? { failure: said(got.outcome) } : {}),
    },
  };
}
