// A message of the person's, drawn in the thread from the moment it is sent and until Fleet's own row takes its place.
//
// **One engine, two ways in.** `outgoing()` holds the messages and lays them over a thread's rows. `withOutgoing` is the
// layer over whatever serves Sessions, a real Fleet or the mock, so both draw it the same way; Helm's box holds its words in
// the same engine (`helm-box.tsx`), over a thread no Session list serves. `send` holds the message here and hands it on;
// the thread shows it at once with a mark that it is going (`delivery: "sending"`), and the mark goes when Fleet takes it. A
// message Fleet did not take stays in the thread, marked, with a Retry. When the row Fleet sends back arrives it is the same
// message: it is drawn in the optimistic row's place, under the same key, so nothing draws again and nothing is there twice.
// Fleet carries no id of ours, so the row is found by its words, and by being new.

import type { Session, SessionRow, SessionsDraft, SentMessage } from "@armada/screens/src/draft/sessions";

type Standing = "sending" | "sent" | "failed" | "echoed" | "dropped";

type Delivery = () => void | boolean | Promise<boolean>;

type Entry = {
  key: string;
  text: string;
  sent: SentMessage;
  at: string;
  state: Standing;
  /** The rows the thread held when it was sent, so a row that was already there is never taken for its echo. */
  known: ReadonlySet<string>;
  /** Fleet's own row, once it has arrived. */
  rowId?: string;
  /** How it goes, so a Retry sends it the way it was first sent. */
  deliver?: Delivery;
};

type Message = Extract<SessionRow, { kind: "message" | "sent" }>;

/** Words compared as Fleet may have trimmed them. */
const same = (a: string, b: string): boolean => a.trim().replace(/\s+/g, " ") === b.trim().replace(/\s+/g, " ");

/** The person's words as Fleet keeps them: their message, or in Helm's thread the row for words sent on to other work. */
const youSaid = (row: SessionRow): row is Message => (row.kind === "message" && row.from.kind === "you") || row.kind === "sent";

/** Words held in a thread while they go. */
export type Held = {
  /** Delivers the words and settles them by whether Fleet took them. Delivering again after a refusal sends them again. */
  deliver: (how: Delivery) => Promise<boolean>;
  /** The words went nowhere: out of the thread. */
  drop: () => void;
};

export type Outgoing = {
  /** Puts the words in thread `id` now, going. `known` is the rows it holds, so none of them is taken for the words' echo. */
  hold: (id: string, sent: SentMessage, known: ReadonlySet<string>) => Held;
  /** `rows` with what is held for `id` on top: Fleet's row where one has arrived, and the words where none has. */
  overlay: (id: string, rows: readonly SessionRow[]) => readonly SessionRow[];
  /** Sends a message that did not go again, the way it was first sent. */
  retry: (id: string, rowId: string, known: ReadonlySet<string>) => void;
  has: (id: string) => boolean;
  /** Changes whenever what is held for `id` does. */
  version: (id: string) => number;
  /** Changes whenever anything held does. */
  total: () => number;
  subscribe: (listener: () => void) => () => void;
};

