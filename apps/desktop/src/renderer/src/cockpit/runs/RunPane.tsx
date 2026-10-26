// A running app on the Cockpit's stage, opened the way every pane is: one head with the way back, which Esc is too. Back returns to the
// view it was opened from, a Job's thread where it was scrolled or the map. The pane takes focus, so its keys act at once. Edit (`e`) turns
// its rows into fields in place, and a repository with no saved run opens this pane on its draft, already editing, with nothing running yet.

import { useEffect, useRef } from "react";
import { Play } from "lucide-react";
import { RunActs, RunNameField, RunPhaseMark, RunningPanel, appPhase, editKeys, useRunEdit } from "@armada/components";
import type { RunPanelEdit } from "@armada/components";
import { repositoryLabel } from "@armada/shell";
import { holdsText } from "@armada/screens/src/keys";

import { useDispatching } from "../../dispatch-panel/dispatching";
import type { CockpitProps } from "../deck/props";
import { PaneHead } from "../deck/panes";
import { deckNow, goBack, openRun } from "../deck/store";
import { askedIn, dismissAsked, letGo, pressEditRun, useAsked, type Asked } from "./acts";
import { useRuns, useRunsState, type RunsApp, type RunsSource } from "./source";

export function RunPane({ id, props }: { id: string; props: CockpitProps }) {
  const source = useRuns();
  const state = useRunsState();
  const asked = useAsked();
  const app = state.apps.find((one) => one.id === id);
  useEffect(() => source?.reading(id), [source, id]);
  // Leaving the pane lets its edit go; what was typed stays kept for the next time it is edited.
  useEffect(() => () => (deckNow().sel?.key === `run:${id}` ? undefined : letGo(id)), [id]);
  if (source === undefined) return null;
  if (asked !== undefined && askedIn(asked, id)) return <Editing key={asked.keep} asked={asked} app={app} source={source} props={props} />;
  if (app === undefined) return null;
  return <Shown app={app} source={source} props={props} />;
}

function useWhere(app: RunsApp | undefined, props: CockpitProps): string | undefined {
  const repositories = props.state.holds.repositories ?? [];
  const served = app === undefined ? undefined : repositories.find((one) => one.root === app.repository);
  return served === undefined ? undefined : repositoryLabel(served, repositories);
}

function Shown({ app, source, props }: { app: RunsApp; source: RunsSource; props: CockpitProps }) {
  const dispatching = useDispatching();
  const pane = useRef<HTMLElement>(null);
  useEffect(() => pane.current?.focus(), [app.id]);
  // A first run is named after its repository, so the head says it once.
  const where = useWhere(app, props);
  const phase = appPhase(app.processes, app.setup);
  const live = phase === "setting-up" || phase === "starting" || phase === "ready";
  const address = app.kind === "browser" ? app.processes.find((one) => one.phase === "ready" && one.url !== undefined)?.url : undefined;
  const edit = () => pressEditRun(source, { repository: app.repository, checkout: app.checkout, ...(app.job === undefined ? {} : { job: app.job }) }, where ?? app.repository, app.id);
  return (
    <section
      ref={pane}
      className="deck-pane"
      aria-label={`Run: ${app.run}`}
      tabIndex={-1}
      data-owns-keys="e o r s"
      onKeyDown={(event) => {
        if (event.metaKey || event.ctrlKey || event.altKey || holdsText(event.target)) return;
        if (event.key === "e") edit();
        else if (event.key === "r") source.restart(app.id);
        else if (event.key === "s" && live) source.stop(app.id);
        else if (event.key === "o" && address !== undefined) props.onOpenLink(address);
      }}
    >
      <PaneHead glyph={Play} kind="Run" title={app.run} where={where === app.run ? undefined : where} mark={<RunPhaseMark phase={phase} />}>
        <RunActs onEdit={edit} onRestart={() => source.restart(app.id)} onStop={() => source.stop(app.id)} stoppable={live} />
      </PaneHead>
      <RunningPanel
        key={app.id}
        app={app}
        repository={where ?? app.repository}
        onOpen={props.onOpenLink}
        onFix={(brief) => (dispatching?.choose("session"), dispatching?.openSheet(brief, app.repository))}
        onPin={(link) => source.pin(app.id, link.to, link.port)}
        onUnpin={(process) => source.pin(app.id, process, undefined)}
        onTryAgain={() => source.restart(app.id)}
      />
    </section>
  );
}

/**
 * The run as fields: a draft not saved yet, with nothing running, or a saved run whose app goes on running beside its fields. Save and run
 * saves and starts the draft; Save and restart replaces the saved run and starts it again. Cancel closes a draft's pane, and puts an edited
 * run's rows back. Esc is Cancel here, not Back.
 */
function Editing({ asked, app, source, props }: { asked: Asked; app: RunsApp | undefined; source: RunsSource; props: CockpitProps }) {
  const dispatching = useDispatching();
  const run = useRunEdit({ draft: asked.draft, name: asked.name, keep: asked.keep });
  const phase = app === undefined ? undefined : appPhase(app.processes, app.setup);
  const live = phase === "setting-up" || phase === "starting" || phase === "ready";
  const edit: RunPanelEdit = {
    run,
    save: asked.edit === undefined ? "Save and run" : "Save and restart",
    onSave: () => {
      if (!run.ready) return;
      const { name, processes, setup } = run.saved();
      run.forget();
      dismissAsked();
      void source.save(asked.target, name, processes, setup, asked.edit?.run).then((id) => {
        if (id !== undefined) openRun(id);
      });
    },
    onCancel: () => {
      dismissAsked();
      if (asked.edit !== undefined) run.forget();
      else goBack();
    },
  };
  return (
    <section className="deck-pane" aria-label={`Run: ${app?.run ?? asked.label}`} data-keeps-escape="" onKeyDown={(event) => editKeys(edit, event)}>
      <PaneHead
        glyph={Play}
        kind="Run"
        title={run.name}
        heading={<RunNameField edit={run} />}
        where={asked.name === asked.label ? undefined : asked.label}
        {...(phase === undefined ? {} : { mark: <RunPhaseMark phase={phase} /> })}
      >
        {app === undefined ? null : <RunActs onEdit={() => undefined} onRestart={() => source.restart(app.id)} onStop={() => source.stop(app.id)} stoppable={live} />}
      </PaneHead>
      <RunningPanel
        key={app?.id ?? asked.keep}
        {...(app === undefined ? { checkout: asked.target.checkout } : { app })}
        repository={asked.label}
        edit={edit}
        onOpen={props.onOpenLink}
        onFix={(brief) => (dispatching?.choose("session"), dispatching?.openSheet(brief, asked.target.repository))}
        {...(app === undefined ? {} : { onTryAgain: () => source.restart(app.id) })}
      />
    </section>
  );
}
