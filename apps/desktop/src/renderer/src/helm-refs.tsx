// What Helm's thread links: a pull request or an issue by its number, a Job, a Session. **Each opens where
// Bridge already shows it**: a Job on its detail, a Session on its page, a pull request on its row of the merge
// line, and an issue on the composer's open issues. The finding is `@armada/helm`'s `scanRefs`; this holds what
// it reads off the window and what each press does.
//
// A number the repository's open lists do not hold (a merged pull request) is looked up once, and kept: the same
// read `#` in the message box makes.

import { useEffect, useMemo, useState } from "react";

import type { NumberedHit, ProseReference, ProseReferences } from "@armada/components";
import { scanRefs } from "@armada/helm";
import type { RefWorld } from "@armada/helm";
import type { RepositorySummary } from "@armada/protocol";

import type { BridgeState } from "../../shared/bridge";
import { useNumbered } from "./numbered";
import { useSessions } from "./sessions-draft";

type Host = {
  state: BridgeState;
  repositories: readonly RepositorySummary[];
  /** Where each kind is shown. A pull request is its branch's row on the merge line. */
  openSession: (id: string) => void;
  openJob: (jobId: string) => void;
  openPullRequest: (branch: string | undefined) => void;
  openIssues: (root: string | undefined) => void;
};

/** Every `#N` the thread's words hold. */
function numbersIn(state: BridgeState): number[] {
  const helm = state.helm;
  if (helm.state !== "open" && helm.state !== "failed") return [];
  const words = helm.items.flatMap((item) => (item.kind === "asked" || item.kind === "sent" ? [item.text] : item.kind === "row" && item.turn.saw.event === "said" ? [item.turn.saw.text] : []));
  return words.flatMap((text) => [...text.matchAll(/(?<![\w&/#])#(\d+)(?!\w)/g)].map((one) => Number(one[1])));
}

export function useHelmRefs({ state, repositories, openSession, openJob, openPullRequest, openIssues }: Host): ProseReferences {
  const manifestId = state.helm.state === "none" ? undefined : state.helm.manifestId;
  const root = repositories.find((one) => one.manifest?.id === manifestId)?.root;
  const numbered = useNumbered(manifestId);
  const sessions = useSessions();
  const [looked, setLooked] = useState<Record<string, NumberedHit | null>>({});
  const wanted = numbersIn(state).join();

  // A number the open lists do not hold is read once.
  useEffect(() => {
    if (numbered === undefined) return;
    const open = new Set(numbered.open.map((one) => one.tag.id));
    for (const number of new Set(wanted === "" ? [] : wanted.split(",").map(Number))) {
      const key = `${manifestId}#${number}`;
      if (open.has(String(number)) || key in looked) continue;
      setLooked((was) => ({ ...was, [key]: null }));
      void numbered.find(number).then((hit) => setLooked((was) => ({ ...was, [key]: hit ?? null })));
    }
  }, [wanted, numbered, manifestId, looked]);

  return useMemo((): ProseReferences => {
    const known = [...(numbered?.open ?? []), ...Object.entries(looked).flatMap(([key, hit]) => (hit !== null && key.startsWith(`${manifestId}#`) ? [hit] : []))];
    const world: RefWorld = {
      jobs: state.jobs,
      sessions: sessions.filter((one) => one.dead === undefined),
      numbered: known.flatMap((one) => (one.tag.kind === "issue" || one.tag.kind === "pull_request" ? [{ kind: one.tag.kind, number: one.tag.id, title: one.tag.title }] : [])),
    };
    const line = state.mergeLines?.lines.find((one) => one.root === root);
    const branchOf = (number: string): string | undefined => {
      const at = Number(number);
      return (
        line?.hub?.pull_requests?.find((one) => one.number === at)?.branch ??
        line?.hub?.merged?.find((one) => one.number === at)?.branch ??
        [...line?.line ?? [], ...line?.landed ?? [], ...line?.sent_back ?? []].find((one) => one.pull_request?.number === at)?.branch
      );
    };
    return {
      scan: (text) => scanRefs(text, world),
      open: (ref: ProseReference) => {
        if (ref.kind === "session") openSession(ref.id);
        else if (ref.kind === "job") openJob(ref.id);
        else if (ref.kind === "pull_request") openPullRequest(branchOf(ref.id));
        else openIssues(root);
      },
    };
  }, [numbered, looked, manifestId, state.jobs, state.mergeLines, sessions, root, openSession, openJob, openPullRequest, openIssues]);
}
