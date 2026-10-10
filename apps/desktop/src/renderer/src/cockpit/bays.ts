// The Cockpit's third reading: each worktree bay as a row, with whoever holds it (a Job, or a Session
// working in it directly) and the Sessions riding with that Job, and the pull request its branch has on
// the merge line, matched by branch. The line itself is the horizon's own dots stood upright, so the
// two readings cannot disagree. Pure, so which bay holds what is checked without drawing a row.

import type { JobSummary, MergeLines, WorktreeSlot } from "@armada/protocol";
import type { MergeLineView } from "@armada/screens";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Item } from "../Dashboard";
import { dotsOf, type Dot } from "./horizon";

/** Who is in a bay, or why nobody is. */
export type Holder =
  | { kind: "job"; id: string; title: string; item: Item | undefined }
  | { kind: "session"; title: string; session: Session | undefined; item: Item | undefined }
  | { kind: "free" }
  | { kind: "closed" }
  /** Unmade, busy, not a checkout, or stranded: what the pool says, in its words. */
  | { kind: "other"; said: string };

export type Bay = {
  key: string;
  slot: number;
  branch: string | undefined;
  warm: boolean;
  behind: number | undefined;
  since: string | undefined;
  holder: Holder;
  /** Sessions working on the Job in the bay, each with its tile where it has one. */
  riders: readonly { session: Session; item: Item | undefined }[];
  /** The bay's branch on the line, where it has a pull request or a landing there. */
  dot: Dot | undefined;
};

/** One repository's bays and its line, top of the line farthest from main. */
export type Harbour = {
  manifest: string;
  /** The repository's label, drawn only where more than one repository has bays. */
  name: string;
  bays: readonly Bay[];
  line: readonly Dot[];
  main: "green" | "red" | undefined;
};

export type BaysRead = {
  harbours: readonly Harbour[];
  /** Live work no bay holds: a Job waiting for one, a Session with none of its own. */
  unbayed: readonly Item[];
};

/** What the pool says of a slot nobody can lease, as Cleanup's tile words it. */
const OTHER: Record<string, string> = { unmade: "Not made yet", busy: "Changing hands", not_a_checkout: "Not a checkout" };

/** The Session a slot is held by: the pool names a process, so a Session is found by its own slot or branch. */
function sessionIn(slot: WorktreeSlot, sessions: readonly Session[]): Session | undefined {
  return sessions.find(
    (one) =>
      one.dead === undefined &&
      one.attachments.some((attachment) => (attachment.kind === "slot" && attachment.slot === slot.slot) || (attachment.kind === "branch" && (attachment.slot === slot.slot || attachment.name === slot.branch))),
  );
}

/** The tile a Job or a Session already has on the glass, so a bay lights and opens as that tile does. */
const tileOf = (items: readonly Item[], owner: string) => items.find((one) => ownerOf(one) === owner);

/** Whose a tile is: a call names its Job or Session as owner, a running Job's own tile is keyed by the Job. */
const ownerOf = (item: Item) => item.owner ?? item.key;

export function baysOf({
  slots,
  jobs,
  sessions,
  items,
  ended,
  views,
  lines,
  manifestOf,
  nameOf,
}: {
  slots: readonly WorktreeSlot[];
  jobs: readonly JobSummary[];
  sessions: readonly Session[];
  /** Every tile the Cockpit holds, calls first, so a Job that asks is found by its call. */
  items: readonly Item[];
  /** The tiles of what is over: found in a bay, never listed as waiting for one. */
  ended: readonly Item[];
  views: readonly MergeLineView[];
  lines: MergeLines | null;
  /** A merge line's root to the Manifest its repository is served under. */
  manifestOf: (root: string) => string | undefined;
  nameOf: (manifest: string) => string;
}): BaysRead {
  const live = sessions.filter((one) => one.dead === undefined);
  const manifests = [...new Set(slots.map((one) => one.manifest_id))];
  const seen = new Set<string>();
  const harbours = manifests.map((manifest): Harbour => {
    const view = views.find((one) => manifestOf(one.root) === manifest);
    const line = view === undefined ? [] : [...dotsOf(view, lines, views, live)].reverse();
    const bays = slots
      .filter((one) => one.manifest_id === manifest)
      .sort((a, b) => a.slot - b.slot)
      .map((slot): Bay => {
        const holder = holderOf(slot, jobs, live, items);
        if (holder.kind === "job") seen.add(holder.id);
        if (holder.kind === "session" && holder.session !== undefined) seen.add(`session:${holder.session.id}`);
        const riders =
          holder.kind === "job"
            ? live.filter((one) => one.attachments.some((attachment) => attachment.kind === "job" && attachment.id === holder.id)).map((session) => ({ session, item: tileOf(items, `session:${session.id}`) }))
            : [];
        riders.forEach((one) => seen.add(`session:${one.session.id}`));
        return {
          key: `${manifest}:${slot.slot}`,
          slot: slot.slot,
          branch: slot.branch,
          warm: slot.warm,
          behind: slot.behind,
          since: slot.since,
          holder,
          riders,
          dot: slot.branch === undefined ? undefined : line.find((one) => one.card.branch === slot.branch),
        };
      });
    return { manifest, name: nameOf(manifest), bays, line, main: view?.hub?.main?.state };
  });
  const over = new Set(ended.map(ownerOf));
  const unbayed = items.filter(
    (one, at) =>
      !over.has(ownerOf(one)) && !ownerOf(one).startsWith("line") && !seen.has(ownerOf(one)) && one.hue !== "ok" && one.hue !== "bad" && items.findIndex((other) => ownerOf(other) === ownerOf(one)) === at,
  );
  return { harbours, unbayed };
}

function holderOf(slot: WorktreeSlot, jobs: readonly JobSummary[], sessions: readonly Session[], items: readonly Item[]): Holder {
  if (slot.closed === true && (slot.held.state === "free" || slot.held.state === "unmade")) return { kind: "closed" };
  switch (slot.held.state) {
    case "free":
      return { kind: "free" };
    case "job": {
      const id = slot.held.job_id;
      const title = jobs.find((one) => one.id === id)?.title ?? slot.held.job_title ?? id;
      return { kind: "job", id, title, item: tileOf(items, id) };
    }
    case "session": {
      const session = sessionIn(slot, sessions);
      return { kind: "session", title: session?.title ?? slot.held.holder, session, item: session === undefined ? undefined : tileOf(items, `session:${session.id}`) };
    }
    case "stranded":
      return { kind: "other", said: `Stranded: ${slot.held.why}` };
    default:
      return { kind: "other", said: OTHER[slot.held.state] ?? slot.held.state };
  }
}
