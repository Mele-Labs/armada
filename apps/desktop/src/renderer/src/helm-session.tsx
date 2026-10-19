// Helm drawn as the Session it is hosted as: its thread as a Session's thread is drawn, what it holds
// as chips under the dock's head, and the sheets those chips open. The dock keeps what is Helm's own.
// Apart from `App.tsx`, which is at its length. A repository with no Helm Session draws as before.

import { useEffect, useMemo, useState, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import { HelmHolds, SessionThread, TileSheet } from "@armada/components";
import type { HelmDockProps } from "@armada/helm";
import { helmOfferedOf } from "@armada/screens/src/copy";
import type { Session, SessionAnswer } from "@armada/screens/src/draft/sessions";
import { useAtFloor } from "@armada/shell";

import { useBackfilled } from "./backfilled";
import { openInPanel } from "./shared-panel/store";
import { entriesOf, ReadingSheet, readingOf, stepsReadingOf } from "./sessions";
import type { HeldReads, LedgerGoes, ReadAsked, Reading } from "./sessions";
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
  openSession: (id: string) => void;
  /** The words held in Helm's thread while they go. */
  words: HelmWords;
};

/** What the dock is given to draw Helm as a Session, and the sheets its chips open, drawn once beside the dock. */
export function useHelmHosted({ manifestId, goes, held, openSession, words }: Host): { hosted: HelmDockProps["hosted"]; sheets: ReactNode } {
  const draft = useSessionsDraft();
  const sessions = useSessions();
  const session = useHelmSession(manifestId);
  const floor = useAtFloor();
  const [reading, setReading] = useState<Reading | undefined>();
  const [slotOpen, setSlotOpen] = useState<number | undefined>();
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
  if (draft === undefined) return { hosted: undefined, sheets: null };
  // The repository's first message has no Session yet and so no thread: the words are shown where it will be.
  if (session === undefined) {
    if (!waiting) return { hosted: undefined, sheets: null };
    return {
      hosted: {
        working: false,
        thread: <SessionThread sessionId={words.key} rows={thread} agentName="Helm" onRetry={words.retry} onAnswer={() => undefined} onOpenSession={openSession} />,
      },
      sheets: null,
    };
  }

  // A pull request opens in the side panel; the other readings open beside the thread.
  const read = (one: ReadAsked) => {
    if (one.kind !== "pull_request") return setReading(one);
    const attached = session.attachments.find((a) => a.kind === "pull_request" && a.number === one.number);
    openInPanel({ kind: "pr", number: one.number, address: attached?.kind === "pull_request" ? attached.address : "" });
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
        rows={thread}
        onRetry={words.retry}
        agentName="Helm"
        working={session.turn.state === "working"}
        {...(draft.stop === undefined ? {} : { onStop: () => draft.stop?.(session.id) })}
        onOpenWindow={(url) => draft.openWindow?.(session.id, url)}
        onOpenCheck={(run) => goes.onOpenCheck(session.id, run)}
        onOpenSent={(target) => openSession(target.id)}
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
        onClose={() => setReading(undefined)}
        onOpenLink={goes.onOpenLink}
      />
      {slot === undefined ? null : <TileSheet row={{ slot }} floor={floor} onClose={() => setSlotOpen(undefined)} />}
    </>
  );
  return { hosted, sheets };
}
