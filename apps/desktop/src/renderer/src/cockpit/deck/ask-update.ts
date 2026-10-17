// Asking a ring for an update: the one message a Session is sent, and what the button that sends it shows. It goes through the call the
// Session's composer sends with. The button reads sent until the next reply lands, or until `SENT_FOR_MS` where no reply can be seen, and
// a press that is already out or sent is not sent twice.

import { useEffect, useSyncExternalStore } from "react";
import type { SessionRow, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { useSessions, useSessionsDraft } from "../../sessions-draft";
import { ASK_FOR_UPDATE, askForUpdate } from "./acts";

/** The longest the button reads sent where no reply can be seen. */
export const SENT_FOR_MS = 5 * 60_000;

/** The route the message goes through: the Sessions draft's send. */
export type UpdateApi = {
  draft: SessionsDraft | undefined;
};

export type Asked = { ok: true } | { ok: false; said: string };

/** Sends the message to the Session a ring belongs to: `session:<id>`. Anything else is not built. */
export async function askRing(key: string, api: UpdateApi): Promise<Asked> {
  const [kind, id] = [key.slice(0, key.indexOf(":")), key.slice(key.indexOf(":") + 1)];
  if (kind === "session") return (await askForUpdate(api.draft, id)) ? { ok: true } : { ok: false, said: "Update not sent" };
  return { ok: false, said: "Not implemented" };
}

/** How many times the message is in a thread, sent by the person. */
export const askedIn = (rows: readonly SessionRow[]): number => rows.filter((row) => row.kind === "message" && row.from.kind === "you" && row.text === ASK_FOR_UPDATE).length;

/** The agent has spoken since the message was sent: it is in the thread more often than `before`, and a message of the agent's follows the last. */
export function repliedSince(rows: readonly SessionRow[], before: number): boolean {
  if (askedIn(rows) <= before) return false;
  const asks = rows.flatMap((row, at) => (row.kind === "message" && row.from.kind === "you" && row.text === ASK_FOR_UPDATE ? [at] : []));
  return rows.slice((asks.at(-1) ?? -1) + 1).some((row) => row.kind === "message" && row.from.kind === "agent");
}

// ---- What the button holds between presses ------------------------------------------------------------------

export type Held = { state: "sending" } | { state: "sent"; before: number } | { state: "refused"; said: string };

const held = new Map<string, Held>();
const timers = new Map<string, ReturnType<typeof setTimeout>>();
const listeners = new Set<() => void>();
const change = () => listeners.forEach((one) => one());
const subscribe = (on: () => void) => {
  listeners.add(on);
  return () => void listeners.delete(on);
};

/** Lets the button go: the reply came, or the time ran out. */
export function forget(key: string): void {
  clearTimeout(timers.get(key));
  timers.delete(key);
  if (held.delete(key)) change();
}

/** Presses for `key`. `before` is how often the thread held the message already. Resolves false where a press is out or sent, and sends nothing. */
export async function press(key: string, api: UpdateApi, before = 0): Promise<boolean> {
  const was = held.get(key);
  if (was !== undefined && was.state !== "refused") return false;
  held.set(key, { state: "sending" });
  change();
  const done = await askRing(key, api);
  if (!done.ok) {
    held.set(key, { state: "refused", said: done.said });
  } else {
    held.set(key, { state: "sent", before });
    timers.set(key, setTimeout(() => forget(key), SENT_FOR_MS));
  }
  change();
  return true;
}

/** What the button for `key` shows, and the press. A Session's reply is read off its thread, which clears the sent mark. */
export function useUpdateAsk(key: string): { held: Held | undefined; press: () => void } {
  const now = useSyncExternalStore(subscribe, () => held.get(key));
  const draft = useSessionsDraft();
  const rows = useSessions().find((one) => `session:${one.id}` === key)?.rows;
  useEffect(() => {
    if (now?.state === "sent" && rows !== undefined && repliedSince(rows, now.before)) forget(key);
  }, [now, rows, key]);
  return { held: now, press: () => void press(key, { draft }, rows === undefined ? 0 : askedIn(rows)) };
}
