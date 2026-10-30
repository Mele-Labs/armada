// The Job a plan group was handed to, on the group's shut row: the shared Job card with its pull request under it, or the Job chip and the pull
// request chip on one line. Two takes, chosen on the mock by `?take=card` (the default) or `?take=chip`, until one is kept and the other deleted.

import { JobCard, JobChip, PullRequestChip } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { peekOfSession } from "../cockpit/deck/ship";
import { jobTitle } from "../issues-tab/marks";
import { usePullRequestMenu } from "../pull-request-menu";
import { checksSaid } from "../session-cards";
import { stateOf } from "../session-state";
import { checksOf, openPrPanel, type PullRequest } from "./IssuesBlock";
import { pullOf } from "./linked";

export const jobTake = (): "card" | "chip" => (typeof window !== "undefined" && new URLSearchParams(window.location.search).get("take") === "chip" ? "chip" : "card");

/** The child's pull request as the shared chip, opened in the side panel as the Issues block opens one. */
function ChildPull({ pr, manifestId, framed }: { pr: PullRequest; manifestId: string | undefined; framed: boolean }) {
  const { on, menu } = usePullRequestMenu(manifestId, pr.number, pr.title, pr.address);
  return (
    <>
      <PullRequestChip {...(framed ? { framed: true } : {})} number={pr.number} title={pr.title} state={pr.state} checks={{ state: checksOf(pr), said: checksSaid(pr.checks) }} data-nav="" onOpen={() => openPrPanel(pr)} {...on} />
      {menu}
    </>
  );
}

export function ChildJob({ child, manifestId, onOpen }: { child: Session; manifestId: string | undefined; onOpen: (id: string) => void }) {
  const { state, said } = stateOf(child);
  const title = jobTitle(child);
  const pr = pullOf(child);
  if (jobTake() === "chip") {
    return (
      <span className="sv-flight__job" data-take="chip">
        <span className="sv-flight__linked">
          <JobChip title={title} state={state} said={`${title}: ${said}`} data-nav="" onOpen={() => onOpen(child.id)} />
        </span>
        {pr === undefined ? null : (
          <span className="sv-flight__linked">
            <ChildPull pr={pr} manifestId={manifestId} framed={false} />
          </span>
        )}
      </span>
    );
  }
  const says = peekOfSession(child).at(-1);
  return (
    <span className="sv-flight__job" data-take="card">
      <JobCard data-nav="" title={title} state={state} said={said} {...(says === undefined ? {} : { says })} {...(child.lastTurnAt === undefined ? {} : { at: child.lastTurnAt })} onOpen={() => onOpen(child.id)} />
      {pr === undefined ? null : <ChildPull pr={pr} manifestId={manifestId} framed />}
    </span>
  );
}
