// What a Job's retro and the Lessons listing are drawn from — `docs/concepts/retro.md`.
//
// A retro is read, and the owner decides what each item is worth: Agree or
// Disagree. Every function below turns the wire into what the sheet and the
// list draw, and `useAnswers` holds what an answer leaves on screen.
//
// **Read on open and on focus, never on a timer.** Nothing on `/events` says a
// retro was written, and a retro is written once, after its Job ends — so a
// surface asks when it opens and again when the window comes back to the
// front, which is when somebody could have missed one being written.

import { useEffect, useRef, useState } from "react";

import type {
  JobRetro,
  LandsIn,
  Lesson,
  LessonAnswer,
  LessonsRead,
  RecordAct,
  RecordAsked,
  RecordCheck,
  RecordNotMet,
  RecordRefusal,
  RecordSaid,
  RecordWaited,
  RetroRead,
  RetroRecord,
} from "@armada/protocol";
import type { LessonAnswers, LessonRow, LessonSettled, RetroCite, RetroNote, RetroSheetItem } from "@armada/components";

import { said as refusalSaid } from "./copy";
import { absoluteOf, lasting } from "./duration";

/** Ask main for one Job's retro. */
export type ReadRetro = (jobId: string) => Promise<RetroRead>;
/** Ask main for the Lessons listing, narrowed to this window's pick: the open items or the saved ones. */
export type ReadLessons = (state: LessonsView) => Promise<LessonsRead>;
/** Which list the page draws: items waiting on the owner, or the Kit items he saved. */
export type LessonsView = "open" | "accepted";
/** One answer on one item, by its id. */
export type AnswerLesson = (lessonId: string) => Promise<LessonAnswer>;

/** A record row as one line: what it is about, what it came to, and when. */
function when(at: string | undefined): { when?: string } {
  const drawn = at === undefined ? null : absoluteOf(at);
  return drawn === null ? {} : { when: drawn };
}

function detail(...parts: (string | undefined)[]): { detail?: string } {
  const said = parts.filter((one): one is string => one !== undefined && one !== "").join(" · ");
  return said === "" ? {} : { detail: said };
}

const refusal = (row: RecordRefusal): RetroCite => ({
  id: row.cite,
  name: row.tool,
  mono: true,
  ...detail(row.tried, row.because),
  ...when(row.at),
});

const check = (row: RecordCheck): RetroCite => ({
  id: row.cite,
  name: row.name,
  mono: true,
  ...detail(row.produced ?? row.expected),
  ...when(row.at),
});

const notMet = (row: RecordNotMet): RetroCite => ({
  id: row.cite,
  name: row.criterion,
  mono: true,
  ...detail(row.produced ?? row.expected),
});

/** Something a Drone said, under its step where it names one. */
const said = (row: RecordSaid): RetroCite =>
  row.step === undefined
    ? { id: row.cite, name: row.said, ...when(row.at) }
    : { id: row.cite, name: row.step, mono: true, ...detail(row.said), ...when(row.at) };

/** A move, and who made it through which door — `via` is what tells a press from an agent's `curl`. */
const act = (row: RecordAct): RetroCite => ({
  id: row.cite,
  name: row.moved,
  mono: true,
  ...detail(row.via === undefined ? row.actor : `${row.actor} via ${row.via}`, row.said),
  ...when(row.at),
});

const asked = (row: RecordAsked): RetroCite => ({
  id: row.cite,
  name: row.about,
  ...detail(row.answer),
  ...when(row.asked_at),
});

const waited = (row: RecordWaited): RetroCite => ({
  id: row.cite,
  name: row.status,
  mono: true,
  ...detail(row.ms === undefined ? undefined : lasting(row.ms)),
  ...when(row.from),
});

/** Every row of the record, by its `cite`. */
function rowsOf(record: RetroRecord): Map<string, RetroCite> {
  const rows: RetroCite[] = [
    ...(record.refusals ?? []).map(refusal),
    ...(record.failed_checks ?? []).map(check),
    ...(record.not_met ?? []).map(notMet),
    ...(record.not_done ?? []).map(said),
    ...(record.said_after ?? []).map(said),
    ...(record.restarts ?? []).map(act),
    ...(record.asked ?? []).map(asked),
    ...(record.waited ?? []).map(waited),
    ...(record.acts ?? []).map(act),
    ...(record.notes ?? []).map(said),
  ];
  return new Map(rows.map((row) => [row.id, row]));
}

