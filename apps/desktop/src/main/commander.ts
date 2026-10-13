// One Job's Commander: the socket that carries its thread, and the two requests the owner sends
// into it. Held beside `helm.ts` for that file's reason: a second socket to the same peer, never a
// second peer, and one that can send. #2169.
//
// **One Job at a time, the open one.** `observe.ts`'s reason: a viewer opens one Job deliberately
// and closing it ends the watching. The card's answer is not tied to it, since the Cockpit answers
// for any Job it lists.
//
// **Nothing here advances a Job.** The owner's words queue for the next turn and never interrupt
// one, and a card's answer is a pick; Fleet wakes the Commander on both.

import WebSocket from "ws";

import type {
  AnswerCommanderCard,
  AskCommander,
  CommanderMessage,
  CommanderThread,
  HeldCommanderThread,
  Outcome,
} from "@armada/protocol";
import { commanderArrived, NO_COMMANDER_ITEMS, SOCKET_CLOSED } from "@armada/protocol";
import type { Guarded } from "./answers";
import { ask, route } from "./request";
import { HOST } from "./runtime-file";

/** The Commander socket of the open Job. */
export class CommanderSocket {
  private readonly publish: (thread: CommanderThread) => void;
  private socket: WebSocket | null = null;
  private jobId: string | null = null;
  private port: number | null = null;
  private held: HeldCommanderThread = NO_COMMANDER_ITEMS;
  /** An item's own identity, since none carries one: counted once per item actually added. */
  private seq = 0;
  /** What arrived this tick, folded once: a backfill lands as a few. */
  private pending: CommanderMessage[] = [];
  private folding: NodeJS.Immediate | null = null;

  constructor(publish: (thread: CommanderThread) => void) {
    this.publish = publish;
  }

  /** Which Job's Commander is watched, or `null` to stop. */
  open(port: number | null, jobId: string | null): void {
    this.close();
    this.jobId = jobId;
    this.port = port;
    this.held = NO_COMMANDER_ITEMS;
    this.seq = 0;
    if (jobId === null) {
      this.publish({ state: "none" });
      return;
    }
    if (port === null) {
      this.publish({ state: "failed", jobId, ...this.held, detail: "Fleet is not connected." });
      return;
    }
    this.publish({ state: "opening", jobId });
    this.connect(port, jobId);
  }

  /** Whether a socket is up. A reconnecting Fleet reopens one that is not. */
  attached(): boolean {
    return this.socket !== null;
  }

  close(): void {
    this.pending = [];
    if (this.folding !== null) clearImmediate(this.folding);
    this.folding = null;
    const socket = this.socket;
    this.socket = null;
    if (socket === null) return;
    socket.removeAllListeners();
    // One listener stays, `observe.ts`'s reason: a socket still shaking hands reports its abort
    // as an `error` on the next tick, with nothing listening, which Node throws.
    socket.on("error", () => {});
    socket.close();
  }

  private connect(port: number, jobId: string): void {
    const socket = new WebSocket(`ws://${HOST}:${port}${route(jobId, "commander/observe")}`);
    this.socket = socket;
    socket.on("message", (data: WebSocket.RawData) => this.arrived(String(data)));
    // A 404 before the upgrade and a socket that broke both land here. Neither is a Commander
    // with nothing to say, which is an open thread with no items.
    socket.on("error", (cause: Error) => this.broke(cause.message));
    socket.on("close", () => this.broke(SOCKET_CLOSED));
  }

  private arrived(text: string): void {
    if (this.jobId === null) return;
    let message: CommanderMessage;
    try {
      message = JSON.parse(text) as CommanderMessage;
    } catch {
      this.broke("Fleet sent a message this Bridge could not read.");
      return;
    }
    this.pending.push(message);
    this.folding ??= setImmediate(() => this.fold());
  }

  /** Everything that arrived since the last fold, folded at once and published once. */
  private fold(): void {
    const jobId = this.jobId;
    const pending = this.pending;
    this.pending = [];
    if (this.folding !== null) clearImmediate(this.folding);
    this.folding = null;
    if (jobId === null || pending.length === 0) return;

    for (const message of pending) {
      const next = commanderArrived(this.held, message, this.seq);
      if (next.added) this.seq += 1;
      this.held = next.held;
    }
    if (pending.some((one) => one.message === "missed") && this.port !== null) {
      // Messages were dropped before this Bridge read them, so the thread has a gap. Reopened, as
      // `observe.ts` does: the backfill redraws it whole, and what is held stays drawn until
      // its `opened` replaces it.
      const port = this.port;
      this.close();
      this.seq = 0;
      this.connect(port, jobId);
      return;
    }
    this.publish({ state: "open", jobId, ...this.held });
  }

  /** The socket went without a sentence. What had arrived stays arrived. */
  private broke(detail: string): void {
    this.fold();
    const jobId = this.jobId;
    if (jobId === null || this.socket === null) return;
    this.close();
    this.publish({ state: "failed", jobId, ...this.held, detail });
  }
}

/**
 * `POST /jobs/:job_id/commander/ask`. Answers at once, and the reply is the socket's. **Blank is
 * refused before it is sent**, matching the 422 Fleet would give it. A message sent mid-turn waits
 * for the next one, so there is nothing to guard against.
 */
export async function askCommander(port: number | null, jobId: string, text: string): Promise<Outcome> {
  if (port === null) return { ok: false, why: "not_connected" };
  if (text.trim() === "") return { ok: false, why: "empty_instruction" };
  const body: AskCommander = { text };
  const answered = await ask(port, "POST", route(jobId, "commander/ask"), body);
  return answered.ok === true ? { ok: true } : answered.outcome;
}

/** `POST /jobs/:job_id/commander/card/answer`: the card's id and an option's id, no prose. */
export function answerCommanderCard(
  guarded: Guarded,
  jobId: string,
  cardId: string,
  optionId: string,
): Promise<Outcome> {
  const body: AnswerCommanderCard = { card_id: cardId, option_id: optionId };
  return guarded(jobId, (port) => ask(port, "POST", route(jobId, "commander/card/answer"), body));
}
