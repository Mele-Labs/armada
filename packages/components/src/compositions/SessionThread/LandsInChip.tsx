import { PullRequestChip } from "../PullRequestChip/PullRequestChip";
import type { PlanLandsIn } from "./lands-in";

/** The chip over a run: the pull request once it is opened, a placeholder under its label before. */
export function LandsInChip({ landsIn, onOpen }: { landsIn: PlanLandsIn; onOpen?: ((number: number) => void) | undefined }) {
  const pull = landsIn.pullRequest;
  if (pull === undefined) return <PullRequestChip framed placeholder={landsIn.label} />;
  return <PullRequestChip framed number={pull.number} title={pull.title} state={pull.state} onOpen={() => onOpen?.(pull.number)} />;
}
