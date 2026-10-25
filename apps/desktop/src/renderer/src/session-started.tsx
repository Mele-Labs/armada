// A Session another Session's agent started or forked when the owner asked it to. **It waits for him**:
// the agent's first message sits in the new Session's box unsent, and nothing runs until he sends it.

import { useEffect } from "react";
import type { ReactNode } from "react";
import { readKept, seedDraft, StartedBy } from "@armada/components";
import type { LedgerEntry } from "@armada/components";
import type { WaitingItem } from "@armada/protocol";
import type { Session, SessionAttachment } from "@armada/screens/src/draft/sessions";

/** The item that waits on the person sending the first message: Fleet gives it the source `first_message`. */
export const isUnsentItem = (item: WaitingItem): boolean => item.source === "first_message";

/** The row for the item that waits on the first message: a press goes to the box. */
export function unsentEntry(item: WaitingItem, goToBox: () => void): LedgerEntry {
  return {
    key: `waiting${item.id}`,
    kind: "waiting",
    name: item.text,
    text: item.text,
    act: "message",
    onOpen: () => {
      goToBox();
      document.querySelector<HTMLElement>('[role="textbox"][aria-label="Message"]')?.focus();
    },
  };
}

/** A fork or a start on the ledger, of the Session that made it and of the one it made. */
export function linkEntry(one: Extract<SessionAttachment, { kind: "forked_to" | "forked_from" | "started_to" | "started_by" }>, sessions: readonly Session[], open: (id: string) => void): LedgerEntry {
  const title = sessions.find((other) => other.id === one.id)?.title ?? one.id;
  const moved = (one.kind === "forked_to" || one.kind === "forked_from") && one.moved !== undefined;
  const way = moved ? (one.kind === "forked_to" ? "Moved to" : "Moved from") : { forked_to: "Forked to", forked_from: "Forked from", started_to: "Started", started_by: "Started by" }[one.kind];
  const name = `${way} ${title}`;
  return { key: `${one.kind}${one.id}`, kind: one.kind.startsWith("fork") ? "fork" : "start", name, text: name, onOpen: () => open(one.id) };
}

/** The row above the thread naming the Session whose agent started this one, if one did. Also seeds the box with its first message. */
export function useStartedBy(session: Session, sessions: readonly Session[], open: (id: string) => void): ReactNode {
  const { id } = session;
  // The words a Job waits to send: what its agent left unsent, or the first message a held Job will begin with.
  const unsent = session.unsent ?? session.start?.message;
  useEffect(() => {
    if (unsent !== undefined && readKept(`session:${id}`) === undefined) seedDraft(id, unsent);
  }, [id, unsent]);
  const by = session.attachments.find((one): one is Extract<SessionAttachment, { kind: "started_by" }> => one.kind === "started_by");
  const from = by === undefined ? undefined : sessions.find((other) => other.id === by.id);
  return from === undefined ? null : <StartedBy title={from.title ?? from.id} onOpen={() => open(from.id)} />;
}
