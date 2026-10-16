// This Session's drones over the parts of the codebase they are touching: the Cockpit's battlefield, cut down to the one Session.
// The floor is the repository's directories as a treemap, the Cockpit's own, with the files a Session names as small bars on their
// plots. Its own drone and a small one for each subagent hover over it and fly to the bar each is on: a file made, changed or taken
// away is lit in the colour of that act. Each drone is doing what Fleet saw it do.
// With no drone yet the slot is empty. Nothing here is drawn from what an agent said: a drone stands where an edit was seen.

import { useMemo } from "react";
import type { RepositorySummary } from "@armada/protocol";
import type { Session, WorkDrone } from "@armada/screens/src/draft/sessions";

import { Battlefield } from "../cockpit/deck/Battlefield";
import { useAreas } from "../cockpit/deck/areas-source";
import type { Areas } from "../cockpit/deck/areas";
import type { DroneIn, RepoIn } from "../cockpit/deck/layout";
import { SvPanel, type PanelState, type Place } from "./SvPanel";

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

export function FieldPanel({ session, repositories, place }: { session: Session; repositories: readonly RepositorySummary[]; place: Place }) {
  const source = useAreas();
  const work = session.work;
  // Before any drone has work the floor has the Session's own drone alone, at rest.
  const drones = work?.field === undefined || work.field.length === 0 ? RESTING : work.field;
  const repository = repositories.find((one) => one.manifest?.id === session.manifestId);
  const root = repository?.root ?? session.manifestId ?? "";
  const name = repository?.manifest?.repository ?? root;
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
        spin: one.path !== "",
        hover: true,
        ping: false,
        stuck: false,
        flare: false,
        working: one.path !== "",
        ...(one.path === "" ? {} : { doing: one.doing, via: one.path }),
        ...(one.agent === undefined ? {} : { parent: own }),
        ...(one.file === undefined ? {} : { file: one.file }),
        touches: one.path === "" ? [] : [{ path: one.file ?? one.path, w: 4 }],
        shown: true,
      })),
    [drones, own, root, session.title, session.id],
  );
  const state: PanelState | undefined = drones === RESTING ? undefined : "live";
  return (
    <SvPanel label="Field" area="field" place={place} state={state}>
      <div className="sv-field">
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
            onSelect={nothing}
            onPad={nothing}
            onGoal={nothing}
            onPr={nothing}
          />
        </div>
    </SvPanel>
  );
}