/**
 * What got in the way, in the order it was written, each with the record rows
 * it cites. **A cite the record does not hold is left out** rather than drawn
 * as a name nobody can read back — Fleet already drops those, and this holds
 * to it for a record read later than its retro was written.
 */
export function itemsOf(retro: JobRetro): RetroSheetItem[] {
  const rows = rowsOf(retro.record);
  return (retro.items ?? []).map((item) => ({
    id: item.id,
    who: item.who,
    ...(item.lands_in === undefined ? {} : { landsIn: item.lands_in }),
    statement: item.statement,
    ...(item.title === undefined ? {} : { title: item.title }),
    ...(item.what === undefined ? {} : { what: item.what }),
    ...(item.fix === undefined ? {} : { fix: item.fix }),
    cites: item.evidence.flatMap((cite) => {
      const row = rows.get(cite);
      return row === undefined ? [] : [row];
    }),
  }));
}

/** Where a retro stands, where it is not written. Nothing for a written one: its items say it. */
export function statusOf(retro: JobRetro): string | undefined {
  const because = retro.why === undefined || retro.why === "" ? "" : `: ${retro.why}`;
  switch (retro.state) {
    case "written":
      return undefined;
    case "pending":
      return "Not written yet";
    case "failed":
      return `Not written${because}`;
    case "skipped":
      return `Skipped${because}`;
  }
}

/** The owner's notes left with this Job's detail open, as Fleet linked them. */
export function notesOf(retro: JobRetro): RetroNote[] {
  return (retro.annotations ?? []).map((note) => ({ id: note.id, text: note.text, ...when(note.at) }));
}

/** `Job 3` off `3-retire-two-guides` — `jobNumber`'s rule, for a lesson that carries no summary. */
export function jobOf(handle: string): string {
  const number = /^(\d+)-/.exec(handle)?.[1];
  return number === undefined ? handle : `Job ${number}`;
}

/** The Lessons list's rows, in Fleet's order: newest retro first. */
export function lessonRowsOf(lessons: readonly Lesson[]): LessonRow[] {
  return lessons.map((lesson) => {
    return {
      id: lesson.id,
      jobId: lesson.job_id,
      who: lesson.who,
      ...(lesson.lands_in === undefined ? {} : { landsIn: lesson.lands_in }),
      statement: lesson.statement,
      ...(lesson.title === undefined ? {} : { title: lesson.title }),
      ...(lesson.what === undefined ? {} : { what: lesson.what }),
      ...(lesson.fix === undefined ? {} : { fix: lesson.fix }),
      job: jobOf(lesson.handle),
      jobExact: lesson.handle,
      when: absoluteOf(lesson.at) ?? lesson.at,
      whenExact: lesson.at,
    };
  });
}

/** The Lessons page's filter: every item, or those whose fix lands in one place. */
export type LessonsTab = "all" | LandsIn;

/** The tabs, in the order the owner named them (3 Oct 2026). No counts: hard rule 7. */
export const LESSONS_TABS: readonly { id: LessonsTab; label: string }[] = [
  { id: "all", label: "All" },
  { id: "armada", label: "Armada" },
  { id: "kit", label: "Kit" },
  { id: "manifest", label: "Manifest" },
];

/** A remembered tab read back. Anything not a tab — nothing stored, an old value — is All. */
export function lessonsTabNamed(value: string | null): LessonsTab {
  return LESSONS_TABS.find((one) => one.id === value)?.id ?? "all";
}

/**
 * The rows under one tab, still newest first. **Filtered here rather than
 * with `?lands_in=`**: the page holds one read, so a tab press asks Fleet for
 * nothing. A row stored before `lands_in` was written is under All alone.
 */
export function underTab(rows: readonly LessonRow[], tab: LessonsTab): LessonRow[] {
  return tab === "all" ? [...rows] : rows.filter((row) => row.landsIn === tab);
}

/** What Agree does for an item, by where its fix lands, as its tooltip says it. */
export function agreeTipOf(landsIn: LandsIn | undefined): string {
  switch (landsIn) {
    case "armada":
      return "Proposes a Job on Armada's repository";
    case "manifest":
      return "Proposes a Job on the Manifest's repository";
    case "kit":
      return "Saves it under Accepted";
    case undefined:
      return "Agrees with it";
  }
}

