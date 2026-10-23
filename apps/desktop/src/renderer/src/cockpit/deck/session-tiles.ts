// The flight controls while a Session is open on the stage. Each is an act Bridge already has for a Session, or it is drawn off
// with the fact that keeps it off: Ask for update, Auto land and Compact are messages to the Session through the route its
// composer sends by (`draft.send`), or its terminal's own (`draft.command`); Report hands off to a new Session started in
// Armada's own repository where Fleet serves it, else the reported one's (`report-brief.ts`, `ReportBox.tsx`), so it is off
// only where the reported Session's repository is not known; Map is the stage's switch. **What is offered is read from what
// Fleet holds** (whether the Session can be spoken to, which commands its agent listed), never from a report by the agent.
// The owner's ask of 15 Oct 2026. **Release** gives back everything the Session holds on the ledger, or hands it to another live
// Session of its repository, behind a confirmation that names each thing (`ReleaseHolds.tsx`); the owner's ask of 16 Oct 2026.

import type { RepositorySummary } from "@armada/protocol";
import { attachmentsOf, type Session, type SessionAttachment, type SessionsDraft } from "@armada/screens/src/draft/sessions";

export type SessionTileId = "ask" | "land" | "compact" | "report" | "map" | "judge" | "release" | "artifacts" | "subagents" | "checks" | "slot" | "requests" | "terminal" | "pause";

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

/**
 * In the order their keys, ⌥⇧1 to ⌥⇧9 and ⌥⇧0, press them; the last has none. `label` is the accessible name, `text` the word under the glyph.
 * The last four open a layer over the stage (`session-view/layers.ts`): Artifacts, Subagents and Checks always, Bays where the Session holds one, lit when a bay is wrong or its work clashes.
 */
export const SESSION_TILES: readonly { id: SessionTileId; label: string; text: string }[] = [
  { id: "ask", label: "Ask for update", text: "Update" },
  { id: "land", label: "Auto land", text: "Auto land" },
  { id: "compact", label: "Compact job context", text: "Compact" },
  { id: "report", label: "Report issue with job", text: "Report" },
  { id: "map", label: "Map", text: "Map" },
  { id: "judge", label: "Judge", text: "Judge" },
  { id: "release", label: "Release everything the Job holds", text: "Release" },
  { id: "artifacts", label: "Artifacts", text: "Artifacts" },
  { id: "subagents", label: "Subagents", text: "Subagents" },
  { id: "checks", label: "Checks", text: "Checks" },
  { id: "slot", label: "Bays", text: "Bays" },
  // Opens Requests in the side panel for this Job. Past the tenth, so no key.
  { id: "requests", label: "Requests", text: "Requests" },
  // The Job's shell, docked or in the side panel as `terminal.openJobTerminalIn` says (`job-shell-dock.tsx`). Moved here from the Job's header on the owner's ask of 23 Oct 2026.
  { id: "terminal", label: "Terminal", text: "Terminal" },
  // Pause reads Resume while the Session is paused (`SessionTiles.tsx`). Its key is p, not a digit.
  { id: "pause", label: "Pause", text: "Pause" },
];

/** The key that presses Pause, or Resume while paused, with a Session open or on the list's cursor row. Not an act of the registry: `p` is Pilot's there. */
export const PAUSE_KEY = "p";

/** The key of the tile at `at`: ⌥⇧1 to ⌥⇧9, then ⌥⇧0, and none for any after. A bare digit is a question's answer. */
export const tileKey = (at: number): string | undefined => (at < 9 ? `⌥⇧${at + 1}` : at === 9 ? "⌥⇧0" : undefined);

/** What a tile's corner cap draws: the digit alone for ⌥⇧ and a digit, which every tile shares and the tooltip spells out; any other key whole. */
export const tileCap = (key: string): string => key.replace(/^⌥⇧(\d)$/, "$1");

// Fleet turns on the forge's auto-merge and merges on a green `ci` (`draft.autoLand`); these words only tell the agent where it stands.
export const LAND_ON_TEXT = "Auto land is on. Fleet turns on auto-merge when your pull request opens and merges it once ci is green. Open the pull request. Fix ci failures and merge conflicts when you are told of them. Do not merge it yourself.";
export const LAND_OFF_TEXT = "Auto land is off. Fleet turns auto-merge off. Do not land on your own.";
export const COMPACT = "compact";

