// The run control on a repository's heading in the Cockpit's list: it runs the repository's app on main. The state of an app going there
// stands in a column of its own beside it, empty where nothing runs, so the control keeps its place down the list.

import { RunControl, RunPhaseButton, appPhase } from "@armada/components";

import { openRun } from "../deck/store";
import { pressAddRun, pressRun } from "./acts";
import { appOn, useRuns, useRunsState, type RunTarget } from "./source";

export function RepositoryRun({ root, label }: { root: string; label: string }) {
  const source = useRuns();
  const state = useRunsState();
  if (source === undefined) return null;
  const saved = state.saved[root];
  const app = appOn(state, root);
  const target: RunTarget = { repository: root, checkout: { kind: "main", branch: "main" } };
  return (
    <span className="deck-list__run">
      <span className="deck-list__run-state">{app === undefined ? null : <RunPhaseButton phase={appPhase(app.processes, app.setup)} said={app.run} onPress={() => openRun(app.id)} />}</span>
      <RunControl subject={label} on="main" runs={saved?.runs ?? []} last={saved?.last} onRun={(run) => pressRun(source, target, label, run)} onAdd={() => pressAddRun(source, target, label)} />
    </span>
  );
}
