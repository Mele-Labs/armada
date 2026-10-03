// The Lessons page: what got in the way across Jobs, newest first, and one
// Job's retro over it — `docs/concepts/retro.md`.
//
// **The owner reads and decides; nothing here acts.** No item is filed,
// proposed or put into a brief, and the page offers nothing that would. A row
// opens its Job's retro, and the retro's one control is its close.

import { useState } from "react";

import { Alert, LessonList, RetroSheet } from "@armada/components";

import { said } from "./copy";
import {
  itemsOf,
  lessonRowsOf,
  notesOf,
  statusOf,
  useReadOnFocus,
  type ReadLessons,
  type ReadRetro,
} from "./retro";

export type LessonsProps = {
  onReadLessons: ReadLessons;
  onReadRetro: ReadRetro;
  /**
   * The rail's pick, `null` on All. **The listing is read again when it moves**:
   * main narrows the read to the pick, so another pick is another list.
   */
  repository: string | null;
  /** The window is at `--window-floor`. */
  floor: boolean;
};

export function Lessons({ onReadLessons, onReadRetro, repository, floor }: LessonsProps) {
  const read = useReadOnFocus(onReadLessons, repository ?? "");
  const [open, setOpen] = useState<{ jobId: string; job: string } | null>(null);
  const rows = read?.ok === true ? lessonRowsOf(read.lessons) : [];

  return (
    <div className="armada-screen__overview">
      {read?.ok === false ? (
        <Alert tone="escalated" title="Lessons could not be read">
          {said(read.outcome)}
        </Alert>
      ) : (
        // Before the read answers, and with nothing written, nothing is drawn.
        <LessonList
          rows={rows}
          openJob={open?.jobId ?? null}
          onOpen={(jobId) => {
            const row = rows.find((one) => one.jobId === jobId);
            setOpen({ jobId, job: row?.job ?? jobId });
          }}
        />
      )}
      {open === null ? null : (
        <JobRetroSheet
          jobId={open.jobId}
          job={open.job}
          read={onReadRetro}
          floor={floor}
          onClose={() => setOpen(null)}
        />
      )}
    </div>
  );
}

/**
 * One Job's retro on its sheet, read when it opens and again on focus. The
 * Lessons page opens it from a row and a Job's Record from its head.
 */
export function JobRetroSheet({
  jobId,
  job,
  read,
  floor,
  onClose,
}: {
  jobId: string;
  /** The Job as a person reads it, for the sheet's subtitle. */
  job: string;
  read: ReadRetro;
  floor: boolean;
  onClose: () => void;
}) {
  const answer = useReadOnFocus(() => read(jobId), jobId);
  const retro = answer?.ok === true ? answer.retro : null;
  const status = retro === null ? undefined : statusOf(retro);
  return (
    <RetroSheet
      open
      job={job}
      reading={answer === undefined}
      {...(answer?.ok === false ? { failure: said(answer.outcome) } : {})}
      {...(status === undefined ? {} : { status })}
      items={retro === null ? [] : itemsOf(retro)}
      notes={retro === null ? [] : notesOf(retro)}
      floor={floor}
      onClose={onClose}
    />
  );
}
