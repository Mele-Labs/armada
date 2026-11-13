// What this phone sent the Operator and what became of it. A send goes to the chosen Mac's own Gateway, or through another
// Mac's where the chosen one is asleep (`POST /api/sessions/dispatch`), which answers at once with a Job, the id of a dispatch the Operator holds, or, for a
// Mac that does not answer, the act it kept (202). What the Operator did with a held dispatch arrives on a Mac's stream
// (`operator.dispatched`), and what waits for a sleeping Mac is `/api/queued`, read again on every `queued` message.
// Rows live as long as the page does: Fleet keeps no list of sends, only of what waits.

import { useSyncExternalStore } from "react";

import { Refused, call, getJson, refusal } from "./client";
import { macOf, onOperator, type Mac } from "./fleet";
import type {
  OperatorAnswer,
  OperatorBody,
  OperatorDispatched,
  QueuedAct,
  QueuedBody,
} from "./gateway";
import { bodyOf } from "./jobState";
import type { Machine } from "./machines";

export type Kind = "idea" | "feature" | "bug";

export type Stage =
  | {
      phase: "reading";
    }
  | ({
      phase: "dispatched";
    } & OperatorDispatched)
  | {
      phase: "queued";
      /** Milliseconds since the epoch, on the Mac that kept it. */
      since: number;
    }
  | {
      phase: "refused";
      why: string;
    };

export type Send = {
  id: string;
  /** RFC 3339. */
  at: string;
  /** The Mac it was sent for, by the name the phone knows it by. Empty for a waiting send this page did not make. */
  mac: string;
  /** Where its Job opens: the Mac it was sent for. */
  origin?: string;
  repository?: string;
  words: string;
  kind?: Kind;
  stage: Stage;
  /** The id `operator.dispatched` will name it by. */
  dispatchId?: string;
  /** Where Fleet keeps it while a Mac cannot be reached: the Mac holding it, and its id. */
  kept?: {
    origin: string;
    id: string;
  };
};

let sends: Send[] = [];
/** What the Operator did, heard before the answer that names the dispatch. */
const early = new Map<string, OperatorDispatched>();
const watchers = new Set<() => void>();
const emit = () => watchers.forEach((watch) => watch());
const watching = (watch: () => void) => {
  watchers.add(watch);
  return () => watchers.delete(watch);
};
const set = (next: Send[]) => {
  sends = [...next].sort((a, b) => (a.at < b.at ? 1 : a.at > b.at ? -1 : 0));
  emit();
};
const change = (id: string, to: Partial<Send>) =>
  set(sends.map((one) => (one.id === id ? { ...one, ...to } : one)));

export const useSends = (): Send[] => useSyncExternalStore(watching, () => sends);

/** The name and repository a waiting send was made with, kept by its id so a reload still names its Mac. */
const REMEMBERED = "pocket:operator:waiting";
type Remembered = Record<
  string,
  {
    mac: string;
    origin: string;
    repository: string;
  }
>;
const remembered = (): Remembered => {
  try {
    return JSON.parse(window.localStorage.getItem(REMEMBERED) ?? "{}") as Remembered;
  } catch {
    return {};
  }
};
const remember = (id: string, about: Remembered[string] | undefined) => {
  const all = remembered();
  if (about === undefined) delete all[id];
  else all[id] = about;
  try {
    window.localStorage.setItem(REMEMBERED, JSON.stringify(all));
  } catch {
    // Without storage a reload names the Mac of a waiting send by nothing.
  }
};

/** The sentence for a refusal: Fleet's own message where it gave one, else the one for a Mac that did not answer. */
const said = (why: unknown, machine: Machine): string => bodyOf(why).message ?? refusal(why, machine);

/** The name the Gateway knows a Mac by: what it calls itself, or the first label of its address. */
export const nameOf = (machine: Machine): string =>
  machine.name !== "" ? machine.name : (new URL(machine.origin).hostname.split(".")[0] ?? "");

