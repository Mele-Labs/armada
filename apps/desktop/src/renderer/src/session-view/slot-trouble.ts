// Every bay a Session holds, and which of them are worth a look: stranded, or a checkout well behind its base. Its write clashes are the
// Session's, not one bay's. Everything is read from what Fleet already holds (the pool and the clashes); nothing is guessed.

import type { WorktreeSlot, WriteClash } from "@armada/protocol";
import { attachmentsOf, type Session } from "@armada/screens/src/draft/sessions";

/** Commits behind its base from which a held checkout is worth saying. */
export const BEHIND_FROM = 20;

export type BayWrong = "stranded" | "behind";

export type HeldBay = {
  slot: WorktreeSlot;
  /** The branch the Session works on in it, where it has one. */
  branch: string | undefined;
  /** What is wrong with it, where something is. */
  wrong: BayWrong | undefined;
  /** What is wrong, as the call in the thread says it. */
  said: string | undefined;
};

export type SessionBays = {
  bays: readonly HeldBay[];
  /** Who the Session's work clashes with. */
  clashes: readonly WriteClash[];
};

/** The Bays key is lit: a bay is wrong. A write clash is its glyph on the key, and does not light it. */
export const baysLit = (held: SessionBays): boolean => held.bays.some((one) => one.wrong !== undefined);

/** The slot numbers the Session names, from its slots and its branches. */
function namedSlots(session: Session): Set<number> {
  const named = [...attachmentsOf(session, "slot").map((one) => one.slot), ...attachmentsOf(session, "branch").map((one) => one.slot)];
  return new Set(named.filter((one): one is number => one !== undefined && one !== 0));
}

/** The pool's slots this Session holds, or held last before they stranded, in slot order. */
export function baysOf(session: Session, slots: readonly WorktreeSlot[], clashes: readonly WriteClash[]): SessionBays {
  const named = namedSlots(session);
  const branches = attachmentsOf(session, "branch");
  const bays = slots
    .filter((one) => session.manifestId === undefined || one.manifest_id === session.manifestId)
    .filter(
      (one) =>
        named.has(one.slot) ||
        ((one.held.state === "hosted_session" || one.held.state === "session") && one.held.session_id === session.id) ||
        (one.stranded?.last_holder?.kind === "session" && one.stranded.last_holder.id === session.id),
    )
    .sort((a, b) => a.slot - b.slot)
    .map((slot): HeldBay => {
      const branch = branches.find((one) => one.slot === slot.slot)?.name ?? slot.branch;
      const named = `Slot ${slot.slot}`;
      if (slot.held.state === "stranded") return { slot, branch, wrong: "stranded", said: `${named} is stranded` };
      if ((slot.behind ?? 0) >= BEHIND_FROM) return { slot, branch, wrong: "behind", said: `${named} is ${slot.behind} commits behind ${slot.base}` };
      return { slot, branch, wrong: undefined, said: undefined };
    });
  const mine = clashes.filter((one) => (one.one.kind === "session" && one.one.id === session.id) || (one.other.kind === "session" && one.other.id === session.id));
  return { bays, clashes: mine };
}
