// This Session's drones over the parts of the codebase they are touching: the Cockpit's battlefield, cut down to the one Session.
// Its own drone and a small one for each subagent hover over the plots of the areas in scope, each doing what Fleet saw it do.
// With no drone yet the slot is empty. Nothing here is drawn from what an agent said: a drone stands where an edit was seen.

import { useMemo } from "react";
import type { RepositorySummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { Battlefield } from "../cockpit/deck/Battlefield";
import { useAreas } from "../cockpit/deck/areas-source";
import type { Areas } from "../cockpit/deck/areas";
import type { DroneIn, RepoIn } from "../cockpit/deck/layout";
import { SvPanel, type PanelState, type Place } from "./SvPanel";

const NONE = new Map();
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
  const drones = work?.field ?? NO_LIST;
  const repository = repositories.find((one) => one.manifest?.id === session.manifestId);
  const root = repository?.root ?? session.manifestId ?? "";
  const name = repository?.manifest?.repository ?? root;
  const scope = work?.plan?.scope;
  const paths = useMemo(() => new Set([...(scope ?? []), ...drones.map((one) => one.path)]), [scope, drones]);
  const repos = useMemo<RepoIn[]>(() => [{ root, name, pads: [], goal: false, areas: narrowed(source(root), paths) }], [root, name, source, paths]);
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
        spin: true,
        hover: true,
        ping: false,
        stuck: false,
        flare: false,
        working: true,
        doing: one.doing,
        via: one.path,
        ...(one.agent === undefined ? {} : { parent: own }),
        touches: [{ path: one.path, w: 4 }],
        shown: true,
      })),
    [drones, own, root, session.title, session.id],
  );
  const state: PanelState | undefined = flying.length === 0 ? undefined : "live";
  return (
    <SvPanel label="Field" area="field" place={place} state={state}>
      {flying.length === 0 ? null : (
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
      )}
    </SvPanel>
  );
}