/** A Mac that does not answer: its stream or a read of its lists failed. */
export const asleep = (mac: Mac): boolean => mac.paired && mac.error !== undefined;

/** The Mac a send for a sleeping Mac goes through: the one serving this app if it answers, else the first paired Mac that does. */
export function receiverOf(macs: Mac[]): Mac | undefined {
  const answering = macs.filter((mac) => mac.paired && !asleep(mac));
  return (
    answering.find((mac) => mac.machine.origin === window.location.origin) ?? answering[0]
  );
}

const apply = (held: OperatorDispatched) => {
  const at = sends.find((one) => one.dispatchId === held.dispatch_id);
  if (at !== undefined) {
    change(at.id, {
      stage: {
        phase: "dispatched",
        ...held,
      },
    });
    return;
  }
  // A parked idea is named by its own id, not the dispatch's: it belongs to the one send still waiting on an answer.
  const waiting = sends.filter(
    (one) => one.stage.phase === "reading" && one.dispatchId !== undefined,
  );
  if (held.way === "parked" && waiting.length === 1) {
    change(waiting[0]!.id, {
      stage: {
        phase: "dispatched",
        ...held,
      },
    });
    return;
  }
  early.set(held.dispatch_id, held);
};

let made = 0;

/**
 * Sends to `chosen` through `receiver`: the same Mac where it answers, and no `mac` in the body; another one where `chosen` is
 * asleep, with `mac` naming it, so the send waits there. Answers the sentence where a Mac refused a send through another because
 * the repository is in no project they share (409): that send leaves no row, and its words go back into the composer.
 */
export async function dispatch(
  receiver: Mac,
  chosen: Mac,
  input: {
    text: string;
    repository: string;
    kind?: Kind;
  },
): Promise<string | undefined> {
  made += 1;
  const row: Send = {
    id: `n${made}`,
    at: new Date().toISOString(),
    mac: nameOf(chosen.machine),
    origin: chosen.machine.origin,
    repository: input.repository,
    words: input.text,
    ...(input.kind === undefined
      ? {}
      : {
          kind: input.kind,
        }),
    stage: {
      phase: "reading",
    },
  };
  set([
    row,
    ...sends,
  ]);
  const body: OperatorBody = {
    text: input.text,
    repository: input.repository,
    ...(input.kind === undefined
      ? {}
      : {
          kind: input.kind,
        }),
    ...(chosen === receiver
      ? {}
      : {
          mac: row.mac,
        }),
  };
  try {
    const response = await call(receiver.machine, "/api/sessions/dispatch", {
      method: "POST",
      body,
    });
    if (response.status === 202) {
      const queued = (await response.json()) as QueuedAct;
      remember(queued.id, {
        mac: row.mac,
        origin: chosen.machine.origin,
        repository: input.repository,
      });
      change(row.id, {
        stage: {
          phase: "queued",
          since: queued.since,
        },
        kept: {
          origin: receiver.machine.origin,
          id: queued.id,
        },
      });
      return;
    }
    const done = (await response.json()) as OperatorAnswer;
    if (done.session !== undefined)
      change(row.id, {
        stage: {
          phase: "dispatched",
          dispatch_id: row.id,
          manifest_id: "",
          way: "started",
          session_id: done.session.id,
          ...(done.session.title === undefined
            ? {}
            : {
                title: done.session.title,
              }),
        },
      });
    else if (done.operator !== undefined) {
      change(row.id, {
        dispatchId: done.operator,
      });
      const heard = early.get(done.operator);
      if (heard !== undefined) {
        early.delete(done.operator);
        apply(heard);
      }
    } else if (done.parked !== undefined)
      change(row.id, {
        stage: {
          phase: "dispatched",
          dispatch_id: done.parked,
          manifest_id: "",
          way: "parked",
        },
      });
    void loadWaiting(receiver.machine);
  } catch (why) {
    if (chosen !== receiver && why instanceof Refused && why.status === 409) {
      set(sends.filter((one) => one.id !== row.id));
      return said(why, receiver.machine);
    }
    change(row.id, {
      stage: {
        phase: "refused",
        why: said(why, receiver.machine),
      },
    });
  }
  return undefined;
}

