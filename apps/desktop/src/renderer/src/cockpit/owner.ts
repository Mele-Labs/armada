// Who a pull request belongs to, so a call about it sends the owner to whoever is already on it
// instead of offering a new Drone. A Session holds it where its attachments name the number (the
// ledger's `pr` rows, which a Fleet that serves Sessions reports). Nobody owns a person's.
// Mock only for Sessions: Bridge reads `who_owns` through no accessor, and does not need it where
// the Sessions' own attachments are held. Pure, so the rule is checked without drawing a card.

import type { JobSummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";
import type { MergeLineView } from "@armada/screens";

export type Owner = { kind: "session" | "job"; id: string; title: string };

/** The pull requests and the red the views carry, as the lookup reads them. */
type Seen = Pick<MergeLineView, "root" | "hub">;

/** Whoever owns pull request `number`: a Session that holds it (a claim shows there too). */
export function pullOwner(sessions: readonly Session[], number: number, branch?: string): Owner | undefined {
  const held = sessions.find((one) =>
    one.attachments.some((attachment) => attachment.kind === "pull_request" && (attachment.number === number || (branch !== undefined && attachment.branch === branch))),
  );
  return held === undefined ? undefined : { kind: "session", id: held.id, title: held.title ?? held.id };
}

/**
 * Whose work a pull request is, where it sits in no bay: a Job on its branch, a Session
 * that holds the number or the branch. A Session that has ended still made it, so it is found after a live one.
 */
export function workOwner(jobs: readonly Pick<JobSummary, "id" | "title" | "branch">[], sessions: readonly Session[], number: number, branch: string): Owner | undefined {
  const job = jobs.find((one) => one.branch === branch);
  if (job !== undefined) return { kind: "job", id: job.id, title: job.title };
  const holds = (one: Session) =>
    one.attachments.some((attachment) => (attachment.kind === "pull_request" && (attachment.number === number || attachment.branch === branch)) || (attachment.kind === "branch" && attachment.name === branch));
  const held = sessions.find((one) => one.dead === undefined && holds(one)) ?? sessions.find(holds);
  return held === undefined ? undefined : { kind: "session", id: held.id, title: held.title ?? held.id };
}

/**
 * Whoever a pull request on the timeline belongs to, open or merged: the Job the line names for it, else
 * whoever `workOwner` finds by the number or the branch. The branch is the one the entry carried, else the
 * one the line lists the number under. Undefined where nobody holds it.
 */
export function timelineOwner(
  views: readonly (Seen & Pick<MergeLineView, "line" | "landed">)[],
  jobs: readonly Pick<JobSummary, "id" | "title" | "branch">[],
  sessions: readonly Session[],
  number: number,
  branch: string | undefined,
): Owner | undefined {
  const listed = views.flatMap((view) => [...view.line, ...view.landed]).find((entry) => entry.pr?.number === number);
  if (listed?.job !== undefined) return { kind: "job", id: listed.job.id, title: listed.job.title };
  const known = branch ?? views.flatMap((view) => view.hub?.pulls ?? []).find((one) => one.number === number)?.branch ?? listed?.branch;
  return known === undefined ? pullOwner(sessions, number) : workOwner(jobs, sessions, number, known);
}

/** Whoever owns the branch that broke main, found the same way through the pull request that merged it. */
export function mainOwner(views: readonly Seen[], sessions: readonly Session[], root: string): Owner | undefined {
  const main = views.find((view) => view.root === root)?.hub?.main;
  if (main?.state !== "red") return undefined;
  const merge = main.red.merge;
  if (merge === undefined) return undefined;
  return pullOwner(sessions, merge.number, merge.branch);
}

/** Whoever owns what a `pull:<number>` or `main:<root>` call is about; nobody for any other call. */
export function ownerOfCall(views: readonly Seen[], sessions: readonly Session[], key: string): Owner | undefined {
  if (key.startsWith("pull:")) return pullOwner(sessions, Number(key.slice("pull:".length)));
  if (key.startsWith("main:")) return mainOwner(views, sessions, key.slice("main:".length));
  return undefined;
}

/**
 * When the failure a call is about arose, in epoch milliseconds: the pull request's `updated_at` (the
 * forge's "anything last happened", the nearest the wire has to when its checks failed) or the instant main
 * first read red (`red_since`). Undefined where the wire carried neither.
 */
export function failedAt(views: readonly Seen[], key: string): number | undefined {
  const at = key.startsWith("pull:")
    ? views.flatMap((view) => view.hub?.pulls ?? []).find((one) => one.number === Number(key.slice("pull:".length)))?.updatedAt
    : key.startsWith("main:")
      ? (() => {
          const main = views.find((view) => view.root === key.slice("main:".length))?.hub?.main;
          return main?.state === "red" ? main.red.since : undefined;
        })()
      : undefined;
  const parsed = at === undefined ? NaN : Date.parse(at);
  return Number.isNaN(parsed) ? undefined : parsed;
}

/**
 * Whether the owner is already on the failure, so the call is not raised. A Job is on it while it runs, or once
 * it was told (a redirect to its Drone, or a step restarted: `last_told_at`) after the failure arose. A
 * Session is on it while its turn works, or once anything was said to it (the owner's words, a Poke, another
 * Session's) after the failure arose; one that went idle with nothing sent since is not, and the call comes
 * back. With no time for the failure, only work in progress counts. `runningJobs` is the Jobs at work; `told`
 * is each Job's `last_told_at`, by id.
 */
export function isOnIt(owner: Owner, sessions: readonly Session[], runningJobs: ReadonlySet<string>, told: ReadonlyMap<string, string>, since: number | undefined): boolean {
  if (owner.kind === "job") {
    if (runningJobs.has(owner.id)) return true;
    const at = Date.parse(told.get(owner.id) ?? "");
    return since !== undefined && !Number.isNaN(at) && at > since;
  }
  const session = sessions.find((one) => one.id === owner.id);
  if (session === undefined || session.dead !== undefined) return false;
  if (session.turn.state === "working") return true;
  if (since === undefined) return false;
  return session.rows.some((row) => row.kind === "message" && row.from.kind !== "agent" && (Date.parse(row.instant ?? row.at) || 0) > since);
}
