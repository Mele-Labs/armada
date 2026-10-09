// Who a pull request belongs to, so a call about it sends the owner to whoever is already on it
// instead of offering a new Drone. A Job opened it where the line's own row says so (`job`, on the
// wire today: `HubPullRequest.job`); a Session holds it where its attachments name the number (the
// ledger's `pr` rows, which a Fleet that serves Sessions reports). Nobody owns a person's.
// Mock only for Sessions: Bridge reads `who_owns` through no accessor, and does not need it where
// the Sessions' own attachments are held. Pure, so the rule is checked without drawing a card.

import type { Session } from "@armada/screens/src/draft/sessions";
import type { MergeLineView } from "@armada/screens";

export type Owner = { kind: "session" | "job"; id: string; title: string };

/** The pull requests and the red the views carry, as the lookup reads them. */
type Seen = Pick<MergeLineView, "root" | "hub">;

/** Whoever owns pull request `number`: one it was claimed for, the Job that opened it, else a Session that holds it. */
export function pullOwner(views: readonly Seen[], sessions: readonly Session[], number: number, branch?: string, claimed?: ReadonlyMap<number, Owner>): Owner | undefined {
  const claim = claimed?.get(number);
  if (claim !== undefined) return claim;
  const pull = views.flatMap((view) => view.hub?.pulls ?? []).find((one) => one.number === number);
  if (pull?.job !== undefined) return { kind: "job", id: pull.job.id, title: pull.job.title };
  const held = sessions.find((one) =>
    one.attachments.some((attachment) => attachment.kind === "pull_request" && (attachment.number === number || (branch !== undefined && attachment.branch === branch))),
  );
  return held === undefined ? undefined : { kind: "session", id: held.id, title: held.title ?? held.id };
}

/** Whoever owns the branch that broke main, found the same way through the pull request that merged it. */
export function mainOwner(views: readonly Seen[], sessions: readonly Session[], root: string, claimed?: ReadonlyMap<number, Owner>): Owner | undefined {
  const main = views.find((view) => view.root === root)?.hub?.main;
  if (main?.state !== "red") return undefined;
  const merge = main.red.merge;
  if (merge === undefined) return undefined;
  const claim = claimed?.get(merge.number);
  if (claim !== undefined) return claim;
  if (merge.job !== undefined) return { kind: "job", id: merge.job.id, title: merge.job.title };
  return pullOwner(views, sessions, merge.number, merge.branch, claimed);
}
