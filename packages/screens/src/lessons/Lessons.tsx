// The Retros page: what got in the way across Sessions, newest first, and one
// Session's retro over it — `docs/concepts/retro.md`.
//
// **The owner reads and answers.** An Armada or Manifest item is agreed with by
// starting a Session on it; a Kit item is saved under Accepted, and where it
// carries a command it adds that command to Kit's allowed commands and the
// button reads Update Kit. Disagree discards it. A row's Session label opens
// that Session's retro.

import { useRef, useState, useSyncExternalStore } from "react";

import { Alert, Button, LessonList, LessonSplit, RetroSheet, Tabs, type LessonRow } from "@armada/components";

import { said } from "../copy";
import {
  citesOf,
  itemsOf,
  LESSONS_TABS,
  lessonRowsOf,
  retroKeyOf,
  retroSubjectOf,
  statusOf,
  underTab,
  useAnswers,
  useRetros,
  useReadOnFocus,
  type AnswerLesson,
  type LessonsTab,
  type LessonsView,
  type ReadLessons,
  type ReadRetro,
  type SessionStarts,
  type StartLessonSession,
} from "./retro";
import type { RetroSubject } from "@armada/protocol";
import { LessonsReview, type AskLesson, type ReviewLessons } from "./LessonsReview";
import { createReviewHold, type ReviewHold } from "./review-hold";

/** Open and Accepted, the two lists an item can be on. */
const VIEWS = [
  { id: "open", label: "Open" },
  { id: "accepted", label: "Accepted" },
];

export type LessonsProps = {
  onReadLessons: ReadLessons;
  onReadRetro: ReadRetro;
  /** Agree with one Kit item, and disagree with any item. */
  onAgreeLesson: AnswerLesson;
  onDisagreeLesson: AnswerLesson;
  /**
   * Start Session on an Armada or Manifest item: the call to Fleet, and the
   * host's way of opening the Session it started with the first message in its message box.
   * **Both or neither**; absent, and no card offers Start Session.
   */
  onStartSession?: StartLessonSession;
  onOpenSession?: (sessionId: string) => void;
  /**
   * The guided review, `Review` beside the tabs: Fleet's model orders the open items, and a
   * question about one is put to it. Absent, and the page offers no Review.
   */
  onReviewLessons?: ReviewLessons;
  onAskLesson?: AskLesson;
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
  /**
   * What the review keeps while the window is open, so leaving the page and coming back resumes at
   * the same card and asks the model nothing (`review-hold.ts`). **The host's**, made once for the
   * window; absent, and the page keeps its own for as long as it is mounted.
   */
  hold?: ReviewHold;
  /**
   * Drawn on the Logbook's Retros tab: every item, Armada, Kit and Manifest, one line each with the open one beside
   * them, no title and no place tabs. The host owns the tab.
   */
  embedded?: boolean;
  /** A retro to open over the list as the page opens: the one a Session's retro card asked for, by its number, or the newest. */
  opening?: { id: string; label: string; n?: number };
};

