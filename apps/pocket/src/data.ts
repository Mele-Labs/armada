// The one module the screens read their Sessions from: the Phone Gateway's lists, kept live
// from /api/live.

import { useEffect, useSyncExternalStore } from "react";

import { Refused, getJson, call } from "./client";
import type { AnswerBody, DispatchBody, LiveChange, NeedsBody, PhoneSession, RepositoriesBody, SessionsBody } from "./gateway";
import { events } from "./sse";

export function ago(from: string, now: number): string {
  const minutes = Math.max(0, Math.floor((now - Date.parse(from)) / 60_000));
  if (minutes < 1) return "now";
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  if (hours < 24) return rest === 0 ? `${hours}h` : `${hours}h ${String(rest).padStart(2, "0")}m`;
  const days = Math.floor(hours / 24);
  return hours % 24 === 0 ? `${days}d` : `${days}d ${hours % 24}h`;
}

type Lists = { needs: PhoneSession[]; live: PhoneSession[]; ended: PhoneSession[]; error?: string; at: number };

let lists: Lists = { needs: [], live: [], ended: [], at: Date.now() };
const watchers = new Set<() => void>();
const set = (next: Partial<Lists>) => {
  lists = { ...lists, ...next };
  watchers.forEach((watch) => watch());
};
const watching = (watch: () => void) => {
  watchers.add(watch);
  return () => watchers.delete(watch);
};

const message = (why: unknown) => (why instanceof Refused ? why.message : "Armada is not answering on your Mac. Open it there, then try again.");

export async function loadLists(): Promise<void> {
  try {
    const [needs, alive, ended] = await Promise.all([
      getJson<NeedsBody>("/api/needs"),
      getJson<SessionsBody>("/api/sessions"),
      getJson<SessionsBody>("/api/sessions?state=ended"),
    ]);
    set({ needs: needs.sessions, live: alive.sessions, ended: ended.sessions, error: undefined, at: Date.now() });
  } catch (why) {
    if (!(why instanceof Refused && why.status === 401)) set({ error: message(why) });
  }
}

let live = false;
let reload: ReturnType<typeof setTimeout> | undefined;
const changed = () => {
  clearTimeout(reload);
  reload = setTimeout(() => void loadLists(), 250);
};

/** Holds /api/live open for as long as the app runs, reading the lists again on every change. */
export async function startLive(): Promise<void> {
  if (live) return;
  live = true;
  void loadLists();
  for (;;) {
    try {
      const response = await call("/api/live");
      if (response.body === null) throw new Refused(0, "");
      void loadLists();
      for await (const text of events(response.body)) {
        const change = JSON.parse(text) as LiveChange;
        if (change.change === "resync") void loadLists();
        else changed();
      }
    } catch (why) {
      if (why instanceof Refused && why.status === 401) break;
      set({ error: message(why) });
    }
    await new Promise((resolve) => setTimeout(resolve, 3000));
  }
  live = false;
}

function useLists(): Lists {
  return useSyncExternalStore(watching, () => lists);
}

/** The ages tick once a minute. */
const useNow = (): number => {
  useEffect(() => {
    const tick = setInterval(() => set({ at: Date.now() }), 60_000);
    return () => clearInterval(tick);
  }, []);
  return useLists().at;
};

/** The Sessions waiting on the owner, the ones with a turn running, and the ones that are over. */
export function useSessions(): { needs: PhoneSession[]; running: PhoneSession[]; done: PhoneSession[]; error?: string } {
  useNow();
  const current = useLists();
  return {
    needs: current.needs,
    running: current.live.filter((session) => session.working && !session.waiting),
    done: current.ended,
    error: current.error,
  };
}

/** The act's sentence on a refusal is the Gateway's own; a 204 changes nothing here, the lists are read again. */
async function send(path: string, body: unknown): Promise<void> {
  await call(path, { method: "POST", body });
  await loadLists();
}

export const answer = (body: AnswerBody): Promise<void> => send("/api/sessions/answer", body);
export const dispatch = (body: DispatchBody): Promise<void> => send("/api/sessions", body);
export const repositories = (): Promise<RepositoriesBody> => getJson<RepositoriesBody>("/api/repositories");

export const refusal = (why: unknown): string => (why instanceof Refused ? why.message : "");

/** Drafts live outside the screens, so leaving one and coming back finds what was typed. */
export const DRAFTS = new Map<string, string>();

/** Runs an act. A send that succeeds clears its drafts and answers null; one that fails keeps them and answers the sentence to show. */
export async function attempt(act: () => Promise<void>, draftIds: string[] = []): Promise<string | null> {
  try {
    await act();
  } catch (why) {
    return refusal(why);
  }
  draftIds.forEach((id) => DRAFTS.delete(id));
  return null;
}
