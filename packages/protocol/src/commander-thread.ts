// What `GET /jobs/:job_id/commander/observe`'s socket comes to, folded. `helm-thread.ts`'s shape
// with a Job for its subject: the fold is written once here, so a socket in main and a recording
// replayed in a test draw the same thread.
//
// **Two things a Helm thread has that this does not: `sent` and `closed`.** Nothing is routed to a
// Commander as a thought, and Fleet never forgets its thread under a viewer. What it has that
// Helm's does not is `woke`, the fact that started a turn.

import type { CommanderMessage } from "./commander";
import type { Turn } from "./reads";

/** One thing a viewer of a Job's Commander is shown, in order. */
export type CommanderThreadItem =
  /** Fleet started a turn, and the fact that did. */
  | { kind: "woke"; id: string; ts: string; why: string }
  /** What the owner said. */
  | { kind: "asked"; id: string; ts: string; text: string }
  /** One row of the session's own turn, `turn.ts`'s shape. */
  | { kind: "row"; id: string; turn: Turn }
  /** The stored session could not be resumed; a new one wrote what follows. */
  | { kind: "fresh"; id: string; ts: string; because: string }
  /** No reply came, and why. */
  | { kind: "unanswered"; id: string; ts: string; why: string };

/**
 * One Job's Commander conversation, as a viewer holds it. **`open` with no items is a Commander
 * nobody has woken or written to**, which is not `opening`, the socket still shaking hands.
 */
export type CommanderThread =
  | { state: "none" }
  | { state: "opening"; jobId: string }
  | {
      state: "open";
      jobId: string;
      /** A turn is being written, or the owner's words are waiting for the next one. */
      replying: boolean;
      /** Older messages the bounded backfill left out, from `opened`. */
      skipped: number;
      /** Messages this viewer fell behind and lost. */
      missed: number;
      items: CommanderThreadItem[];
    }
  | {
      state: "failed";
      jobId: string;
      replying: boolean;
      skipped: number;
      missed: number;
      items: CommanderThreadItem[];
      detail: string;
    };

/** Nothing is being watched. */
export const NO_COMMANDER_THREAD: CommanderThread = { state: "none" };

/** What the fold holds between messages: `CommanderThread`'s `open` and `failed` states, without the tag. */
export type HeldCommanderThread = {
  replying: boolean;
  skipped: number;
  missed: number;
  items: CommanderThreadItem[];
};

/** Nothing has arrived yet, and nothing has been lost. */
export const NO_COMMANDER_ITEMS: HeldCommanderThread = { replying: false, skipped: 0, missed: 0, items: [] };

/**
 * What one message on the socket does to the thread held so far. `seq` is an item's own identity,
 * counted by the caller once per item that was actually added, which `added` says.
 *
 * **A turn is over at its `ended` row.** Anything else on a turn means it is still going, and a
 * `woke` or an `asked` means one is about to start.
 */
export function commanderArrived(
  held: HeldCommanderThread,
  message: CommanderMessage,
  seq: number,
): { held: HeldCommanderThread; added: boolean } {
  switch (message.message) {
    case "opened":
      return { held: { replying: message.replying, skipped: message.skipped, missed: 0, items: [] }, added: false };
    case "woke":
      return {
        held: { ...held, replying: true, items: [...held.items, { kind: "woke", id: String(seq), ts: message.ts, why: message.why }] },
        added: true,
      };
    case "asked":
      return {
        held: { ...held, replying: true, items: [...held.items, { kind: "asked", id: String(seq), ts: message.ts, text: message.text }] },
        added: true,
      };
    case "row": {
      const { message: _tag, ts, step, by, ...saw } = message;
      const turn: Turn = { ts, seq, step, by: by ?? "drone", saw };
      return {
        held: { ...held, replying: saw.event !== "ended", items: [...held.items, { kind: "row", id: String(seq), turn }] },
        added: true,
      };
    }
    case "fresh":
      return {
        held: { ...held, replying: true, items: [...held.items, { kind: "fresh", id: String(seq), ts: message.ts, because: message.because }] },
        added: true,
      };
    case "unanswered":
      return {
        held: { ...held, replying: false, items: [...held.items, { kind: "unanswered", id: String(seq), ts: message.ts, why: message.why }] },
        added: true,
      };
    case "missed":
      // Counted and said, never skipped quietly, `turnArrived`'s own reason.
      return { held: { ...held, missed: held.missed + message.dropped }, added: false };
  }
}