export function Lessons({
  onReadLessons,
  onReadRetro,
  onAgreeLesson,
  onDisagreeLesson,
  onStartSession,
  onOpenSession,
  onReviewLessons,
  onAskLesson,
  repository,
  floor,
  tab,
  onTab,
  hold,
  embedded = false,
  opening,
}: LessonsProps) {
  const own = useRef<ReviewHold | undefined>(undefined);
  const reviews = hold ?? (own.current ??= createReviewHold());
  // Review or the list is remembered with the review, the way the tab is remembered for the viewer.
  const reviewing = useSyncExternalStore(reviews.subscribe, reviews.reviewing);
  const [held, setHeld] = useState<LessonsTab>("all");
  const [view, setView] = useState<LessonsView>("open");
  const showing = embedded ? "all" : (tab ?? held);
  // Keyed by the pick and the list, so another of either is another read.
  // Leaving the review reads the list again: it answered items the list's own read still holds.
  const [reads, setReads] = useState(0);
  const read = useReadOnFocus(() => onReadLessons(view), `${repository ?? ""}:${view}:${reads}`);
  // A Session's Retro press opens the newest of that Session's retros: `n` absent.
  const [open, setOpen] = useState<{ subject: RetroSubject; session: string } | null>(
    opening === undefined ? null : { subject: { kind: "session", id: opening.id, ...(opening.n === undefined ? {} : { n: opening.n }) }, session: opening.label },
  );
  const sessions = sessionStartsOf(onStartSession, onOpenSession);
  const answered = useAnswers(onAgreeLesson, onDisagreeLesson, sessions);
  const retros = useRetros(onReadRetro);
  // The items whose Evidence was pressed, so a read shared by a retro's cards speaks on the one that asked.
  const [pressed, setPressed] = useState<ReadonlySet<string>>(new Set());
  const cited = new Map((read?.ok === true ? read.lessons : []).map((lesson) => [lesson.id, lesson.evidence]));
  const subjects = new Map((read?.ok === true ? read.lessons : []).map((lesson) => [lesson.id, retroSubjectOf(lesson)]));
  const leaveReview = () => {
    reviews.setReviewing(false);
    setReads((was) => was + 1);
  };
  const rows = underTab(read?.ok === true ? lessonRowsOf(read.lessons) : [], showing).flatMap((row) => {
    // **The list holds the ids a row cites; its retro holds the rows.** Read once per retro, on the first press.
    const ids = cited.get(row.id) ?? [];
    const subject = subjects.get(row.id) ?? { kind: "session" as const, id: row.sessionId };
    const got = retros.of(subject);
    let withEvidence: LessonRow = row;
    if (ids.length > 0 && got?.state === "read") {
      withEvidence = { ...row, cites: citesOf(got.retro, ids) };
    } else if (ids.length > 0) {
      withEvidence = {
        ...row,
        evidence: {
          onAsk: () => {
            setPressed((was) => new Set(was).add(row.id));
            retros.ask(subject);
          },
          // Only the card that asked shows the wait or the failure, though its retro's other cards share the read.
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

  const openRow = (sessionId: string, rowId: string) => {
    const row = rows.find((one) => one.id === rowId);
    setOpen({ subject: subjects.get(rowId) ?? { kind: "session", id: sessionId }, session: row?.session ?? sessionId });
  };

  return (
    <div className="armada-screen__overview armada-lessons-page">
      {read?.ok === false ? (
        <Alert tone="escalated" title="Retros could not be read">
          {said(read.outcome)}
        </Alert>
      ) : reviewing && onReviewLessons !== undefined && onAskLesson !== undefined ? (
        <>
          <div className="armada-lessons__bar">
            <Button variant="ghost" size="sm" onClick={leaveReview}>
              Back to the list
            </Button>
          </div>
          <LessonsReview
            lessons={read?.ok === true ? read.lessons : undefined}
            onReview={onReviewLessons}
            onAsk={onAskLesson}
            hold={reviews}
            holdKey={repository ?? ""}
            onReadRetro={onReadRetro}
            onAgreeLesson={onAgreeLesson}
            onDisagreeLesson={onDisagreeLesson}
            {...(sessions === undefined ? {} : { sessions })}
            onOpenRetro={(subject, label) => setOpen({ subject, session: label })}
            onList={leaveReview}
            paused={open !== null}
          />
        </>
      ) : (
        <>
          {embedded ? null : <h1 className="armada-page__title">Retros</h1>}
          <div className="armada-lessons__bar">
            {/* Where the fix lands, one place a tab. No count on any of them. */}
            {embedded ? null : (
              <Tabs
                items={[...LESSONS_TABS]}
                value={showing}
                onChange={(id) => {
                  const next = id as LessonsTab;
                  setHeld(next);
                  onTab?.(next);
                }}
              />
            )}
            <Tabs items={VIEWS} value={view} onChange={(id) => setView(id as LessonsView)} />
            {onReviewLessons === undefined || onAskLesson === undefined ? null : (
              <Button
                size="sm"
                onClick={() => {
                  // The review is of the open items, so the list it resolves them from is the open one.
                  setView("open");
                  reviews.setReviewing(true);
                }}
              >
                Review
              </Button>
            )}
          </div>
          {/* Before the read answers, and with nothing under the tab, nothing is drawn. */}
          {/* Inside the Logbook the items are a list and the one open beside it, as the entries are. */}
          {embedded ? <LessonSplit rows={rows} onOpen={openRow} /> : <LessonList rows={rows} onOpen={openRow} />}
        </>
      )}
      {open === null ? null : (
        <SessionRetroSheet
          subject={open.subject}
          session={open.session}
          read={onReadRetro}
          onAgreeLesson={onAgreeLesson}
          onDisagreeLesson={onDisagreeLesson}
          {...(sessions === undefined ? {} : { sessions })}
          floor={floor}
          onClose={() => setOpen(null)}
        />
      )}
    </div>
  );
}

/** Start Session's two halves as one, or nothing where the host gave only one of them. */
export function sessionStartsOf(
  start: StartLessonSession | undefined,
  open: ((sessionId: string) => void) | undefined,
): SessionStarts | undefined {
  return start === undefined || open === undefined ? undefined : { start, open };
}

/** `sessionStartsOf` as a prop to spread, for a host that passes the halves separately. */
export function withSessions(
  start: StartLessonSession | undefined,
  open: ((sessionId: string) => void) | undefined,
): { sessions?: SessionStarts } {
  const sessions = sessionStartsOf(start, open);
  return sessions === undefined ? {} : { sessions };
}

/**
 * One Session's retro on its sheet, read when it opens and again on focus. The
 * Retros page opens it from a row and a Session's thread from its retro card.
 */
export function SessionRetroSheet({
  subject,
  session,
  read,
  onAgreeLesson,
  onDisagreeLesson,
  sessions,
  floor,
  onClose,
}: {
  subject: RetroSubject;
  /** The Session as a person reads it, for the sheet's subtitle. */
  session: string;
  read: ReadRetro;
  onAgreeLesson: AnswerLesson;
  onDisagreeLesson: AnswerLesson;
  /** Start Session on an item of this retro; absent, and the sheet offers none. */
  sessions?: SessionStarts;
  floor: boolean;
  onClose: () => void;
}) {
  const answer = useReadOnFocus(() => read(subject), retroKeyOf(subject));
  const answered = useAnswers(onAgreeLesson, onDisagreeLesson, sessions);
  const retro = answer?.ok === true ? answer.retro : null;
  const status = retro === null ? undefined : statusOf(retro);
  const items = (retro === null ? [] : itemsOf(retro)).flatMap((item) => {
    const shown = answered(item.id ?? "", item.landsIn, {
      ...(item.state === undefined ? {} : { state: item.state }),
      ...(item.sessionProposed === undefined ? {} : { sessionProposed: item.sessionProposed }),
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
      session={session}
      reading={answer === undefined}
      {...(answer?.ok === false ? { failure: said(answer.outcome) } : {})}
      {...(status === undefined ? {} : { status })}
      items={items}
      floor={floor}
      onClose={onClose}
    />
  );
}
