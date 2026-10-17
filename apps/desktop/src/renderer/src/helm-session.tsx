// Helm drawn as the Session it is hosted as: its thread as a Session's thread is drawn, what it holds
// as chips under the dock's head, and the sheets those chips open. The dock keeps what is Helm's own.
// Apart from `App.tsx`, which is at its length. A repository with no Helm Session draws as before.

import { useEffect, useMemo, useState, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import { HelmHolds, SessionThread, TileSheet } from "@armada/components";
import type { HelmDockProps } from "@armada/helm";
import { helmOfferedOf } from "@armada/screens/src/copy";
import type { Session, SessionAnswer } from "@armada/screens/src/draft/sessions";
import type { JobSummary, MergeLines, SettingsList, WorkflowSummary } from "@armada/protocol";
import { useAtFloor } from "@armada/shell";

import { useBackfilled } from "./backfilled";
import { withApprovals } from "./helm-approval";
import type { Pressed } from "./helm-approval";
import { proposeRequest } from "./dispatch";
import { pullRequestPlaceOf } from "./pull-request-place";
import { entriesOf, ReadingSheet, readingOf, stepsReadingOf } from "./sessions";
import type { HeldReads, LedgerGoes, Reading } from "./sessions";
import { useSessions, useSessionsDraft } from "./sessions-draft";
import { threadRowsFrom } from "./session-thread-rows";
import type { HelmWords } from "./helm-box";

const NO_ROWS: ReturnType<typeof threadRowsFrom> = [];
const NO_SESSION_ROWS: Session["rows"] = [];

/** Helm's Session for the repository the dock answers for, among the Sessions the window lists. */
export function helmSessionOf(sessions: readonly Session[], manifestId: string | undefined): Session | undefined {
  return manifestId === undefined ? undefined : sessions.find((one) => one.helm === true && one.dead === undefined && one.manifestId === manifestId);
}

/**
 * Helm's Session for the repository the dock answers for. **Whatever the rail picked**: a real Fleet's list
 * holds the picked repository's Sessions, and Helm may be pointed at another, so it is asked for by name;
 * where the list holds every Session, it is found there.
 */
export function useHelmSession(manifestId: string | undefined): Session | undefined {
  const draft = useSessionsDraft();
  const sessions = useSessions();
  if (manifestId === undefined) return undefined;
  return draft?.helmOf?.(manifestId) ?? helmSessionOf(sessions, manifestId);
}

type Host = {
  manifestId: string | undefined;
  goes: LedgerGoes;
  held: HeldReads;
  board: readonly JobSummary[];
  workflows: readonly WorkflowSummary[];
  /** The press on a Job's approval gate, which an approval Helm asked for is answered with. */
  onApprove: (jobId: string) => Promise<{ ok: boolean }>;
  lines: MergeLines | null;
  settings: SettingsList | null;
  openSession: (id: string) => void;
  openJob: (jobId: string) => void;
  /** The words held in Helm's thread while they go. */
  words: HelmWords;
};

/** What the dock is given to draw Helm as a Session, and the sheets its chips open, drawn once beside the dock. */
export function useHelmHosted({ manifestId, goes, held, board, workflows, onApprove, lines, settings, openSession, openJob, words }: Host): { hosted: HelmDockProps["hosted"]; sheets: ReactNode } {
  const draft = useSessionsDraft();
  const sessions = useSessions();
  const session = useHelmSession(manifestId);
  const floor = useAtFloor();
  const [reading, setReading] = useState<Reading | undefined>();
  const [slotOpen, setSlotOpen] = useState<number | undefined>();
  const [pressed, setPressed] = useState<Record<string, Pressed>>({});
  const id = session?.id;
  const watch = draft?.watch;
  // A real Fleet reads a thread when it is opened and follows it from there.
  useEffect(() => (id === undefined ? undefined : watch?.(id)), [watch, id]);
  const { onWant } = held;
  // The panel a slot's chip opens reads what Fleet holds, so it is wanted while Helm holds one.
  const holdsSlot = session?.attachments.some((one) => one.kind === "slot") === true;
  useEffect(() => {
    if (!holdsSlot) return undefined;
    onWant(true);
    return () => onWant(false);
  }, [holdsSlot, onWant]);
  // Words the person sent are in the thread from the press, going, and Fleet's row takes their place when it arrives.
  const version = useSyncExternalStore(words.engine.subscribe, () => words.engine.version(words.key), () => 0);
  const rows = useMemo(() => words.engine.overlay(words.key, session?.rows ?? NO_SESSION_ROWS), [words, session?.rows, version]);
  const waiting = session === undefined && rows.length > 0;
  const thread = useBackfilled(session === undefined && !waiting ? NO_ROWS : threadRowsFrom(rows, session?.attachments), id ?? words.key);
  const drawn = withApprovals(thread, {
    jobs: board,
    workflows,
    pressed,
    onApprove,
    onPress: (cardId, press) => setPressed(({ [cardId]: _was, ...rest }) => (press === undefined ? rest : { ...rest, [cardId]: press })),
  });
  if (draft === undefined) return { hosted: undefined, sheets: null };
  // The repository's first message has no Session yet and so no thread: the words are shown where it will be.
  if (session === undefined) {
    if (!waiting) return { hosted: undefined, sheets: null };
    return {
      hosted: {
        working: false,
        thread: <SessionThread sessionId={words.key} rows={drawn} agentName="Helm" onRetry={words.retry} onAnswer={() => undefined} onOpenSession={openSession} />,
      },
      sheets: null,
    };
  }

  // `links.openPullRequestsIn` decides where a pull request opens: Armada's panel, or the forge in the browser.
  const read = (one: Reading) => {
    const address = one.kind === "pull_request" ? session.attachments.find((a) => a.kind === "pull_request" && a.number === one.number) : undefined;
    if (address?.kind === "pull_request" && pullRequestPlaceOf(settings) === "browser") goes.onOpenLink(address.address);
    else setReading(one);
  };
  const entries = entriesOf(
    session,
    goes,
    read,
    setSlotOpen,
    sessions,
    openSession,
    (url) => draft.openWindow?.(session.id, url),
    undefined,
    () => undefined,
    { slots: held.held.state === "read" ? (held.held.held.slots ?? []) : [] },
    undefined,
  );
  const slot = held.held.state === "read" ? (held.held.held.slots ?? []).find((one) => one.slot === slotOpen) : undefined;
  const questions = session.asked?.questions;
  const hosted: NonNullable<HelmDockProps["hosted"]> = {
    working: session.turn.state === "working",
    holds: <HelmHolds entries={entries} />,
    thread: (
      <SessionThread
        sessionId={session.id}
        rows={drawn}
        onRetry={words.retry}
        agentName="Helm"
        working={session.turn.state === "working"}
        {...(draft.stop === undefined ? {} : { onStop: () => draft.stop?.(session.id) })}
        onOpenWindow={(url) => draft.openWindow?.(session.id, url)}
        onOpenCheck={(run) => goes.onOpenCheck(session.id, run)}
        onOpenSent={(target) => (target.kind === "session" ? openSession(target.id) : openJob(target.id))}
        // Helm runs in auto, so a permission is not asked here: what holds it is a question it put.
        {...(questions === undefined || session.asked === undefined
          ? {}
          : {
              asked: {
                command: session.asked.command,
                questions,
                ...(session.asked.offers === undefined
                  ? {}
                  : { offers: helmOfferedOf(session.asked.offers).map((one) => ({ id: one.offer, label: one.label, means: one.means })) }),
              },
            })}
        onAnswer={(answer, answers) => draft.answer(session.id, answer as SessionAnswer | undefined, answers)}
        onOpenSession={openSession}
      />
    ),
  };
  const sheets = (
    <>
      <ReadingSheet
        sessionId={session.id}
        readSubagent={draft.subagent}
        one={readingOf(session, reading)}
        steps={stepsReadingOf(session, reading, draft)}
        draft={draft}
        lines={lines}
        onClose={() => setReading(undefined)}
        onOpenLink={goes.onOpenLink}
        onAct={(number, act) => {
          const done = draft.act(session.id, number, act);
          // Review is a request: the link is the whole of it, and the proposer picks the code review workflow.
          const opened = session.attachments.find((one) => one.kind === "pull_request" && one.number === number);
          if (act === "review" && opened?.kind === "pull_request" && draft.reviews !== "fleet") void proposeRequest(opened.address, []);
          return done;
        }}
      />
      {slot === undefined ? null : <TileSheet row={{ slot }} floor={floor} onOpenJob={goes.onOpenJob} onClose={() => setSlotOpen(undefined)} />}
    </>
  );
  return { hosted, sheets };
}
