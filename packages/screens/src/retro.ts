// What a Job's retro and the Lessons listing are drawn from — `docs/concepts/retro.md`.
//
// **Nothing here acts and nothing proposes.** A retro is read, and the owner
// decides what each item is worth; every function below turns the wire into
// what the sheet and the list draw, and nothing more.
//
// **Read on open and on focus, never on a timer.** Nothing on `/events` says a
// retro was written, and a retro is written once, after its Job ends — so a
// surface asks when it opens and again when the window comes back to the
// front, which is when somebody could have missed one being written.

import { useEffect, useRef, useState } from "react";

import type {
  JobRetro,
  Lesson,
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
import type { LessonRow, RetroCite, RetroNote, RetroSheetItem } from "@armada/components";

import { absoluteOf, lasting } from "./duration";

/** Ask main for one Job's retro. */
export type ReadRetro = (jobId: string) => Promise<RetroRead>;
/** Ask main for the Lessons listing, narrowed to this window's pick. */
export type ReadLessons = () => Promise<LessonsRead>;

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
    who: item.who,
    statement: item.statement,
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
  const seen = new Map<string, number>();
  return lessons.map((lesson) => {
    const at = seen.get(lesson.job_id) ?? 0;
    seen.set(lesson.job_id, at + 1);
    return {
      id: `${lesson.job_id}:${at}`,
      jobId: lesson.job_id,
      who: lesson.who,
      statement: lesson.statement,
      job: jobOf(lesson.handle),
      jobExact: lesson.handle,
      when: absoluteOf(lesson.at) ?? lesson.at,
      whenExact: lesson.at,
    };
  });
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
