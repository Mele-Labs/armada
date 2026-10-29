// A Job another Job's agent started or forked when the owner asked it to. **It waits for him**: it is held on its start card,
// the agent's first message sits in its box, and nothing runs until he presses Start.

import { useEffect } from "react";
import type { ReactNode } from "react";
import { forgetKept, readKept, seedDraft, StartedBy, writeKept } from "@armada/components";
import type { LedgerEntry } from "@armada/components";
import type { Session, SessionAttachment } from "@armada/screens/src/draft/sessions";

/** A fork or a start on the ledger, of the Session that made it and of the one it made. */
export function linkEntry(one: Extract<SessionAttachment, { kind: "forked_to" | "forked_from" | "started_to" | "started_by" }>, sessions: readonly Session[], open: (id: string) => void): LedgerEntry {
  const title = sessions.find((other) => other.id === one.id)?.title ?? one.id;
  const moved = (one.kind === "forked_to" || one.kind === "forked_from") && one.moved !== undefined;
  const way = moved ? (one.kind === "forked_to" ? "Moved to" : "Moved from") : { forked_to: "Forked to", forked_from: "Forked from", started_to: "Started", started_by: "Started by" }[one.kind];
  const name = `${way} ${title}`;
  return { key: `${one.kind}${one.id}`, kind: one.kind.startsWith("fork") ? "fork" : "start", name, text: name, onOpen: () => open(one.id) };
}

/**
 * The row above the thread naming the Session whose agent started this one, if one did. Also seeds the box with its first
 * message, and takes the seed back out once the Job has begun some other way than this box, as its first turn then went
 * without it.
 */
export function useStartedBy(session: Session, sessions: readonly Session[], open: (id: string) => void): ReactNode {
  const { id } = session;
  // The first message a held Job will begin with.
  const first = session.start?.message;
  useEffect(() => {
    const box = `session:${id}`;
    const seeded = `seeded:${id}`;
    if (first !== undefined) {
      if (readKept(box) !== undefined) return;
      seedDraft(id, first);
      writeKept(seeded, readKept<{ text: string }>(box)?.text ?? first);
      return;
    }
    const was = readKept<string>(seeded);
    if (was === undefined) return;
    if (readKept<{ text: string }>(box)?.text === was) forgetKept(box);
    forgetKept(seeded);
  }, [id, first]);
  const by = session.attachments.find((one): one is Extract<SessionAttachment, { kind: "started_by" }> => one.kind === "started_by");
  const from = by === undefined ? undefined : sessions.find((other) => other.id === by.id);
  return from === undefined ? null : <StartedBy title={from.title ?? from.id} onOpen={() => open(from.id)} />;
}