export function outgoing(): Outgoing {
  const entries = new Map<string, Entry[]>();
  const versions = new Map<string, number>();
  const listeners = new Set<() => void>();
  const cached = new Map<string, { rows: readonly SessionRow[]; version: number; out: readonly SessionRow[] }>();
  let count = 0;
  let total = 0;

  const changed = (id: string): void => {
    versions.set(id, (versions.get(id) ?? 0) + 1);
    total += 1;
    listeners.forEach((one) => one());
  };

  const overlay = (id: string, base: readonly SessionRow[]): readonly SessionRow[] => {
    const mine = entries.get(id)?.filter((one) => one.state !== "dropped");
    if (mine === undefined || mine.length === 0) return base;
    const have = versions.get(id) ?? 0;
    const was = cached.get(id);
    if (was !== undefined && was.rows === base && was.version === have) return was.out;
    const taken = new Set(mine.flatMap((one) => (one.rowId === undefined ? [] : [one.rowId])));
    for (const entry of mine) {
      if (entry.state === "echoed" || entry.state === "failed") continue;
      const fresh = base.filter((row): row is Message => youSaid(row) && !entry.known.has(row.id) && !taken.has(row.id));
      // Its words first; failing that, a Fleet that rewrote them and said so with a new row of the person's.
      const found = fresh.find((row) => same(row.text, entry.text)) ?? (entry.state === "sent" ? fresh[0] : undefined);
      if (found === undefined) continue;
      entry.state = "echoed";
      entry.rowId = found.id;
      taken.add(found.id);
    }
    const keyed = new Map(mine.flatMap((one) => (one.rowId === undefined ? [] : [[one.rowId, one.key] as const])));
    const rows: SessionRow[] = base.map((row) => (keyed.has(row.id) && (row.kind === "message" || row.kind === "sent") ? { ...row, stableKey: keyed.get(row.id)! } : row));
    for (const entry of mine) {
      if (entry.state === "echoed") continue;
      rows.push({
        id: entry.key,
        at: entry.at,
        kind: "message",
        from: { kind: "you" },
        text: entry.text,
        ...(entry.sent.files.length === 0 ? {} : { files: entry.sent.files.map(({ id, name }) => ({ id, name })) }),
        ...(entry.sent.sketches.length === 0 ? {} : { sketches: entry.sent.sketches.map(({ id, title }) => ({ id, title })) }),
        ...(entry.sent.tags.length === 0 ? {} : { tags: entry.sent.tags }),
        ...(entry.state === "sending" ? { delivery: "sending" as const } : entry.state === "failed" ? { delivery: "failed" as const } : {}),
      });
    }
    cached.set(id, { rows: base, version: have, out: rows });
    return rows;
  };

  const settle = (id: string, entry: Entry, answer: void | boolean): boolean => {
    const went = answer !== false;
    // Fleet's own row may have come first, and then there is nothing left to say about the message.
    if (entry.state !== "echoed" && entry.state !== "dropped") {
      entry.state = went ? "sent" : "failed";
      changed(id);
    }
    return went;
  };

  /** Hands the message on and settles it by the answer: to whether Fleet took it. */
  const dispatch = (id: string, entry: Entry, how: Delivery): Promise<boolean> => {
    entry.deliver = how;
    let answer: void | boolean | Promise<boolean>;
    try {
      answer = how();
    } catch {
      answer = false;
    }
    return Promise.resolve(answer).then(
      (went) => settle(id, entry, went),
      () => settle(id, entry, false),
    );
  };

  /** Back to going: the same words, in the thread as it is now. */
  const again = (id: string, entry: Entry, known: ReadonlySet<string>, how: Delivery): Promise<boolean> => {
    if (entry.state !== "sending") {
      entry.state = "sending";
      entry.known = known;
      changed(id);
    }
    return dispatch(id, entry, how);
  };

  return {
    hold: (id, sent, known) => {
      const now = new Date().toISOString();
      count += 1;
      const entry: Entry = { key: `sending:${count}`, text: sent.text, sent, at: now, state: "sending", known };
      entries.set(id, [...(entries.get(id) ?? []), entry]);
      changed(id);
      return {
        deliver: (how) => (entry.state === "dropped" ? Promise.resolve(false) : again(id, entry, entry.known, how)),
        drop: () => {
          if (entry.state === "dropped" || entry.state === "echoed") return;
          entry.state = "dropped";
          changed(id);
        },
      };
    },
    overlay,
    retry: (id, rowId, known) => {
      const entry = entries.get(id)?.find((one) => one.key === rowId);
      if (entry === undefined || entry.state !== "failed" || entry.deliver === undefined) return;
      void again(id, entry, known, entry.deliver);
    },
    has: (id) => entries.get(id)?.some((one) => one.state !== "dropped") === true,
    version: (id) => versions.get(id) ?? 0,
    total: () => total,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => void listeners.delete(listener);
    },
  };
}

export function withOutgoing(draft: SessionsDraft): SessionsDraft {
  const held = outgoing();
  const cached = new Map<string, { session: Session; version: number; out: Session }>();
  let last: { base: readonly Session[]; version: number; out: readonly Session[] } | undefined;
  let sent = false;

  const rowsOf = (id: string): ReadonlySet<string> => new Set(draft.get().find((one) => one.id === id)?.rows.map((one) => one.id));

  /** The Session with what it holds of its own on top: Fleet's row where one has arrived, and the message where none has. */
  const overlay = (session: Session): Session => {
    if (!held.has(session.id)) return session;
    const have = held.version(session.id);
    const was = cached.get(session.id);
    if (was !== undefined && was.session === session && was.version === have) return was.out;
    const out = { ...session, rows: [...held.overlay(session.id, session.rows)] };
    cached.set(session.id, { session, version: have, out });
    return out;
  };

  const wrapped = Object.create(draft) as SessionsDraft;
  return Object.assign(wrapped, {
    get: (): readonly Session[] => {
      const base = draft.get();
      if (!sent) return base;
      if (last !== undefined && last.base === base && last.version === held.total()) return last.out;
      const out = base.map(overlay);
      // Nothing of ours changed what is drawn: the same list, so what reads it does not draw again.
      const kept = last !== undefined && last.out.length === out.length && out.every((one, at) => one === last!.out[at]);
      last = { base, version: held.total(), out: kept ? last!.out : out };
      return last.out;
    },
    subscribe: (listener: () => void): (() => void) => {
      const own = held.subscribe(listener);
      const off = draft.subscribe(listener);
      return () => {
        own();
        off();
      };
    },
    send: (id: string, message: SentMessage): Promise<boolean> => {
      sent = true;
      // To whether Fleet took it, as `send` answers. The message box does not wait on that: it is in the thread already.
      return held.hold(id, message, rowsOf(id)).deliver(() => draft.send(id, message));
    },
    retry: (id: string, rowId: string): void => held.retry(id, rowId, rowsOf(id)),
  });
}
