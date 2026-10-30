/** The pull request a plan's group lands in: its label until it is opened, then the pull request itself. */
export type PlanLandsIn = { label: string; pullRequest?: { number: number; title: string; state: "open" | "draft" | "merged" | "closed" } };

/** What a group with no `landsIn` lands in, on a Job that opens pull requests. */
const FIRST = "PR 1";

/**
 * A plan's groups gathered under the pull request each lands in, in the order each pull request is first named. A Job that
 * opens none gets one run with no `landsIn`, and draws no chip.
 */
export function landingsOf<G extends { id: string; landsIn?: PlanLandsIn | undefined }>(groups: readonly G[], opens: boolean): { key: string; landsIn?: PlanLandsIn; groups: G[] }[] {
  if (!opens) return [{ key: "", groups: [...groups] }];
  const runs: { key: string; landsIn: PlanLandsIn; groups: G[] }[] = [];
  for (const group of groups) {
    const landsIn = group.landsIn ?? { label: FIRST };
    const run = runs.find((one) => one.key === landsIn.label);
    if (run === undefined) runs.push({ key: landsIn.label, landsIn, groups: [group] });
    else {
      run.groups.push(group);
      if (run.landsIn.pullRequest === undefined && landsIn.pullRequest !== undefined) run.landsIn = landsIn;
    }
  }
  return runs;
}
