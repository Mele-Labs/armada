/** The pull request a plan's group lands in: its label until it is opened, then the pull request itself. */
export type PlanLandsIn = { label: string; pullRequest?: { number: number; title: string; state: "open" | "draft" | "merged" | "closed" } };

/** What a group with no `landsIn` lands in, on a Job that opens pull requests. */
const FIRST = "PR 1";

/**
 * A plan's groups gathered under the pull request each lands in, in the order each pull request is first named. A Job that
 * opens none gets one run with no `landsIn`, and draws no chip. **A group that goes to a Job of its own (`ownJob`) is a run of its own**
 * and its `landsIn` is ignored, since its Job lands its own pull request: the run's head is that Job's card, not a chip.
 */
export function landingsOf<G extends { id: string; landsIn?: PlanLandsIn | undefined; ownJob?: true | undefined }>(groups: readonly G[], opens: boolean): { key: string; landsIn?: PlanLandsIn; /** The group that goes to a Job of its own, whose Job heads this run. */ own?: G; groups: G[] }[] {
  const runs: { key: string; landsIn?: PlanLandsIn; own?: G; groups: G[] }[] = [];
  for (const group of groups) {
    if (group.ownJob === true) {
      runs.push({ key: `job:${group.id}`, own: group, groups: [group] });
      continue;
    }
    const landsIn = opens ? (group.landsIn ?? { label: FIRST }) : undefined;
    const run = runs.find((one) => one.own === undefined && one.key === (landsIn?.label ?? ""));
    if (run === undefined) runs.push({ key: landsIn?.label ?? "", ...(landsIn === undefined ? {} : { landsIn }), groups: [group] });
    else {
      run.groups.push(group);
      if (landsIn !== undefined && run.landsIn?.pullRequest === undefined && landsIn.pullRequest !== undefined) run.landsIn = landsIn;
    }
  }
  if (runs.length === 0 && !opens) runs.push({ key: "", groups: [] });
  return runs;
}
