// The flight controls while a Session is open on the stage. Each is an act Bridge already has for a Session, or it is drawn off
// with the fact that keeps it off: Ask for update, Auto land and Compact are messages to the Session through the route its
// composer sends by (`draft.send`), or its terminal's own (`draft.command`); Report hands off to a new Session started in
// Armada's own repository where Fleet serves it, else the reported one's (`report-brief.ts`, `ReportBox.tsx`), so it is off
// only where the reported Session's repository is not known; Map is the stage's switch. **What is offered is read from what
// Fleet holds** (whether the Session can be spoken to, which commands its agent listed), never from a report by the agent.
// The owner's ask of 15 Oct 2026. **Release** gives back everything the Session holds on the ledger, or hands it to another live
// Session of its repository, behind a confirmation that names each thing (`ReleaseHolds.tsx`); the owner's ask of 16 Oct 2026.

import type { RepositorySummary } from "@armada/protocol";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

export type SessionTileId = "ask" | "land" | "compact" | "report" | "map" | "release";

const ARMADA = "armada";

/**
 * Where a report's new Session starts: the Armada repository where Fleet serves one, since a fix to how a Session behaved is
 * usually Armada's, and the reported Session's own where it does not. Armada's is the Manifest named `armada`. The owner's ask
 * of 15 Oct 2026, from a machine serving several.
 */
export function reportRoot(repositories: readonly RepositorySummary[], reported: string | undefined): string | undefined {
  const armada = repositories.find((one) => one.manifest !== undefined && (one.manifest.id.toLowerCase() === ARMADA || one.manifest.repository.toLowerCase() === ARMADA));
  return armada?.root ?? reported;
}

/** In the order their keys, `1` to `5`, press them. `label` is the accessible name, `text` the word under the glyph. */
export const SESSION_TILES: readonly { id: SessionTileId; label: string; text: string }[] = [
  { id: "ask", label: "Ask for update", text: "Update" },
  { id: "land", label: "Auto land", text: "Auto land" },
  { id: "compact", label: "Compact session context", text: "Compact" },
  { id: "report", label: "Report issue with session", text: "Report" },
  { id: "map", label: "Map", text: "Map" },
  { id: "release", label: "Release everything the Session holds", text: "Release" },
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

/** One thing a Session holds on the ledger, as the release confirmation names it. `kind` is the ledger's. */
export type Held = { kind: "slot" | "branch" | "pr" | "job" | "need"; name: string; said: string };

/**
 * Everything a release acts on, by name, in the ledger's order: slots, branches, open pull requests, Jobs, needs. A merged pull request
 * is spent and a release leaves it, so it is not named.
 */
export function heldOf(session: Session): Held[] {
  const held: Held[] = [];
  for (const kind of ["slot", "branch", "pull_request", "job"] as const) {
    for (const one of session.attachments) {
      if (one.kind !== kind) continue;
      if (one.kind === "slot") held.push({ kind: "slot", name: `slot-${one.slot}`, said: one.handed === undefined ? "Slot" : `Slot of piloted Job #${one.handed.job}` });
      else if (one.kind === "branch") held.push({ kind: "branch", name: one.name, said: "Branch" });
      else if (one.kind === "pull_request" && one.state !== "merged") held.push({ kind: "pr", name: `#${one.number} ${one.title}`.trim(), said: "Pull request" });
      else if (one.kind === "job") held.push({ kind: "job", name: `#${one.number} ${one.title}`, said: "Job" });
    }
  }
  for (const need of session.needs ?? []) held.push({ kind: "need", name: need.path, said: need.what === "" ? "Need" : `Need: ${need.what}` });
  return held;
}

/** A row Fleet answered, named as `heldOf` named it, off the Session as it stood before the release. */
export function nameOfHeldRow(session: Session, row: { kind: string; target: string }): string {
  if (row.kind === "slot") return `slot-${row.target}`;
  if (row.kind === "pr") {
    const pr = session.attachments.find((one) => one.kind === "pull_request" && String(one.number) === row.target);
    return pr?.kind === "pull_request" ? `#${pr.number} ${pr.title}`.trim() : `#${row.target}`;
  }
  if (row.kind === "job") {
    const job = session.attachments.find((one) => one.kind === "job" && one.id === row.target);
    return job?.kind === "job" ? `#${job.number} ${job.title}` : row.target;
  }
  return row.target;
}

/** The Sessions a release may hand to: live, of the same repository, and not this one. */
export function handTargets(session: Session, all: readonly Session[]): Session[] {
  if (session.manifestId === undefined) return [];
  return all.filter((one) => one.id !== session.id && one.dead === undefined && one.manifestId === session.manifestId);
}

/** The confirm key: the act it sends, named. */
export const releaseKeyOf = (to: Session | undefined): string => (to === undefined ? "Release all" : `Hand to ${to.title ?? to.address ?? to.id}`);

/** `root` is the repository the Session works in, where Bridge knows it: a report starts its new Session there. A Session that is ended or quiet can still be reported. */
export function offeredOf(session: Session, draft: SessionsDraft, root: string | undefined): Offered {
  const off: Offered["off"] = root === undefined ? { report: "The repository is not known" } : {};
  if (draft.releaseHolds === undefined) off.release = "Fleet does not offer this";
  else if (heldOf(session).length === 0) off.release = "The Session holds nothing";
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
