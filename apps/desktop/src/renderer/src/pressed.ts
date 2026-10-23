// A control pressed on a Session, drawn as pressed from the moment of the press and until Fleet's own record carries it.
//
// **One engine, laid over whatever serves Sessions**, as `outgoing.ts` lays a sent message over a thread. `pressed()` holds
// what was expected of each Session (`paused`, `autoLand`, the plan approved) and patches it onto the Session's row;
// `withPressed` is the layer over a draft, so Pause, Resume, Auto land and Approve change the screen at the press, with
// `Session.pressed` naming the controls still going, and the going mark goes when the answer lands. An answer that is a
// refusal, or a delivery that throws, drops the patch, and the control reverts to what Fleet holds; the refusal's words
// are the draft's to say, as they are today. The patch also goes, silently, once the row carries what was expected.

import type { PressedKey, Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

type Standing = "going" | "settled" | "dropped";

type Answer = void | boolean | Promise<void | boolean>;

type Entry = { key: PressedKey; want: boolean; state: Standing };

/** Whether Fleet's own row already says what was pressed for. */
const carries = (session: Session, entry: Entry): boolean => {
  if (entry.key === "paused") return (session.paused === true) === entry.want;
  if (entry.key === "autoLand") return (session.autoLand === true) === entry.want;
  return session.work?.plan?.approved === true;
};

/** The row with one expectation laid on it. */
const patched = (session: Session, entry: Entry): Session => {
  if (entry.key === "paused") {
    const { paused: _, ...rest } = session;
    return entry.want ? { ...rest, paused: true } : rest;
  }
  if (entry.key === "autoLand") {
    const { autoLand: _, ...rest } = session;
    return entry.want ? { ...rest, autoLand: true } : rest;
  }
  return session.work?.plan === undefined ? session : { ...session, work: { ...session.work, plan: { ...session.work.plan, approved: true } } };
};

export type Pressed = {
  /** Expects `key` to read `want` on Session `id` from now, and delivers the press: resolves to whether Fleet took it. */
  press: (id: string, key: PressedKey, want: boolean, how: () => Answer) => Promise<boolean>;
  /** `session` with what is expected of it on top, and `pressed` naming what is still going. Itself where nothing is. */
  overlay: (session: Session) => Session;
  has: (id: string) => boolean;
  /** Changes whenever what is held for `id` does. */
  version: (id: string) => number;
  /** Changes whenever anything held does. */
  total: () => number;
  subscribe: (listener: () => void) => () => void;
};

export function pressed(): Pressed {
  const entries = new Map<string, Entry[]>();
  const versions = new Map<string, number>();
  const listeners = new Set<() => void>();
  let total = 0;

  const changed = (id: string): void => {
    versions.set(id, (versions.get(id) ?? 0) + 1);
    total += 1;
    listeners.forEach((one) => one());
  };

  const live = (id: string): Entry[] => (entries.get(id) ?? []).filter((one) => one.state !== "dropped");

  return {
    press: (id, key, want, how) => {
      // A second press on the same control replaces the first: the later expectation is the one drawn.
      for (const one of live(id)) if (one.key === key) one.state = "dropped";
      const entry: Entry = { key, want, state: "going" };
      entries.set(id, [...live(id), entry]);
      changed(id);
      let answer: Answer;
      try {
        answer = how();
      } catch {
        answer = false;
      }
      return Promise.resolve(answer).then(
        (went) => {
          if (entry.state === "dropped") return went !== false;
          entry.state = went === false ? "dropped" : "settled";
          changed(id);
          return went !== false;
        },
        () => {
          if (entry.state !== "dropped") {
            entry.state = "dropped";
            changed(id);
          }
          return false;
        },
      );
    },
    overlay: (session) => {
      const mine = live(session.id);
      if (mine.length === 0) return session;
      let out = session;
      const going: PressedKey[] = [];
      for (const entry of mine) {
        // Fleet's row says it already: nothing left to lay over. Marked here, not announced, since this runs inside a read.
        if (entry.state === "settled" && carries(session, entry)) {
          entry.state = "dropped";
          continue;
        }
        out = patched(out, entry);
        if (entry.state === "going") going.push(entry.key);
      }
      if (out === session) return session;
      return going.length === 0 ? out : { ...out, pressed: going };
    },
    has: (id) => live(id).length > 0,
    version: (id) => versions.get(id) ?? 0,
    total: () => total,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => void listeners.delete(listener);
    },
  };
}

export function withPressed(draft: SessionsDraft): SessionsDraft {
  const held = pressed();
  const cached = new Map<string, { session: Session; version: number; out: Session }>();
  let last: { base: readonly Session[]; version: number; out: readonly Session[] } | undefined;
  let touched = false;

  const overlay = (session: Session): Session => {
    if (!held.has(session.id)) return session;
    const have = held.version(session.id);
    const was = cached.get(session.id);
    if (was !== undefined && was.session === session && was.version === have) return was.out;
    const out = held.overlay(session);
    cached.set(session.id, { session, version: have, out });
    return out;
  };

  const wrapped = Object.create(draft) as SessionsDraft;
  return Object.assign(wrapped, {
    get: (): readonly Session[] => {
      const base = draft.get();
      if (!touched) return base;
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
    ...(draft.pause === undefined
      ? {}
      : {
          pause: (id: string): Promise<boolean> => {
            touched = true;
            return held.press(id, "paused", true, () => draft.pause!(id));
          },
        }),
    ...(draft.resume === undefined
      ? {}
      : {
          resume: (id: string): Promise<boolean> => {
            touched = true;
            return held.press(id, "paused", false, () => draft.resume!(id));
          },
        }),
    ...(draft.autoLand === undefined
      ? {}
      : {
          autoLand: (id: string, on: boolean): Promise<boolean> => {
            touched = true;
            return held.press(id, "autoLand", on, () => draft.autoLand!(id, on));
          },
        }),
    ...(draft.work === undefined
      ? {}
      : {
          work: {
            ...draft.work,
            approve: (id: string): void => {
              touched = true;
              void held.press(id, "plan", true, () => draft.work!.approve(id));
            },
          },
        }),
  });
}
