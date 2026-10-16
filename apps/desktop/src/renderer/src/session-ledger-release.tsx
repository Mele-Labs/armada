// Release on one row of a Session's ledger: a confirm naming the row, then `release_session_holds` with that row and no target.
// The owner, 16 Oct 2026: "I should also be able to release items from the ledger panel one by one." Fleet leaves a row given back
// out of `attachments`, so the row as it stood is kept here and drawn given back until the view moves to another Session.

import { useState } from "react";
import type { ReactNode } from "react";
import type { LedgerEntry } from "@armada/components";
import type { Session, SessionAttachment, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { ReleaseRowDialog } from "./cockpit/deck/ReleaseHolds";
import { heldKey, heldOfAttachment, type Held } from "./cockpit/deck/session-tiles";

/** A row released from here: as it stood when pressed, and Fleet's why where it kept it. */
type Released = { attachment: SessionAttachment; kept?: string };

const NONE: ReadonlyMap<string, Released> = new Map();

export type LedgerRelease = {
  /** The Session with each row given back from here added again, so the ledger still draws it. */
  shown: Session;
  /** What a row's entry carries: Release, or what came of it. */
  rowOf: (one: SessionAttachment, entry: Omit<LedgerEntry, "card">) => Pick<LedgerEntry, "release" | "released">;
  /** The confirm, while one is open. */
  dialog: ReactNode;
};

export function useLedgerRelease(session: Session, draft: SessionsDraft | undefined): LedgerRelease {
  const [asking, setAsking] = useState<Held | undefined>();
  const [released, setReleased] = useState<{ id: string; rows: ReadonlyMap<string, Released> } | undefined>();
  const rows = released?.id === session.id ? released.rows : NONE;
  const send = draft?.releaseHolds;
  const release = async (one: Held): Promise<boolean> => {
    const before = session.attachments;
    const answer = await send?.(session.id, undefined, [{ kind: one.kind, target: one.target }]);
    if (answer === undefined) return false;
    const at = (row: { kind: string; target: string }) =>
      before.find((it) => {
        const held = heldOfAttachment(it);
        return held !== undefined && heldKey(held) === heldKey(row);
      });
    setReleased((was) => {
      const next = new Map(was?.id === session.id ? was.rows : NONE);
      // A slot takes its branch with it, so `moved` can name more than the one row pressed.
      for (const row of answer.moved) {
        const attachment = at(row);
        if (attachment !== undefined) next.set(heldKey(row), { attachment });
      }
      for (const row of answer.kept) {
        const attachment = at(row);
        if (attachment !== undefined) next.set(heldKey(row), { attachment, kept: row.why });
      }
      return { id: session.id, rows: next };
    });
    setAsking(undefined);
    return true;
  };
  // A branch row carries its slot's chip, and the slot has no row of its own, so its Release gives back the slot, which takes the branch.
  const carriedSlot = (one: SessionAttachment): Held | undefined => {
    if (one.kind !== "branch" || one.slot === undefined) return undefined;
    const slot = session.attachments.find((it) => it.kind === "slot" && it.slot === one.slot);
    const held = slot === undefined ? undefined : heldOfAttachment(slot);
    return held === undefined ? undefined : { ...held, name: `${one.name} in ${held.name}` };
  };
  const live = new Set(session.attachments.map((one) => heldOfAttachment(one)).flatMap((one) => (one === undefined ? [] : [heldKey(one)])));
  const gone = [...rows.entries()].filter(([key, one]) => one.kept === undefined && !live.has(key)).map(([, one]) => one.attachment);
  return {
    shown: gone.length === 0 ? session : { ...session, attachments: [...session.attachments, ...gone] },
    rowOf: (one, entry) => {
      const held = heldOfAttachment(one);
      if (held === undefined) return {};
      const asked = carriedSlot(one) ?? held;
      const was = rows.get(heldKey(held)) ?? rows.get(heldKey(asked));
      if (was !== undefined && was.kept === undefined) return { released: {} };
      const offered = send !== undefined && session.dead !== "ended" && entry.finished !== true;
      return {
        ...(offered ? { release: { name: held.name, onPress: () => setAsking(asked) } } : {}),
        ...(was?.kept === undefined ? {} : { released: { kept: was.kept } }),
      };
    },
    dialog: asking === undefined ? null : <ReleaseRowDialog held={asking} send={release} onCancel={() => setAsking(undefined)} />,
  };
}
