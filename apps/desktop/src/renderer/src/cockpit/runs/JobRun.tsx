// The run control in a Job's header: it runs the Job's own bay, not main. Drawn only where the Job holds a bay.

import { useEffect } from "react";
import type { RepositorySummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";
import { RunControl, RunPhaseButton, appPhase } from "@armada/components";
import { repositoryLabel } from "@armada/shell";

import { openRun } from "../deck/store";
import { pressAddRun, pressRun } from "./acts";
import { appOn, useRuns, useRunsState, type RunTarget } from "./source";
import { jobTarget } from "./target";

export function JobRun({ session, repositories }: { session: Session; repositories: readonly RepositorySummary[] }) {
  const source = useRuns();
  const state = useRunsState();
  const target = jobTarget(session, repositories);
  // Keyed by what the target says, since it is a new object on every draw.
  const seen = target === undefined ? undefined : JSON.stringify(target);
  useEffect(() => (source === undefined || seen === undefined ? undefined : source.seen(JSON.parse(seen) as RunTarget)), [source, seen]);
  if (source === undefined || target === undefined) return null;
  const saved = state.saved[target.repository];
  const app = appOn(state, target.repository, session.id);
  const served = repositories.find((one) => one.root === target.repository);
  const label = served === undefined ? target.repository : repositoryLabel(served, repositories);
  return (
    <>
      {app === undefined ? null : <RunPhaseButton phase={appPhase(app.processes)} said={app.run} onPress={() => openRun(app.id)} />}
      <RunControl subject="this Job" on="this bay" runs={saved?.runs ?? []} last={saved?.last} onRun={(run) => pressRun(source, target, label, run)} onAdd={() => pressAddRun(target, label)} />
    </>
  );
}
