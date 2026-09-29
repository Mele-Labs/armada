import type { ActivityActor } from "../ActivityLog/ActivityLog";
import type { DroneTurn, TurnStep } from "./DroneTurns";

/** One line of a block: a call, or a run of the Drone thinking. */
export type Member =
  | { of: "call"; turn: DroneTurn }
  /**
   * At least one row, always: a run is opened with the turn that started it
   * and only ever grows. Written as a non-empty tuple so the head below is a
   * turn rather than a lookup every caller has to assert.
   */
  | { of: "quiet"; turns: [DroneTurn, ...DroneTurn[]] };

type Item =
  | { of: "turn"; turn: DroneTurn }
  /** Consecutive calls and thinking, refused calls included. Non-empty for the same reason. */
  | { of: "block"; members: [Member, ...Member[]] };

/** One speaker's consecutive turns. Never empty, for the same reason as a run. */
export type CardEntry = { of: "card"; who?: ActivityActor; items: [Item, ...Item[]] };

type Entry =
  | CardEntry
  /** Keyed by the row it stands above, which is stable while rows only append. */
  | { of: "step"; step?: TurnStep; above: string };

/**
 * The rows, gathered into a card per run of one speaker, with calls and runs of
 * quiet rows gathered into blocks inside it and each change of step marked
 * between cards.
 *
 * **A run of one is still a run.** Left alone it renders the decoder's own
 * words for a turn it could not place, which is the reading this collapse
 * exists to remove, and one line that reads like its neighbours beats one that
 * does not.
 *
 * **A boundary breaks a card and a run**, because either spanning two steps
 * would attribute the whole of it to whichever the reader guessed.
 */
export function runs(turns: DroneTurn[], steps: boolean): Entry[] {
  // **Nothing is marked where no row anywhere carries a step.** Every row of
  // such a transcript predates the field, so "not recorded" would be the only
  // line on screen and would contrast with nothing. A transcript that gains a
  // step part-way through says so at the point it does.
  //
  // **With `steps` off nothing is marked and nothing splits on a step**: a card
  // broken at a line nobody sees would read as a random break.
  const attributed = steps && turns.some((turn) => turn.step !== undefined);
  const entries: Entry[] = [];
  let under: string | undefined;
  let opened = false;
  let card: CardEntry | undefined;
  for (const turn of turns) {
    if (attributed && (!opened || turn.step?.id !== under)) {
      entries.push({ of: "step", step: turn.step, above: turn.id });
      under = turn.step?.id;
      opened = true;
      card = undefined;
    }
    const member: Member | undefined =
      turn.quiet === true ? { of: "quiet", turns: [turn] } : isCall(turn) ? { of: "call", turn } : undefined;
    const item: Item = member === undefined ? { of: "turn", turn } : { of: "block", members: [member] };
    // **A turn with no speaker never shares a card**, not even with another
    // unnamed one: two rows under one head says one speaker wrote both.
    if (card === undefined || card.who === undefined || card.who !== turn.who) {
      card = { of: "card", ...(turn.who === undefined ? {} : { who: turn.who }), items: [item] };
      entries.push(card);
      continue;
    }
    // Only a sentence, or another row that is neither, ends a block.
    const last = card.items[card.items.length - 1];
    if (member !== undefined && last?.of === "block") {
      const tail = last.members[last.members.length - 1];
      if (member.of === "quiet" && tail?.of === "quiet") tail.turns.push(turn);
      else last.members.push(member);
      continue;
    }
    card.items.push(item);
  }
  return entries;
}

/** By kind, never by shape: a session line carries a subject alone, as a bare call does. */
function isCall(turn: DroneTurn): boolean {
  return turn.kind === "called" || turn.kind === "refused";
}

/** The turn a card opens with, whose instant its head carries. */
export function firstOf(card: CardEntry): DroneTurn {
  const head = card.items[0];
  return head.of === "turn" ? head.turn : leadOf(head.members[0]);
}

export function leadOf(member: Member): DroneTurn {
  return member.of === "call" ? member.turn : member.turns[0];
}
