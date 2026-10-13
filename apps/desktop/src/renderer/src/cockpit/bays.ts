// What the Bays filter reads: each worktree bay with whoever holds it (a Job, or a Session working in it
// directly) and the Sessions riding with that Job, the pull request its branch has on the merge line,
// matched by branch against the horizon's own dots so the two cannot disagree, and the Jobs queued for a
// bay. Pure, so which bay holds what is checked without drawing a line.

import type { JobSummary, MergeLines, WorktreeSlot } from "@armada/protocol";
import type { MergeLineView } from "@armada/screens";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Item } from "../Dashboard";
import { dotsOf, type Dot } from "./horizon";
import { workOwner, type Owner } from "./owner";

/** Who is in a bay, or why nobody is. */
export type Holder =
  | { kind: "job"; id: string; title: string; item: Item | undefined }
  /** `by` is the process the pool names, worded, where no Session in Armada is the one in the bay. */
  | { kind: "session"; title: string; session: Session | undefined; item: Item | undefined; by: string | undefined }
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

/**
 * What happened to the work before its pull request, each as an ISO instant: it asked for a bay, held one
 * (`slot` where the record names it), and its pull request was opened. A Job's record names no bay and no
 * release, so those are absent there.
 */
export type WorkTrail = { began?: string; bay?: { slot?: number; from: string; until?: string }; opened?: string };

/** Whose work a pull request is, and the tile that owner already has on the glass, for its live state. */
export type PullHolder = Owner & { item: Item | undefined; trail: WorkTrail | undefined };

/** The journey a Session's ledger or a Job's record keeps of the work behind pull request `number`. */
function trailOf(owner: Owner, jobs: readonly JobSummary[], sessions: readonly Session[], number: number): WorkTrail | undefined {
  if (owner.kind === "job") {
    const job = jobs.find((one) => one.id === owner.id);
    return job === undefined ? undefined : { began: job.created_at, ...(job.started_at === undefined ? {} : { bay: { from: job.started_at } }) };
  }
  const trail = sessions.find((one) => one.id === owner.id)?.trail;
  if (trail === undefined) return undefined;
  const bay = trail.bays?.at(-1);
  const opened = trail.opened?.[number];
  return { began: trail.started, ...(bay === undefined ? {} : { bay }), ...(opened === undefined ? {} : { opened }) };
}

/** One repository's bays and its line, top of the line farthest from main. */
export type Harbour = {
  manifest: string;
  /** The repository's label, drawn only where more than one repository has bays. */
  name: string;
  bays: readonly Bay[];
  line: readonly Dot[];
  /** Whose each pull request is, by the dot's key; absent for a person's. */
  holders?: ReadonlyMap<string, PullHolder>;
  main: "green" | "red" | undefined;
};

export type BaysRead = {
  harbours: readonly Harbour[];
  /** Jobs queued for a bay, the longest waiting first: the next one in is the first. */
  waiting: readonly Item[];
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
    const holders = new Map<string, PullHolder>();
    const line = (view === undefined ? [] : [...dotsOf(view, lines, views, live)].reverse()).map((dot): Dot => {
      if (!dot.key.startsWith("pull:")) return dot;
      const owner = workOwner(views, jobs, sessions, Number(dot.key.slice(5)), dot.card.branch);
      if (owner === undefined) return dot;
      const number = Number(dot.key.slice(5));
      holders.set(dot.key, { ...owner, item: tileOf(items, owner.kind === "job" ? owner.id : `session:${owner.id}`), trail: trailOf(owner, jobs, sessions, number) });
      return { ...dot, card: { ...dot.card, owner } };
    });
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
    return { manifest, name: nameOf(manifest), bays, line, holders, main: view?.hub?.main?.state };
  });
  const over = new Set(ended.map(ownerOf));
  const unbayed = items.filter(
    (one, at) =>
      !over.has(ownerOf(one)) && !ownerOf(one).startsWith("line") && !seen.has(ownerOf(one)) && one.hue !== "ok" && one.hue !== "bad" && items.findIndex((other) => ownerOf(other) === ownerOf(one)) === at,
  );
  const queued = (one: Item) => one.hue === "queued" && one.job !== undefined;
  const since = (one: Item) => Date.parse(one.at ?? "") || 0;
  const waiting = unbayed.filter(queued).sort((a, b) => since(a) - since(b));
  return { harbours, waiting };
}

/**
 * A Session not yet named, or a terminal Armada does not know, is called by the branch it is on:
 * a process path or a Session id says nothing of the work.
 */
function sessionHolder(slot: WorktreeSlot, items: readonly Item[], session: Session | undefined, served: string | undefined, last: string | undefined, by: string | undefined): Holder {
  const title = session?.title ?? served ?? slot.branch ?? session?.address ?? last ?? "A Session";
  return { kind: "session", title, session, item: session === undefined ? undefined : tileOf(items, `session:${session.id}`), by };
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
      // The pool names a process: Fleet names the Session it began from where a hosted one started it, and
      // otherwise a Session is found by its own slot or branch.
      const named = slot.held.session_id;
      const session = named !== undefined ? sessions.find((one) => one.id === named) : sessionIn(slot, sessions);
      const pid = /\(pid (\d+)\)/.exec(slot.held.holder)?.[1];
      const by = session !== undefined || named !== undefined ? undefined : pid === undefined ? slot.held.holder : `A terminal, pid ${pid}`;
      return sessionHolder(slot, items, session, slot.held.session_title, by ?? slot.held.holder, by);
    }
    case "hosted_session": {
      const named = slot.held.session_id;
      return sessionHolder(slot, items, sessions.find((one) => one.id === named), slot.held.session_title, undefined, undefined);
    }
    case "stranded":
      return { kind: "other", said: `Stranded: ${slot.held.why}` };
    default:
      return { kind: "other", said: OTHER[slot.held.state] ?? slot.held.state };
  }
}
