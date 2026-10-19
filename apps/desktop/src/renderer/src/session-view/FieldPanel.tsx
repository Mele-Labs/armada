// This Session's drones over the parts of the codebase they are touching: the Cockpit's battlefield, cut down to the one Session.
// The floor is the repository's directories as a treemap, the Cockpit's own, with the files a Session names as small bars on their
// plots. Its own drone and a small one for each subagent hover over it and fly to the bar each is on: a file made, changed or taken
// away is lit in the colour of that act. Each drone is doing what Fleet saw it do.
// With no drone yet the slot is empty. Nothing here is drawn from what an agent said: a drone stands where an edit was seen.
// The plan is carried along the Field's foot as one bar of a segment for each task, and the plots a group names are framed as the plan
// stands there. The control in the band, or `deck_field`'s key (`DeckHost.tsx`), opens the Field large in the layer over the stage (`FieldLayer.tsx`).

import { useMemo } from "react";
import { Maximize2 } from "lucide-react";
import { Button, Tooltip, boundKeyFor, useKeyBindings } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import type { Session, WorkDrone } from "@armada/screens/src/draft/sessions";

import { Battlefield } from "../cockpit/deck/Battlefield";
import { useAreas } from "../cockpit/deck/areas-source";
import type { Areas } from "../cockpit/deck/areas";
import type { DroneIn, RepoIn } from "../cockpit/deck/layout";
import { isWorking, workingDrones } from "./field";
import { PlanBar } from "./FieldPlan";
import { groupsOf, plotsOf } from "./field-plan";
import { toggleLayer, useLayer } from "./layers";
import { SvPanel, type PanelState, type Place } from "./SvPanel";
import { jobIdOf } from "@armada/shell";

const NONE = new Map();
const RESTING: readonly WorkDrone[] = [{ id: "main", path: "", doing: "read" }];
const NO_LIST: never[] = [];
const nothing = () => undefined;

/** Only the plots the Session works in: the directories its scope and its drones name, under their top directories. */
function narrowed(areas: Areas, paths: ReadonlySet<string>): Areas {
  return areas.flatMap((area) => {
    const kids = area.kids.filter(([kid]) => paths.has(`${area.dir}/${kid}`));
    return kids.length === 0 ? [] : [{ dir: area.dir, kids }];
  });
}

/** Everything the Field draws for a Session, read once: its floor, its drones, and the plan as bar and as plots. */
function useField(session: Session, repositories: readonly RepositorySummary[]) {
  const source = useAreas();
  const work = session.work;
  // Before any drone has work the floor has the Session's own drone alone, at rest.
  // Only the subagents at work are drawn: one that has finished has left the Field (the Subagents layer still lists it).
  const working = useMemo(() => workingDrones(session, work?.field), [session, work?.field]);
  const drones = working === undefined || working.length === 0 ? RESTING : working;
  // A Session in a folder with no Manifest is found by where it started.
  const repository = repositories.find((one) => jobIdOf(one) === session.manifestId) ?? (session.manifestId === undefined ? repositories.find((one) => one.root === session.cwd) : undefined);
  const root = repository?.root ?? session.manifestId ?? "";
  const name = repository?.manifest?.repository ?? repository?.root.split("/").pop() ?? root;
  const scope = work?.plan?.scope;
  const paths = useMemo(() => new Set([...(scope ?? []), ...drones.map((one) => one.path)].filter((one) => one !== "")), [scope, drones]);
  const files = work?.files;
  const repos = useMemo<RepoIn[]>(() => [{ root, name, pads: [], goal: false, areas: paths.size === 0 ? source(root) : narrowed(source(root), paths), ...(files === undefined ? {} : { files }) }], [root, name, source, paths, files]);
  const own = `session:${session.id}`;
  const flying = useMemo<DroneIn[]>(
    () =>
      drones.map((one) => ({
        key: one.agent === undefined ? own : `${own}/${one.id}`,
        kind: "session",
        title: one.agent ?? session.title ?? session.id,
        root,
        said: one.task ?? "Working",
        hue: "calm",
        spin: isWorking(session, one),
        hover: true,
        ping: false,
        stuck: false,
        flare: false,
        working: isWorking(session, one),
        ...(one.path === "" ? {} : { doing: one.doing, via: one.path }),
        ...(one.agent === undefined ? {} : { parent: own }),
        ...(one.file === undefined ? {} : { file: one.file }),
        touches: one.path === "" ? [] : [{ path: one.file ?? one.path, w: 4 }],
        shown: true,
      })),
    [drones, own, root, session],
  );
  const plan = work?.plan;
  const groups = useMemo(() => (plan === undefined ? [] : groupsOf(plan)), [plan]);
  const plots = repos[0]!.areas.flatMap((area) => (area.kids.length === 0 ? [area.dir] : area.kids.map(([kid]) => `${area.dir}/${kid}`)));
  const planned = useMemo(() => (plan === undefined ? undefined : plotsOf(plan, plots)), [plan, plots.join("|")]);
  return { repos, flying, groups, planned, resting: drones === RESTING };
}

/** The map and, along its foot, the plan. The same in the panel and in the layer; `large` is the layer's, which names each group under its stretch. */
export function FieldMap({ session, repositories, large = false }: { session: Session; repositories: readonly RepositorySummary[]; large?: boolean }) {
  const { repos, flying, groups, planned } = useField(session, repositories);
  return (
    <div className="sv-field sv-field--plan" data-large={large || undefined}>
      <div className="sv-field__map">
        <Battlefield
          repos={repos}
          drones={flying}
          pads={NONE}
          goals={NONE}
          prs={NO_LIST}
          trails={NONE}
          plays={NO_LIST}
          facts={NONE}
          calls={NONE}
          clashes={NO_LIST}
          selected={undefined}
          own={large ? `${session.id}:large` : session.id}
          {...(planned === undefined ? {} : { planned })}
          onSelect={nothing}
          onPad={nothing}
          onGoal={nothing}
          onPr={nothing}
        />
      </div>
      {groups.length === 0 ? null : <PlanBar groups={groups} titled={large} />}
    </div>
  );
}

export function FieldPanel({ session, repositories, place }: { session: Session; repositories: readonly RepositorySummary[]; place: Place }) {
  const { resting } = useField(session, repositories);
  const expanded = useLayer(session.id) === "field";
  const id = session.id;
  useKeyBindings();
  const state: PanelState | undefined = resting ? undefined : "live";
  const expand = (
    <Tooltip label={expanded ? "Collapse" : "Expand"} shortcut={boundKeyFor("deck_field")}>
      <Button variant="ghost" size="sm" iconOnly aria-label={expanded ? "Collapse" : "Expand"} onClick={() => toggleLayer(id, "field")}>
        <Maximize2 size={16} strokeWidth={2} aria-hidden />
      </Button>
    </Tooltip>
  );
  return (
    <SvPanel label="Field" area="field" place={place} state={state} acts={expand}>
      {/* One canvas at a time: while the layer holds the Field large, the panel draws none. */}
      {expanded ? <div className="sv-field sv-field--plan" /> : <FieldMap session={session} repositories={repositories} />}
    </SvPanel>
  );
}
