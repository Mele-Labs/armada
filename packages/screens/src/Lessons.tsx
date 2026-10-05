// The Lessons page: what got in the way across Jobs, newest first, and one
// Job's retro over it — `docs/concepts/retro.md`.
//
// **The owner reads and answers.** Each item offers Agree and Disagree. Agree
// on an Armada or Manifest item proposes a Job at the approval gate; on a Kit
// item it saves the item under Accepted. Disagree discards it. A row's Job
// label opens that Job's retro.

import { useState } from "react";

import { Alert, LessonList, RetroSheet, Tabs } from "@armada/components";

import { said } from "./copy";
import {
  itemsOf,
  LESSONS_TABS,
  lessonRowsOf,
  notesOf,
  statusOf,
  underTab,
  useAnswers,
  useReadOnFocus,
  type AnswerLesson,
  type LessonsTab,
  type LessonsView,
  type ReadLessons,
  type ReadRetro,
} from "./retro";

/** Open and Accepted, the two lists an item can be on. */
const VIEWS = [
  { id: "open", label: "Open" },
  { id: "accepted", label: "Accepted" },
];

export type LessonsProps = {
  onReadLessons: ReadLessons;
  onReadRetro: ReadRetro;
  /** Agree with one item, and disagree with one. */
  onAgreeLesson: AnswerLesson;
  onDisagreeLesson: AnswerLesson;
  /** Open the Job an agreed item proposed. The shell's own navigation. */
  onOpenJob?: (jobId: string) => void;
  /**
   * The rail's pick, `null` on All. **The listing is read again when it moves**:
   * main narrows the read to the pick, so another pick is another list.
   */
  repository: string | null;
  /** The window is at `--window-floor`. */
  floor: boolean;
  /**
   * Which place the list is narrowed to, and the press that moves it. **The
   * host's**, so it can be remembered for the viewer; absent is All.
   */
  tab?: LessonsTab;
  onTab?: (tab: LessonsTab) => void;
};

export function Lessons({
  onReadLessons,
  onReadRetro,
  onAgreeLesson,
  onDisagreeLesson,
  onOpenJob,
  repository,
  floor,
  tab,
  onTab,
}: LessonsProps) {
  const [held, setHeld] = useState<LessonsTab>("all");
  const [view, setView] = useState<LessonsView>("open");
  const showing = tab ?? held;
  // Keyed by the pick and the list, so another of either is another read.
  const read = useReadOnFocus(() => onReadLessons(view), `${repository ?? ""}:${view}`);
  const [open, setOpen] = useState<{ jobId: string; job: string } | null>(null);
  const answered = useAnswers(onAgreeLesson, onDisagreeLesson, onOpenJob);
  const rows = underTab(read?.ok === true ? lessonRowsOf(read.lessons) : [], showing).flatMap((row) => {
    // A saved item is read and nothing more: the view says it is accepted.
    if (view === "accepted") return [row];
    const shown = answered(row.id, row.landsIn);
    if (shown.gone) return [];
    return [
      {
        ...row,
        ...(shown.answers === undefined ? {} : { answers: shown.answers }),
        ...(shown.settled === undefined ? {} : { settled: shown.settled }),
      },
    ];
  });

  return (
    <div className="armada-screen__overview">
      {read?.ok === false ? (
        <Alert tone="escalated" title="Lessons could not be read">
          {said(read.outcome)}
        </Alert>
      ) : (
        <>
          <div className="armada-lessons__bar">
            {/* Where the fix lands, one place a tab. No count on any of them. */}
            <Tabs
              items={[...LESSONS_TABS]}
              value={showing}
              onChange={(id) => {
                const next = id as LessonsTab;
                setHeld(next);
                onTab?.(next);
              }}
            />
            <Tabs items={VIEWS} value={view} onChange={(id) => setView(id as LessonsView)} />
          </div>
          {/* Before the read answers, and with nothing under the tab, nothing is drawn. */}
          <LessonList
            rows={rows}
            onOpen={(jobId) => {
              const row = rows.find((one) => one.jobId === jobId);
              setOpen({ jobId, job: row?.job ?? jobId });
            }}
          />
        </>
      )}
      {open === null ? null : (
        <JobRetroSheet
          jobId={open.jobId}
          job={open.job}
          read={onReadRetro}
          onAgreeLesson={onAgreeLesson}
          onDisagreeLesson={onDisagreeLesson}
          {...(onOpenJob === undefined ? {} : { onOpenJob })}
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
  onAgreeLesson,
  onDisagreeLesson,
  onOpenJob,
  floor,
  onClose,
}: {
  jobId: string;
  /** The Job as a person reads it, for the sheet's subtitle. */
  job: string;
  read: ReadRetro;
  onAgreeLesson: AnswerLesson;
  onDisagreeLesson: AnswerLesson;
  onOpenJob?: (jobId: string) => void;
  floor: boolean;
  onClose: () => void;
}) {
  const answer = useReadOnFocus(() => read(jobId), jobId);
  const answered = useAnswers(onAgreeLesson, onDisagreeLesson, onOpenJob);
  const retro = answer?.ok === true ? answer.retro : null;
  const status = retro === null ? undefined : statusOf(retro);
  const items = (retro === null ? [] : itemsOf(retro)).flatMap((item) => {
    const shown = answered(item.id ?? "", item.landsIn);
    if (shown.gone) return [];
    return [
      {
        ...item,
        ...(shown.answers === undefined ? {} : { answers: shown.answers }),
        ...(shown.settled === undefined ? {} : { settled: shown.settled }),
      },
    ];
  });
  return (
    <RetroSheet
      open
      job={job}
      reading={answer === undefined}
      {...(answer?.ok === false ? { failure: said(answer.outcome) } : {})}
      {...(status === undefined ? {} : { status })}
      items={items}
      notes={retro === null ? [] : notesOf(retro)}
      floor={floor}
      onClose={onClose}
    />
  );
}
