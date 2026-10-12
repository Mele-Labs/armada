// The Store migration spike Session waiting on the person to merge #1850, which its panel's acts
// settle. Over the `sessions` scenario, on its own so the item is not on every walk over it. The
// walk `session-pr-acts` plays it.

import type { Scenario } from "../moment";
import { others } from "../sessions/script";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const spike = () => {
  const one = others().find((it) => it.id === "s3")!;
  return { ...one, waitingFor: [{ id: "merge-1850", text: "Merge #1850 once ci is green", since: minutesAgo(130), source: "agent" as const, act: { kind: "approve_pr" as const, target: "#1850" } }] };
};

export const s224SessionPrActs: Scenario = {
  ...s200Sessions,
  name: "session-pr-acts",
  says: "A Session waiting on the person to merge its pull request",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [spike()]) },
};