/** A message from the person with nothing attached, as the deck's Reply sends one. */
export const plain = (text: string) => ({ text, files: [], sketches: [], tags: [] });

/** Why the Session cannot be spoken to, or nothing where it can. */
export function mute(session: Session): string | undefined {
  if (session.dead === "ended") return "The Job ended";
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
export type Held = { kind: "slot" | "branch" | "pr"; target: string; name: string; said: string };

/** The ledger row an attachment is, as a release names it, or nothing where a release leaves it: a merged pull request is spent, and artifacts and links are not held. */
export function heldOfAttachment(one: SessionAttachment): Held | undefined {
  if (one.kind === "slot") return { kind: "slot", target: String(one.slot), name: `slot-${one.slot}`, said: "Slot" };
  if (one.kind === "branch") return { kind: "branch", target: one.name, name: one.name, said: "Branch" };
  if (one.kind === "pull_request" && one.state !== "merged") return { kind: "pr", target: String(one.number), name: `#${one.number} ${one.title}`.trim(), said: "Pull request" };
  return undefined;
}

/** A row's key across what Bridge sends and what Fleet answers. */
export const heldKey = (one: { kind: string; target: string }) => `${one.kind}:${one.target}`;

/** Everything a release acts on, by name, in the ledger's order: slots, branches, open pull requests. */
export function heldOf(session: Session): Held[] {
  const held: Held[] = [];
  for (const kind of ["slot", "branch", "pull_request"] as const) {
    for (const one of session.attachments) {
      const row = one.kind === kind ? heldOfAttachment(one) : undefined;
      if (row !== undefined) held.push(row);
    }
  }
  return held;
}

/** A row Fleet answered, named as `heldOf` named it, off the Session as it stood before the release. */
export function nameOfHeldRow(session: Session, row: { kind: string; target: string }): string {
  if (row.kind === "slot") return `slot-${row.target}`;
  if (row.kind === "pr") {
    const pr = session.attachments.find((one) => one.kind === "pull_request" && String(one.number) === row.target);
    return pr?.kind === "pull_request" ? `#${pr.number} ${pr.title}`.trim() : `#${row.target}`;
  }
  return row.target;
}

/** The Sessions a release may hand to: live, of the same repository, and not this one. */
export function handTargets(session: Session, all: readonly Session[]): Session[] {
  if (session.manifestId === undefined) return [];
  return all.filter((one) => one.id !== session.id && one.dead === undefined && one.manifestId === session.manifestId);
}

/** The confirm key: the act it sends, named. `all` is whether every row is ticked. */
export const releaseKeyOf = (to: Session | undefined, all = true): string => (to === undefined ? (all ? "Release all" : "Release") : `Hand to ${to.title ?? to.address ?? to.id}`);

/** `root` is the repository the Session works in, where Bridge knows it: a report starts its new Session there. A Session that is ended or quiet can still be reported. */
export function offeredOf(session: Session, draft: SessionsDraft, root: string | undefined, holdsBays = false): Offered {
  const off: Offered["off"] = root === undefined ? { report: "The repository is not known" } : {};
  // What the Session made or used can be read after it ended; the layers are drawn from what Bridge holds.
  if (attachmentsOf(session, "subagent").length === 0) off.subagents = "The Job has no subagents";
  if (!session.rows.some((one) => one.kind === "check")) off.checks = "The Job has run no Checks";
  if (!holdsBays) off.slot = "The Job holds no bay";
  // A judge reads the work against its plan, so it needs a plan to read and a way to be sent.
  if (draft.work?.judge === undefined || session.work?.plan === undefined) off.judge = "No plan to judge against";
  if (draft.releaseHolds === undefined) off.release = "Fleet does not offer this";
  else if (heldOf(session).length === 0) off.release = "The Job holds nothing";
  if (draft.pause === undefined) off.pause = "Fleet does not offer this";
  if (draft.autoLand === undefined) off.land = "Fleet does not offer this";
  if (draft.terminalRuns === undefined) off.terminal = "Fleet does not offer this";
  const silent = mute(session);
  if (silent !== undefined) {
    off.pause = silent;
    off.judge = silent;
    off.ask = silent;
    off.land = silent;
    off.compact = silent;
    off.terminal = silent;
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
