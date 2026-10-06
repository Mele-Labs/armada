// The Retros page: what got in the way across Jobs, newest first, and one
// Job's retro over it — `docs/concepts/retro.md`.
//
// **The owner reads and answers.** Each item offers Agree and Disagree. Agree
// on an Armada or Manifest item proposes a Job at the approval gate; on a Kit
// item it saves the item under Accepted, and where the item carries a command
// it adds that command to Kit's allowed commands and the button reads Update
// Kit. Disagree discards it. A row's Job
// label opens that Job's retro.

import { useState } from "react";

import { Alert, LessonList, RetroSheet, Tabs, type LessonRow } from "@armada/components";

import { said } from "./copy";
import {
  citesOf,
  itemsOf,
  LESSONS_TABS,
  lessonRowsOf,
  notesOf,
  statusOf,
  underTab,
  useAnswers,
  useJobRetros,
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
  const retros = useJobRetros(onReadRetro);
  // The items whose Evidence was pressed, so a read shared by a Job's cards speaks on the one that asked.
  const [pressed, setPressed] = useState<ReadonlySet<string>>(new Set());
  const cited = new Map((read?.ok === true ? read.lessons : []).map((lesson) => [lesson.id, lesson.evidence]));
  const rows = underTab(read?.ok === true ? lessonRowsOf(read.lessons) : [], showing).flatMap((row) => {
    // **The list holds the ids a row cites; its Job's retro holds the rows.** Read once per Job, on the first press.
    const ids = cited.get(row.id) ?? [];
    const got = retros.of(row.jobId);
    let withEvidence: LessonRow = row;
    if (ids.length > 0 && got?.state === "read") {
      withEvidence = { ...row, cites: citesOf(got.retro, ids) };
    } else if (ids.length > 0) {
      withEvidence = {
        ...row,
        evidence: {
          onAsk: () => {
            setPressed((was) => new Set(was).add(row.id));
            retros.ask(row.jobId);
          },
          // Only the card that asked shows the wait or the failure, though its Job's other cards share the read.
          ...(pressed.has(row.id) && got?.state === "pending" ? { pending: true } : {}),
          ...(pressed.has(row.id) && got?.state === "failed" ? { failure: said(got.outcome) } : {}),
        },
      };
    }
    // A saved item is read and nothing more: the view says it is accepted. One that
    // updated Kit says so, with the command Fleet applied.
    if (view === "accepted") {
      const shown = row.applied === undefined ? undefined : answered(row.id, row.landsIn, { state: "accepted", applied: row.applied });
      return [{ ...withEvidence, ...(shown?.settled === undefined ? {} : { settled: shown.settled }) }];
    }
    const shown = answered(row.id, row.landsIn, {
      state: "open",
      ...(row.change === undefined ? {} : { change: row.change }),
    });
    if (shown.gone) return [];
    return [
      {
        ...withEvidence,
        ...(shown.answers === undefined ? {} : { answers: shown.answers }),
        ...(shown.settled === undefined ? {} : { settled: shown.settled }),
      },
    ];
  });

  return (
    <div className="armada-screen__overview">
      {read?.ok === false ? (
        <Alert tone="escalated" title="Retros could not be read">
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
 * Retros page opens it from a row and a Job's Record from its head.
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
    const shown = answered(item.id ?? "", item.landsIn, {
      ...(item.state === undefined ? {} : { state: item.state }),
      ...(item.jobProposed === undefined ? {} : { jobProposed: item.jobProposed }),
      ...(item.change === undefined ? {} : { change: item.change }),
      ...(item.applied === undefined ? {} : { applied: item.applied }),
    });
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
