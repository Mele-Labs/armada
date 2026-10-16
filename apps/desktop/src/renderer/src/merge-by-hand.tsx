// A form edit's pull request the forge would not auto-merge, so a person merges it (#2396): the acts Sessions has on a pull
// request, drawn wherever it is told (the Manifest form's line, the deck's pull request pane, its call), and the pull request
// panel Sessions opens, opened from the form's number. No Session owns the pull request, so an act goes through the Sessions
// draft under no Session's id until Fleet has a by-repository press for it.

import { useState, type ReactNode } from "react";
import { PullRequestActs, PullRequestPanel, type PullRequestChecks } from "@armada/components";
import type { PullRequestAct } from "@armada/screens/src/draft/sessions";

import type { BridgeState } from "../../shared/bridge";
import { viewsOf } from "./merge-line";
import { useSessionsDraft } from "./sessions-draft";

/** The id an act is pressed under, where no Session holds the pull request. */
const NO_SESSION = "";

const checksOf = (ci: string | undefined): "pending" | "passed" | "failed" => (ci === "passed" ? "passed" : ci === "failed" ? "failed" : "pending");

/** The act pressed that Fleet has not answered, and the press that sends one. */
function useByHandAct(number: number) {
  const draft = useSessionsDraft();
  const [pending, setPending] = useState<PullRequestAct | undefined>(undefined);
  const press = (act: PullRequestAct) => {
    setPending(act);
    void (draft?.act(NO_SESSION, number, act) ?? Promise.resolve(false)).finally(() => setPending(undefined));
  };
  return { pending, press };
}

/** Sessions' own acts on one pull request: Merge once its checks have passed. */
export function ByHandActs({ number, ci }: { number: number; ci?: string }) {
  const { pending, press } = useByHandAct(number);
  return <PullRequestActs state="open" checks={checksOf(ci)} auto={false} {...(pending === undefined ? {} : { pending })} onAct={press} />;
}

/**
 * The pull request waiting for a person, where the merge line holds one: what the Manifest form draws under Save, and the panel
 * its number opens. The panel goes when the pull request does.
 */
export function useMergeByHand(state: Pick<BridgeState, "mergeLines" | "repository" | "holds" | "landFollowed" | "jobs">, onOpenLink: (address: string) => void) {
  const pull = viewsOf(state).flatMap((view) => view.hub?.pulls ?? []).find((one) => one.mergeByHand === true);
  const row = state.mergeLines?.lines.flatMap((line) => line.hub?.pull_requests ?? []).find((one) => one.number === pull?.number);
  const [open, setOpen] = useState(false);
  const { pending, press } = useByHandAct(pull?.number ?? 0);
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
