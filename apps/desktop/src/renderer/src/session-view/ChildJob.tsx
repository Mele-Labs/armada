// The Job a plan group was handed to, which takes the group's row on the spine: the shared Job card titled with the Job's title and the group's title under it,
// with the Job's pull request, when it has one, below. A Job started outside the plan is the same card with no group line, in Now and then Done.

import { JobCard, PullRequestChip } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { jobTitle } from "../issues-tab/marks";
import { usePullRequestMenu } from "../pull-request-menu";
import { checksSaid } from "../session-cards";
import { stateOf } from "../session-state";
import { checksOf, openPrPanel, type PullRequest } from "./IssuesBlock";
import { pullOf } from "./linked";

/** The child's pull request as the shared chip, opened in the side panel as the Issues block opens one. */
function ChildPull({ pr, manifestId }: { pr: PullRequest; manifestId: string | undefined }) {
  const { on, menu } = usePullRequestMenu(manifestId, pr.number, pr.title, pr.address);
  return (
    <>
      <PullRequestChip framed number={pr.number} title={pr.title} state={pr.state} checks={{ state: checksOf(pr), said: checksSaid(pr.checks) }} data-nav="" onOpen={() => openPrPanel(pr)} {...on} />
      {menu}
    </>
  );
}

export function ChildJob({ child, group, manifestId, onOpen }: { child: Session; /** The title of the plan group it was handed from, absent for a Job started outside the plan. */ group?: string | undefined; manifestId: string | undefined; onOpen: (id: string) => void }) {
  const { state, said } = stateOf(child);
  const title = jobTitle(child);
  const pr = pullOf(child);
  return (
    <span className="sv-flight__job">
      <JobCard data-nav="" title={title} state={state} said={said} {...(group === undefined ? {} : { handedFrom: group })} {...(child.lastTurnAt === undefined ? {} : { at: child.lastTurnAt })} onOpen={() => onOpen(child.id)} />
      {pr === undefined ? null : <ChildPull pr={pr} manifestId={manifestId} />}
    </span>
  );
}
