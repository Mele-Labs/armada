// A form edit's pull request the forge would not auto-merge, so a person merges it (#2396): the acts Sessions has on a pull
// request, drawn wherever it is told (the Manifest form's line, the deck's pull request pane, its call), and the pull request
// panel Sessions opens, opened from the form's number. No Session owns the pull request, so an act is pressed on the repository,
// which Fleet names by its Manifest (`actInRepository`).

import { useState, type ReactNode } from "react";
import { PullRequestActs, PullRequestPanel, type PullRequestChecks } from "@armada/components";
import type { PullRequestAct } from "@armada/screens/src/draft/sessions";

import type { BridgeState } from "../../shared/bridge";
import { viewsOf } from "./merge-line";
import { useRepositoryActs } from "./sessions-draft";

const checksOf = (ci: string | undefined): "pending" | "passed" | "failed" => (ci === "passed" ? "passed" : ci === "failed" ? "failed" : "pending");

/** The act pressed that Fleet has not answered, and the press that sends one. */
function useByHandAct(root: string, number: number) {
  const act = useRepositoryActs().actInRepository;
  const [pending, setPending] = useState<PullRequestAct | undefined>(undefined);
  const press = (what: PullRequestAct) => {
    setPending(what);
    void (act?.(root, number, what) ?? Promise.resolve(false)).finally(() => setPending(undefined));
  };
  return { pending, press };
}

/** Sessions' own acts on one pull request: Merge once its checks have passed. */
export function ByHandActs({ root, number, ci }: { root: string; number: number; ci?: string }) {
  const { pending, press } = useByHandAct(root, number);
  return <PullRequestActs state="open" checks={checksOf(ci)} auto={false} {...(pending === undefined ? {} : { pending })} onAct={press} />;
}

/**
 * The pull request waiting for a person, where the merge line holds one: what the Manifest form draws under Save, and the panel
 * its number opens. The panel goes when the pull request does.
 */
export function useMergeByHand(state: Pick<BridgeState, "mergeLines" | "repository" | "holds" | "landFollowed" | "jobs">, onOpenLink: (address: string) => void) {
  const view = viewsOf(state).find((one) => one.hub?.pulls.some((pull) => pull.mergeByHand === true));
  const pull = view?.hub?.pulls.find((one) => one.mergeByHand === true);
  const row = state.mergeLines?.lines.find((line) => line.root === view?.root)?.hub?.pull_requests?.find((one) => one.number === pull?.number);
  const [open, setOpen] = useState(false);
  const { pending, press } = useByHandAct(view?.root ?? "", pull?.number ?? 0);
  if (pull === undefined) return { mergeByHand: undefined, panel: null as ReactNode };
  const checks = checksOf(pull.ci);
  const panelChecks: PullRequestChecks = checks === "failed" ? { state: "failed", failing: [] } : { state: checks };
  return {
    mergeByHand: { number: pull.number, checks, ...(pending === undefined ? {} : { pending }), onOpen: () => setOpen(true), onAct: press },
    panel: open ? (
      <PullRequestPanel
        number={pull.number}
        title={row?.title ?? `#${pull.number}`}
        branch={pull.branch}
        address={pull.url}
        state="open"
        checks={panelChecks}
        auto={false}
        {...(pending === undefined ? {} : { pending })}
        onAct={press}
        onOpenLink={onOpenLink}
        onClose={() => setOpen(false)}
      />
    ) : null,
  };
}