/** What Disagree does, the same for every place. */
export const DISAGREE_TIP = "Discards it";

/** The words an answered item reads as while it stays on screen. */
const AGREED = "Agreed";
const PROPOSED_JOB = "Proposed Job";

/** What an item shows of its own answer: the buttons, what it settled as, or that it has gone. */
export type AnswerView = { answers?: LessonAnswers; settled?: LessonSettled; gone: boolean };

/**
 * What answering leaves on screen, for the list and the sheet alike.
 *
 * **An answer sends once per item** and the item's buttons hold while it is out.
 * Agreed for an Armada or Manifest item stays, reading `Agreed` with a link to
 * the Job it proposed, until the surface is read again, when Fleet no longer
 * lists it as open. Agreeing a Kit item and disagreeing with any item take it
 * off at once. A refusal stays on the item, with both answers.
 */
export function useAnswers(
  agree: AnswerLesson | undefined,
  disagree: AnswerLesson | undefined,
  openJob: ((jobId: string) => void) | undefined,
): (lessonId: string, landsIn: LandsIn | undefined) => AnswerView {
  const [pressing, setPressing] = useState<Record<string, "agree" | "disagree">>({});
  const [refused, setRefused] = useState<Record<string, string>>({});
  const [gone, setGone] = useState<ReadonlySet<string>>(new Set());
  const [agreed, setAgreed] = useState<Record<string, string | undefined>>({});

  async function press(lessonId: string, which: "agree" | "disagree", landsIn: LandsIn | undefined) {
    const send = which === "agree" ? agree : disagree;
    if (send === undefined) return;
    setPressing((was) => ({ ...was, [lessonId]: which }));
    setRefused(({ [lessonId]: _, ...rest }) => rest);
    const answer = await send(lessonId);
    setPressing(({ [lessonId]: _, ...rest }) => rest);
    if (!answer.ok) {
      setRefused((was) => ({ ...was, [lessonId]: refusalSaid(answer.outcome) }));
      return;
    }
    if (which === "agree" && landsIn !== "kit" && answer.lesson.state === "agreed") {
      setAgreed((was) => ({ ...was, [lessonId]: answer.lesson.job_proposed }));
    } else {
      setGone((was) => new Set(was).add(lessonId));
    }
  }

  return (lessonId, landsIn) => {
    if (gone.has(lessonId)) return { gone: true };
    if (lessonId in agreed) {
      const job = agreed[lessonId];
      return {
        gone: false,
        settled: {
          said: AGREED,
          ...(job === undefined || openJob === undefined
            ? {}
            : { job: { label: PROPOSED_JOB, onOpen: () => openJob(job) } }),
        },
      };
    }
    if (agree === undefined || disagree === undefined) return { gone: false };
    const held = pressing[lessonId];
    const why = refused[lessonId];
    return {
      gone: false,
      answers: {
        agreeTip: agreeTipOf(landsIn),
        disagreeTip: DISAGREE_TIP,
        onAgree: () => void press(lessonId, "agree", landsIn),
        onDisagree: () => void press(lessonId, "disagree", landsIn),
        ...(held === undefined ? {} : { pressing: held }),
        ...(why === undefined ? {} : { refusal: why }),
      },
    };
  };
}

/**
 * A read asked when the surface mounts and again whenever the window comes
 * back to the front. **`undefined` until the first answer.** A later answer
 * replaces the earlier one without blanking it, so a focus does not flash the
 * surface back to its skeleton.
 */
export function useReadOnFocus<T>(read: () => Promise<T>, key: string): T | undefined {
  const [held, setHeld] = useState<{ key: string; read: T } | undefined>(undefined);
  // **Read through a ref**, so a caller's lambda rebuilt every render does not
  // re-run the effect — the answer sets state, and the loop would feed itself.
  const latest = useRef(read);
  latest.current = read;
  useEffect(() => {
    let live = true;
    const again = () =>
      void latest.current().then((answer) => {
        // An answer for a Job or a pick this surface has left is not this one's.
        if (live) setHeld({ key, read: answer });
      });
    again();
    window.addEventListener("focus", again);
    return () => {
      live = false;
      window.removeEventListener("focus", again);
    };
  }, [key]);
  return held?.key === key ? held.read : undefined;
}
