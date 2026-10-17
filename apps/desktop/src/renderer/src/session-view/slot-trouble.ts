// When a Session's slot and branch are worth a look. They are drawn only then: a stranded slot, a checkout well behind its base, or a
// write clash with another holder. Everything is read from what Fleet already holds (the pool and the clashes); nothing is guessed.

import type { WorktreeSlot, WriteClash } from "@armada/protocol";
import { attachmentsOf, type Session } from "@armada/screens/src/draft/sessions";

/** Commits behind its base from which a held checkout is worth saying. */
export const BEHIND_FROM = 20;

export type SlotTroubleKind = "stranded" | "behind" | "conflicting";

export type SlotTrouble = {
  kind: SlotTroubleKind;
  slot: WorktreeSlot;
  /** The branch the Session works on in it, where it has one. */
  branch: string | undefined;
  /** What is wrong, as the call in the thread and the key's tooltip say it. */
  said: string;
  /** Who it clashes with, for a conflict. */
  clashes: readonly WriteClash[];
};

/** The slot number the Session works in, from its slot or its branch. */
export function slotOf(session: Session): number | undefined {
  const branch = attachmentsOf(session, "branch")[0];
  const slot = attachmentsOf(session, "slot")[0]?.slot ?? branch?.slot;
  return slot === undefined || slot === 0 ? undefined : slot;
}

export function slotTroubleOf(session: Session, slots: readonly WorktreeSlot[], clashes: readonly WriteClash[]): SlotTrouble | undefined {
  const number = slotOf(session);
  if (number === undefined) return undefined;
  const slot = slots.find((one) => one.slot === number && (session.manifestId === undefined || one.manifest_id === session.manifestId));
  if (slot === undefined) return undefined;
  const branch = attachmentsOf(session, "branch")[0]?.name ?? slot.branch;
  const mine = clashes.filter((one) => (one.one.kind === "session" && one.one.id === session.id) || (one.other.kind === "session" && one.other.id === session.id));
  const named = `Slot ${slot.slot}`;
  if (slot.held.state === "stranded") return { kind: "stranded", slot, branch, said: `${named} is stranded`, clashes: mine };
  if (mine.length > 0) return { kind: "conflicting", slot, branch, said: `${named} conflicts with another holder`, clashes: mine };
  if ((slot.behind ?? 0) >= BEHIND_FROM) return { kind: "behind", slot, branch, said: `${named} is ${slot.behind} commits behind ${slot.base}`, clashes: mine };
  return undefined;
}
