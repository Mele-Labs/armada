// A running app on the Cockpit's stage, opened the way every pane is: one head with the way back, which Esc is too. Back returns to the
// view it was opened from, a Job's thread where it was scrolled or the map. The pane takes focus, so its three keys act at once.

import { useEffect, useRef } from "react";
import { Play } from "lucide-react";
import { RunActs, RunPhaseMark, RunningPanel, appPhase } from "@armada/components";
import { repositoryLabel } from "@armada/shell";
import { holdsText } from "@armada/screens/src/keys";

import { useDispatching } from "../../dispatch-panel/dispatching";
import type { CockpitProps } from "../deck/props";
import { PaneHead } from "../deck/panes";
import { useRuns, useRunsState } from "./source";

export function RunPane({ id, props }: { id: string; props: CockpitProps }) {
  const source = useRuns();
  const dispatching = useDispatching();
  const state = useRunsState();
  const pane = useRef<HTMLElement>(null);
  const app = state.apps.find((one) => one.id === id);
  useEffect(() => pane.current?.focus(), [id]);
  if (source === undefined || app === undefined) return null;
  const repositories = props.state.holds.repositories ?? [];
  const served = repositories.find((one) => one.root === app.repository);
  // A first run is named after its repository, so the head says it once.
  const where = served === undefined ? undefined : repositoryLabel(served, repositories);
  const phase = appPhase(app.processes);
  const live = phase === "starting" || phase === "ready";
  const address = app.kind === "browser" ? app.processes.find((one) => one.phase === "ready" && one.url !== undefined)?.url : undefined;
  return (
    <section
      ref={pane}
      className="deck-pane"
      aria-label={`Run: ${app.run}`}
      tabIndex={-1}
      data-owns-keys="o r s"
      onKeyDown={(event) => {
        if (event.metaKey || event.ctrlKey || event.altKey || holdsText(event.target)) return;
        if (event.key === "r") source.restart(app.id);
        else if (event.key === "s" && live) source.stop(app.id);
        else if (event.key === "o" && address !== undefined) props.onOpenLink(address);
      }}
    >
      <PaneHead glyph={Play} kind="Run" title={app.run} where={where === app.run ? undefined : where} mark={<RunPhaseMark phase={phase} />}>
        <RunActs onRestart={() => source.restart(app.id)} onStop={() => source.stop(app.id)} stoppable={live} />
      </PaneHead>
      <RunningPanel
        app={app}
        onOpen={props.onOpenLink}
        onFix={(brief) => (dispatching?.choose("session"), dispatching?.openSheet(brief, app.repository))}
        onPin={(link) => source.pin(app.id, link.to, link.port)}
        onUnpin={(process) => source.pin(app.id, process, undefined)}
      />
    </section>
  );
}
