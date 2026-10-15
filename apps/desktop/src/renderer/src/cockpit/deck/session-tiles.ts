// The flight controls while a Session is open on the stage. Each is an act Bridge already has for a Session, or it is drawn off
// with the fact that keeps it off: Ask for update, Auto land and Compact are messages to the Session through the route its
// composer sends by (`draft.send`), or its terminal's own (`draft.command`); Report has no route for a Session; Map is the
// stage's switch. **What is offered is read from what Fleet holds** (whether the Session can be spoken to, which commands its
// agent listed), never from a report by the agent. The owner's ask of 15 Oct 2026.

import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

export type SessionTileId = "ask" | "land" | "compact" | "report" | "map";

/** In the order their keys, `1` to `5`, press them. `label` is the accessible name, `text` the word under the glyph. */
export const SESSION_TILES: readonly { id: SessionTileId; label: string; text: string }[] = [
  { id: "ask", label: "Ask for update", text: "Update" },
  { id: "land", label: "Auto land", text: "Auto land" },
  { id: "compact", label: "Compact session context", text: "Compact" },
  { id: "report", label: "Report issue with session", text: "Report" },
  { id: "map", label: "Map", text: "Map" },
];

export const LAND_ON_TEXT = "Auto land is on. When your work is done, land it: open the pull request, merge main in, turn on auto-merge, watch ci, and fix failures and conflicts until it is in main.";
export const LAND_OFF_TEXT = "Auto land is off. Do not land on your own.";
export const COMPACT = "compact";

/** A message from the person with nothing attached, as the deck's Reply sends one. */
export const plain = (text: string) => ({ text, files: [], sketches: [], tags: [] });

/** Why the Session cannot be spoken to, or nothing where it can. */
export function mute(session: Session): string | undefined {
  if (session.dead === "ended") return "The Session ended";
  if (session.dead === "quiet") return "The terminal is not listening";
  return undefined;
}

/** How Compact reaches the Session: typed in its terminal, or sent as a message to the agent Bridge hosts. */
export type CompactRoute = { how: "terminal" } | { how: "message" };

export type Offered = {
  /** Why each is off, where it is. */
  off: Partial<Record<SessionTileId, string>>;
  compact?: CompactRoute;
};

export function offeredOf(session: Session, draft: SessionsDraft): Offered {
  const off: Offered["off"] = { report: "Reports are filed on a Job" };
  const silent = mute(session);
  if (silent !== undefined) {
    off.ask = silent;
    off.land = silent;
    off.compact = silent;
    return { off };
  }
  if (session.terminal === true) {
    if (draft.command !== undefined && (session.commands ?? []).some((one) => one.name === COMPACT)) return { off, compact: { how: "terminal" } };
    off.compact = "The terminal has not listed compact";
    return { off };
  }
  if (draft.commands.some((one) => one.name === COMPACT)) return { off, compact: { how: "message" } };
  off.compact = "The agent has not listed compact";
  return { off };
}
