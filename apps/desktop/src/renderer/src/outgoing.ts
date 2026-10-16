// A message of the person's, drawn in the thread from the moment it is sent and until Fleet's own row takes its place.
//
// **One layer over whatever serves Sessions**, a real Fleet or the mock, so both draw it the same way. `send` holds the
// message here and hands it on; the thread shows it at once with a mark that it is going (`delivery: "sending"`), and the
// mark goes when Fleet takes it. A message Fleet did not take stays in the thread, marked, with a Retry. When the row Fleet
// sends back arrives it is the same message: it is drawn in the optimistic row's place, under the same key, so nothing draws
// again and nothing is there twice. Fleet carries no id of ours, so the row is found by its words, and by being new.

import { clock } from "@armada/screens/src/duration";
import type { Session, SessionRow, SessionsDraft, SentMessage } from "@armada/screens/src/draft/sessions";

type Standing = "sending" | "sent" | "failed" | "echoed";

type Entry = {
  key: string;
  text: string;
  sent: SentMessage;
  at: string;
  instant: string;
  state: Standing;
  /** The rows the thread held when it was sent, so a row that was already there is never taken for its echo. */
  known: ReadonlySet<string>;
  /** Fleet's own row, once it has arrived. */
  rowId?: string;
};

type Message = Extract<SessionRow, { kind: "message" }>;

/** Words compared as Fleet may have trimmed them. */
const same = (a: string, b: string): boolean => a.trim().replace(/\s+/g, " ") === b.trim().replace(/\s+/g, " ");

const youSaid = (row: SessionRow): row is Message => row.kind === "message" && row.from.kind === "you";

export function withOutgoing(draft: SessionsDraft): SessionsDraft {
  const entries = new Map<string, Entry[]>();
  const versions = new Map<string, number>();
  const listeners = new Set<() => void>();
  const cached = new Map<string, { session: Session; version: number; out: Session }>();
  let count = 0;
  let last: { base: readonly Session[]; version: number; out: readonly Session[] } | undefined;
  let version = 0;

  const changed = (id: string): void => {
    versions.set(id, (versions.get(id) ?? 0) + 1);
    version += 1;
    listeners.forEach((one) => one());
  };

  const rowsOf = (id: string): ReadonlySet<string> => new Set(draft.get().find((one) => one.id === id)?.rows.map((one) => one.id));

  /** The Session with what it holds of its own on top: Fleet's row where one has arrived, and the message where none has. */
  const overlay = (session: Session): Session => {
    const mine = entries.get(session.id);
    if (mine === undefined) return session;
    const have = versions.get(session.id) ?? 0;
    const was = cached.get(session.id);
    if (was !== undefined && was.session === session && was.version === have) return was.out;
    const taken = new Set(mine.flatMap((one) => (one.rowId === undefined ? [] : [one.rowId])));
    for (const entry of mine) {
      if (entry.state === "echoed" || entry.state === "failed") continue;
      const fresh = session.rows.filter((row): row is Message => youSaid(row) && !entry.known.has(row.id) && !taken.has(row.id));
      // Its words first; failing that, a Fleet that rewrote them and said so with a new row of the person's.
      const found = fresh.find((row) => same(row.text, entry.text)) ?? (entry.state === "sent" ? fresh[0] : undefined);
      if (found === undefined) continue;
      entry.state = "echoed";
      entry.rowId = found.id;
      taken.add(found.id);
    }
    const keyed = new Map(mine.flatMap((one) => (one.rowId === undefined ? [] : [[one.rowId, one.key] as const])));
    const rows: SessionRow[] = session.rows.map((row) => (keyed.has(row.id) && row.kind === "message" ? { ...row, stableKey: keyed.get(row.id)! } : row));
    for (const entry of mine) {
      if (entry.state === "echoed") continue;
      rows.push({
        id: entry.key,
        at: entry.at,
        kind: "message",
        instant: entry.instant,
        from: { kind: "you" },
        text: entry.text,
        ...(entry.sent.files.length === 0 ? {} : { files: entry.sent.files.map(({ id, name }) => ({ id, name })) }),
        ...(entry.sent.sketches.length === 0 ? {} : { sketches: entry.sent.sketches.map(({ id, title }) => ({ id, title })) }),
        ...(entry.sent.tags.length === 0 ? {} : { tags: entry.sent.tags }),
        ...(entry.state === "sending" ? { delivery: "sending" as const } : entry.state === "failed" ? { delivery: "failed" as const } : {}),
      });
    }
    const out = { ...session, rows };
    cached.set(session.id, { session, version: have, out });
    return out;
  };

  const settle = (id: string, entry: Entry, answer: void | boolean): boolean => {
    const went = answer !== false;
    // Fleet's own row may have come first, and then there is nothing left to say about the message.
    if (entry.state !== "echoed") {
      entry.state = went ? "sent" : "failed";
      changed(id);
    }
    return went;
  };

  /** Hands the message on and settles it by the answer: to whether Fleet took it. */
  const dispatch = (id: string, entry: Entry): Promise<boolean> => {
    let answer: void | boolean | Promise<boolean>;
    try {
      answer = draft.send(id, entry.sent);
    } catch {
      answer = false;
    }
    return Promise.resolve(answer).then(
      (went) => settle(id, entry, went),
      () => settle(id, entry, false),
    );
  };

  const wrapped = Object.create(draft) as SessionsDraft;
  return Object.assign(wrapped, {
    get: (): readonly Session[] => {
      const base = draft.get();
      if (entries.size === 0) return base;
      if (last !== undefined && last.base === base && last.version === version) return last.out;
      const out = base.map(overlay);
      // Nothing of ours changed what is drawn: the same list, so what reads it does not draw again.
      const kept = last !== undefined && last.out.length === out.length && out.every((one, at) => one === last!.out[at]);
      last = { base, version, out: kept ? last!.out : out };
      return last.out;
    },
    subscribe: (listener: () => void): (() => void) => {
      listeners.add(listener);
      const off = draft.subscribe(listener);
      return () => {
        listeners.delete(listener);
        off();
      };
    },
    send: (id: string, sent: SentMessage): Promise<boolean> => {
      const now = new Date().toISOString();
      count += 1;
      const entry: Entry = { key: `sending:${count}`, text: sent.text, sent, at: clock(now), instant: now, state: "sending", known: rowsOf(id) };
      entries.set(id, [...(entries.get(id) ?? []), entry]);
      changed(id);
      // To whether Fleet took it, as `send` answers. The message box does not wait on that: it is in the thread already.
      return dispatch(id, entry);
    },
    retry: (id: string, rowId: string): void => {
      const entry = entries.get(id)?.find((one) => one.key === rowId);
      if (entry === undefined || entry.state !== "failed") return;
      entry.state = "sending";
      entry.known = rowsOf(id);
      changed(id);
      void dispatch(id, entry);
    },
  });
}