const wordsOf = (queued: QueuedAct): string =>
  queued.act.body?.then?.find((one) => one.message !== undefined)?.message?.text ??
  queued.act.body?.request ??
  "";

/** Reads what waits on one Mac and brings the rows in line: a send that left the queue was sent, and one new to this page is drawn. */
export async function loadWaiting(machine: Machine): Promise<void> {
  let list: QueuedAct[];
  try {
    list = (await getJson<QueuedBody>(machine, "/api/queued")).queued.filter(
      (one) => one.act.kind === "dispatch",
    );
  } catch {
    return;
  }
  const waits = (one: Send) => one.kept?.origin === machine.origin;
  const left = sends.filter(
    (one) => waits(one) && !list.some((queued) => queued.id === one.kept!.id),
  );
  left.forEach((one) => remember(one.kept!.id, undefined));
  const known = remembered();
  const next: Send[] = sends
    .filter((one) => !left.includes(one))
    .map((one) => {
      const queued = list.find((that) => that.id === one.kept?.id);
      if (queued === undefined) return one;
      return {
        ...one,
        stage:
          queued.refused === undefined
            ? ({
                phase: "queued",
                since: queued.since,
              } as const)
            : ({
                phase: "refused",
                why: queued.refused,
              } as const),
      };
    });
  for (const queued of list) {
    if (next.some((one) => one.kept?.id === queued.id)) continue;
    const about = known[queued.id];
    next.push({
      id: `q${queued.id}`,
      at: new Date(queued.since).toISOString(),
      mac: about?.mac ?? "",
      ...(about === undefined
        ? {}
        : {
            origin: about.origin,
            repository: about.repository,
          }),
      words: wordsOf(queued),
      ...(queued.act.body?.kind === undefined
        ? {}
        : {
            kind: queued.act.body.kind,
          }),
      stage:
        queued.refused === undefined
          ? {
              phase: "queued",
              since: queued.since,
            }
          : {
              phase: "refused",
              why: queued.refused,
            },
      kept: {
        origin: machine.origin,
        id: queued.id,
      },
    });
  }
  // What left the queue went to its Mac, which says what it did with it on its own stream where the phone hears it.
  set([
    ...next,
    ...left.map((one) => {
      const { kept: _kept, ...sent } = one;
      return {
        ...sent,
        stage: {
          phase: "reading",
        } as const,
      };
    }),
  ]);
}

/**
 * Takes back what waits, or says why it cannot: Fleet's own sentence for one already gone or being sent. Answers the send as
 * it was, for its words to go back into the composer. A refused send that never left the phone has only its row to drop.
 */
export async function takeBack(
  send: Send,
): Promise<
  | {
      sent: Send;
    }
  | {
      said: string;
    }
> {
  const mac = send.kept === undefined ? undefined : macOf(send.kept.origin);
  if (send.kept === undefined || mac === undefined) {
    // A send the Gateway refused never left the phone: only its row goes.
    set(sends.filter((one) => one.id !== send.id));
    return {
      sent: send,
    };
  }
  try {
    await call(mac.machine, "/api/queued/take_back", {
      method: "POST",
      body: {
        id: send.kept.id,
      },
    });
  } catch (why) {
    return {
      said: said(why, mac.machine),
    };
  }
  remember(send.kept.id, undefined);
  set(sends.filter((one) => one.id !== send.id));
  return {
    sent: send,
  };
}

/** Follows the Operator on every Mac's stream, and reads what waits on each paired Mac. Answers how to stop. */
export function watchOperator(macs: Mac[]): () => void {
  macs.filter((mac) => mac.paired).forEach((mac) => void loadWaiting(mac.machine));
  return onOperator((origin, held) => {
    if (held !== undefined) apply(held);
    else {
      const mac = macOf(origin);
      if (mac !== undefined) void loadWaiting(mac.machine);
    }
  });
}
